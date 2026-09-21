use super::*;

impl MaintenanceCliCoordinator {
    pub(crate) fn new(
        app: &App,
        build_dir: &Path,
        search_path: Vec<PathBuf>,
    ) -> Result<Self, String> {
        Ok(Self {
            context: MaintenanceCliContext::from_app(app, build_dir, search_path)?,
            inspection: None,
            cleanup_preview: None,
            operation: None,
            snapshot: None,
            local_archive: None,
            archive_intent: None,
            deferred_archive_push: None,
            next_preview_id: 1,
        })
    }

    pub(crate) fn operation_active(&self) -> bool {
        self.inspection.is_some() || self.cleanup_preview.is_some() || self.operation.is_some()
    }

    pub(crate) async fn shutdown(&mut self) {
        if let Some(worker) = self.inspection.take() {
            worker.handle.abort();
            let _ = worker.handle.await;
        }
        if let Some(mut active) = self.operation.take() {
            if let Some(wait) = active.cancellation.take() {
                wait.abort();
                let _ = wait.await;
            } else if let Some(mut runner) = active.runner.take() {
                let _ = runner.cancel(active.id).await;
            }
        }
        if let Some(mut preview) = self.cleanup_preview.take() {
            let _ = preview.runner.cancel(preview.id).await;
        }
    }

    pub(crate) async fn handle_effect(&mut self, app: &mut App, effect: Effect) -> bool {
        let Effect::Maintenance(effect) = effect else {
            return false;
        };
        match effect {
            MaintenanceEffect::InspectCapability { request } => {
                self.start_inspection(InspectionPurpose::Refresh(request));
            }
            MaintenanceEffect::InspectServices { request } => {
                self.start_inspection(InspectionPurpose::Services(request));
            }
            MaintenanceEffect::PreviewReadiness {
                capability_request,
                request,
            } => self.start_inspection(InspectionPurpose::PreviewReadiness {
                capability_request,
                request: Box::new(request),
            }),
            MaintenanceEffect::PreviewCleanup {
                capability_request,
                request,
            } => {
                if self.cleanup_preview.is_some() || self.operation.is_some() {
                    app.notification =
                        Some("another Maintenance operation is already active".into());
                } else {
                    self.start_inspection(InspectionPurpose::PreviewCleanup {
                        capability_request,
                        request: Box::new(request),
                    });
                }
            }
            MaintenanceEffect::PreviewPrService {
                capability_request,
                request,
            } => self.start_inspection(InspectionPurpose::PreviewPrService {
                capability_request,
                request: Box::new(request),
            }),
            MaintenanceEffect::StartOperation { id, preview } => {
                if self.operation.is_some() {
                    fail(app, id, "another Maintenance operation is already active");
                } else {
                    self.start_inspection(InspectionPurpose::Start { id, preview });
                }
            }
            MaintenanceEffect::CancelOperation(id) => self.cancel(app, id),
            MaintenanceEffect::PreviewLockedSignatureCache {
                capability_request,
                request,
            } => self.start_inspection(InspectionPurpose::PreviewLockedSignatureCache {
                capability_request,
                request: Box::new(request),
            }),
            MaintenanceEffect::PreviewBuildHistoryComparison {
                capability_request,
                request,
            } => self.start_inspection(InspectionPurpose::PreviewBuildHistoryComparison {
                capability_request,
                request: Box::new(request),
            }),
            MaintenanceEffect::PreviewGitArchive {
                capability_request,
                request,
            } => self.start_inspection(InspectionPurpose::PreviewGitArchive {
                capability_request,
                request: Box::new(request),
            }),
            MaintenanceEffect::OpenEvidence(_) | MaintenanceEffect::Navigate(_) => return false,
        }
        true
    }

    pub(super) fn start_inspection(&mut self, purpose: InspectionPurpose) {
        if let Some(worker) = self.inspection.take() {
            worker.handle.abort();
        }
        let context = self.context.clone();
        self.inspection = Some(InspectionWorker {
            purpose,
            deadline: tokio::time::Instant::now() + INSPECTION_TIMEOUT,
            handle: tokio::task::spawn_blocking(move || inspect(context)),
        });
    }

    pub(super) fn cancel(&mut self, app: &mut App, id: MaintenanceSessionId) {
        let Some(active) = self.operation.as_mut() else {
            reject_cancellation(app, id, "no Maintenance operation is active");
            return;
        };
        if active.id != id || active.cancellation.is_some() {
            reject_cancellation(app, id, "the exact Maintenance session is not cancellable");
            return;
        }
        let Some(mut runner) = active.runner.take() else {
            reject_cancellation(app, id, "Maintenance cancellation is already in progress");
            return;
        };
        active.cancellation = Some(tokio::spawn(async move {
            let result = runner.cancel(id).await.map_err(|error| error.to_string());
            (runner, result)
        }));
    }

    pub(crate) async fn poll(&mut self, app: &mut App) {
        self.poll_inspection(app).await;
        self.poll_cleanup_preview(app).await;
        self.poll_operation(app).await;
    }

    pub(super) async fn poll_inspection(&mut self, app: &mut App) {
        let Some(worker) = self.inspection.as_ref() else {
            return;
        };
        if !worker.handle.is_finished() && tokio::time::Instant::now() < worker.deadline {
            return;
        }
        let worker = self
            .inspection
            .take()
            .expect("inspection worker was checked");
        if !worker.handle.is_finished() {
            worker.handle.abort();
            self.inspection_failed(
                app,
                worker.purpose,
                "Maintenance inspection timed out".into(),
            );
            return;
        }
        match worker.handle.await {
            Ok(result) => self.finish_inspection(app, worker.purpose, result).await,
            Err(error) => self.inspection_failed(
                app,
                worker.purpose,
                format!("Maintenance inspection worker was lost: {error}"),
            ),
        }
    }

    pub(super) fn inspection_failed(
        &mut self,
        app: &mut App,
        purpose: InspectionPurpose,
        message: String,
    ) {
        match purpose {
            InspectionPurpose::Refresh(request) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::CapabilityFailed {
                        request,
                        message: message.clone(),
                    }),
                );
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::ServicesFailed {
                        request,
                        message: message.clone(),
                    }),
                );
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::IntegrationsFailed { request, message }),
                );
            }
            InspectionPurpose::Services(request) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::ServicesFailed { request, message }),
                );
            }
            InspectionPurpose::Start { id, .. } => fail(app, id, &message),
            InspectionPurpose::PreviewReadiness { .. }
            | InspectionPurpose::PreviewCleanup { .. }
            | InspectionPurpose::PreviewPrService { .. }
            | InspectionPurpose::PreviewLockedSignatureCache { .. }
            | InspectionPurpose::PreviewBuildHistoryComparison { .. }
            | InspectionPurpose::PreviewGitArchive { .. }
            | InspectionPurpose::PreviewGitArchivePush { .. } => {
                app.notification = Some(message);
            }
        }
    }

    pub(super) async fn finish_inspection(
        &mut self,
        app: &mut App,
        purpose: InspectionPurpose,
        result: MaintenanceInspection,
    ) {
        match purpose {
            InspectionPurpose::Refresh(request) => {
                match result.capability {
                    Ok(snapshot) => {
                        let partial = !snapshot.limitations.is_empty()
                            || snapshot.tools.iter().any(|tool| {
                                matches!(tool, MaintenanceToolCapability::Unavailable { .. })
                            });
                        self.snapshot = Some(snapshot.clone());
                        let _ = update(
                            app,
                            Action::Maintenance(MaintenanceAction::CapabilityLoaded {
                                request,
                                snapshot,
                                partial,
                            }),
                        );
                    }
                    Err(message) => {
                        let _ = update(
                            app,
                            Action::Maintenance(MaintenanceAction::CapabilityFailed {
                                request,
                                message,
                            }),
                        );
                    }
                }
                apply_services(app, request, result.services);
                apply_integrations(app, request, result.integrations);
            }
            InspectionPurpose::Services(request) => apply_services(app, request, result.services),
            InspectionPurpose::Start { id, preview } => match result.capability {
                Ok(snapshot) => self.begin_operation(app, id, *preview, snapshot).await,
                Err(message) => fail(app, id, &message),
            },
            InspectionPurpose::PreviewReadiness {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => self.preview_readiness(app, capability_request, *request, snapshot),
                Err(message) => app.notification = Some(message),
            },
            InspectionPurpose::PreviewCleanup {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => {
                    self.begin_cleanup_preview(app, capability_request, *request, snapshot)
                        .await;
                }
                Err(message) => app.notification = Some(message),
            },
            InspectionPurpose::PreviewPrService {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => {
                    self.preview_pr_service(app, capability_request, *request, snapshot)
                }
                Err(message) => app.notification = Some(message),
            },
            InspectionPurpose::PreviewLockedSignatureCache {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => {
                    self.preview_locked_signature_cache(app, capability_request, *request, snapshot)
                }
                Err(message) => app.notification = Some(message),
            },
            InspectionPurpose::PreviewBuildHistoryComparison {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => {
                    self.preview_buildhistory(app, capability_request, *request, snapshot)
                }
                Err(message) => app.notification = Some(message),
            },
            InspectionPurpose::PreviewGitArchive {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => {
                    self.preview_git_archive(app, capability_request, *request, snapshot)
                }
                Err(message) => app.notification = Some(message),
            },
            InspectionPurpose::PreviewGitArchivePush {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => {
                    self.preview_git_archive_push(app, capability_request, *request, snapshot)
                }
                Err(message) => app.notification = Some(message),
            },
        }
    }
}
