use super::*;

impl MaintenanceCliCoordinator {
    pub(super) async fn begin_operation(
        &mut self,
        app: &mut App,
        id: MaintenanceSessionId,
        confirmed: MaintenanceOperationPreview,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if confirmed.capability_request == 0
            || self
                .snapshot
                .as_ref()
                .is_none_or(|cached| cached != &snapshot)
        {
            fail(
                app,
                id,
                "Maintenance capability changed; refresh and confirm again",
            );
            return;
        }
        let operation = confirmed.operation.clone();
        let built = match operation {
            MaintenanceOperation::SstateReadiness(request) => {
                MaintenanceSstateCommandSpec::readiness(
                    id,
                    confirmed.capability_request,
                    &snapshot,
                    confirmed.id,
                    request,
                )
                .map(|(preview, command)| {
                    (
                        preview,
                        command,
                        OperationStage::Execute(Box::new(EvidencePlan::None)),
                    )
                })
                .map_err(|error| error.to_string())
            }
            MaintenanceOperation::SstateCleanup(preview) => {
                MaintenanceSstateCommandSpec::cleanup_preview(
                    id,
                    &snapshot,
                    preview.request.clone(),
                )
                .map(|command| {
                    (
                        confirmed.clone(),
                        command,
                        OperationStage::CleanupPreview(Box::new(CleanupPreviewStage {
                            capability_request: confirmed.capability_request,
                            operation_id: confirmed.id,
                            snapshot,
                            confirmed: preview,
                            stdout: Vec::new(),
                        })),
                    )
                })
                .map_err(|error| error.to_string())
            }
            MaintenanceOperation::PrService(request) => {
                let evidence = match request.operation {
                    PrServiceOperation::Export => EvidencePlan::PrExport(request.file.clone()),
                    PrServiceOperation::Import => EvidencePlan::None,
                };
                pr_service_command(
                    id,
                    confirmed.capability_request,
                    &snapshot,
                    confirmed.id,
                    request,
                )
                .map(|(preview, command)| {
                    (
                        preview,
                        command,
                        OperationStage::Execute(Box::new(evidence)),
                    )
                })
                .map_err(|error| error.to_string())
            }
            MaintenanceOperation::LockedSignatureCache(request) => locked_signature_command(
                id,
                confirmed.capability_request,
                &snapshot,
                confirmed.id,
                request,
            )
            .map(|(preview, command, before)| {
                (
                    preview,
                    command,
                    OperationStage::Execute(Box::new(EvidencePlan::Release(before))),
                )
            })
            .map_err(|error| error.to_string()),
            MaintenanceOperation::BuildHistoryComparison(request) => buildhistory_command(
                id,
                confirmed.capability_request,
                &snapshot,
                confirmed.id,
                request,
            )
            .map(|(preview, command)| {
                (
                    preview,
                    command,
                    OperationStage::Execute(Box::new(EvidencePlan::None)),
                )
            })
            .map_err(|error| error.to_string()),
            MaintenanceOperation::BuildCompare(request) => {
                build_compare_command(id, &snapshot, request)
                    .map(|command| {
                        (
                            confirmed.clone(),
                            command,
                            OperationStage::Execute(Box::new(EvidencePlan::None)),
                        )
                    })
                    .map_err(|error| error.to_string())
            }
            MaintenanceOperation::GitArchive(request) if request.push_remote.is_some() => self
                .local_archive
                .as_ref()
                .ok_or_else(|| {
                    "local Git archive evidence is unavailable; run local archive first".to_owned()
                })
                .and_then(|local| {
                    git_archive_push_command(
                        id,
                        confirmed.capability_request,
                        &snapshot,
                        confirmed.id,
                        request,
                        local,
                    )
                    .map(|(preview, command)| {
                        (
                            preview,
                            command,
                            OperationStage::Execute(Box::new(EvidencePlan::None)),
                        )
                    })
                    .map_err(|error| error.to_string())
                }),
            MaintenanceOperation::GitArchive(request) => {
                let evidence_request = self
                    .archive_intent
                    .take()
                    .filter(|(preview_id, original)| {
                        let mut local = original.clone();
                        local.push_remote = None;
                        *preview_id == confirmed.id && local == request
                    })
                    .map(|(_, original)| original)
                    .unwrap_or_else(|| request.clone());
                git_archive_local_command(
                    id,
                    confirmed.capability_request,
                    &snapshot,
                    confirmed.id,
                    &evidence_request,
                )
                .map(|(preview, command)| {
                    (
                        preview,
                        command,
                        OperationStage::Execute(Box::new(EvidencePlan::GitArchive(
                            evidence_request,
                        ))),
                    )
                })
                .map_err(|error| error.to_string())
            }
        };
        let (fresh_preview, command, stage) = match built {
            Ok(value) => value,
            Err(message) => {
                fail(app, id, &message);
                return;
            }
        };
        if fresh_preview != confirmed {
            fail(
                app,
                id,
                "Maintenance preview changed; inspect and confirm the exact operation again",
            );
            return;
        }
        let mut runner = MaintenanceSstateJobRunner::new();
        if let Err(error) = runner.start(command).await {
            fail(app, id, &error.to_string());
            return;
        }
        self.operation = Some(MaintenanceCliOperation {
            id,
            runner: Some(runner),
            cancellation: None,
            stage,
        });
    }

    pub(super) async fn poll_operation(&mut self, app: &mut App) {
        let Some(active) = self.operation.as_mut() else {
            return;
        };
        if active
            .cancellation
            .as_ref()
            .is_some_and(JoinHandle::is_finished)
        {
            let wait = active
                .cancellation
                .take()
                .expect("cancellation was checked");
            match wait.await {
                Ok((runner, Ok(_))) => active.runner = Some(runner),
                Ok((runner, Err(message))) => {
                    active.runner = Some(runner);
                    reject_cancellation(app, active.id, &message);
                }
                Err(error) => {
                    let id = active.id;
                    lose(
                        app,
                        id,
                        &format!("Maintenance cancellation worker was lost: {error}"),
                    );
                    self.operation = None;
                    return;
                }
            }
        }
        let Some(runner) = active.runner.as_mut() else {
            return;
        };
        let event = match tokio::time::timeout(Duration::from_millis(1), runner.next_event()).await
        {
            Ok(Ok(event)) => event,
            Ok(Err(error)) => {
                let id = active.id;
                lose(app, id, &error.to_string());
                self.operation = None;
                return;
            }
            Err(_) => return,
        };
        self.apply_runner_event(app, event).await;
    }

    pub(super) async fn apply_runner_event(
        &mut self,
        app: &mut App,
        event: MaintenanceSstateRunnerEvent,
    ) {
        let Some(active) = self.operation.as_mut() else {
            return;
        };
        match event {
            MaintenanceSstateRunnerEvent::Started { id } => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::SessionRunning {
                        id,
                        started_at: SystemTime::now(),
                    }),
                );
            }
            MaintenanceSstateRunnerEvent::Output {
                id,
                stream,
                line,
                truncated,
            } => {
                if let OperationStage::CleanupPreview(stage) = &mut active.stage
                    && stream == yoctui_model::MaintenanceOutputStream::Stdout
                {
                    stage.stdout.push(line.clone());
                }
                let text = if truncated {
                    format!("{line} [truncated]")
                } else {
                    line
                };
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::SessionOutput { id, stream, text }),
                );
            }
            MaintenanceSstateRunnerEvent::Completed { id, exit_code } => {
                if matches!(active.stage, OperationStage::CleanupPreview(_)) {
                    self.advance_cleanup(app).await;
                } else {
                    let evidence = match self.capture_evidence() {
                        Ok(evidence) => evidence,
                        Err(message) => {
                            fail(
                                app,
                                id,
                                &format!("Maintenance evidence validation failed: {message}"),
                            );
                            self.operation = None;
                            return;
                        }
                    };
                    let _ = update(
                        app,
                        Action::Maintenance(MaintenanceAction::CompleteSession {
                            id,
                            exit_code: exit_code.unwrap_or(0),
                            evidence,
                            finished_at: SystemTime::now(),
                        }),
                    );
                    self.operation = None;
                    if let Some(request) = self.deferred_archive_push.take()
                        && let Some(capability_request) = app.maintenance.capability.request()
                    {
                        self.start_inspection(InspectionPurpose::PreviewGitArchivePush {
                            capability_request,
                            request: Box::new(request),
                        });
                    }
                }
            }
            MaintenanceSstateRunnerEvent::Failed { id, exit_code } => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::FailSession {
                        id,
                        message: "Maintenance command exited unsuccessfully".into(),
                        exit_code,
                        finished_at: SystemTime::now(),
                    }),
                );
                self.operation = None;
            }
            MaintenanceSstateRunnerEvent::CancellationRequested { .. } => {}
            MaintenanceSstateRunnerEvent::Cancelled { id, .. } => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::CancelSession {
                        id,
                        finished_at: SystemTime::now(),
                    }),
                );
                self.operation = None;
            }
            MaintenanceSstateRunnerEvent::CancellationRejected { id, message } => {
                reject_cancellation(app, id, &message);
            }
            MaintenanceSstateRunnerEvent::TimedOut { id, .. } => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::TimeoutSession {
                        id,
                        finished_at: SystemTime::now(),
                    }),
                );
                self.operation = None;
            }
            MaintenanceSstateRunnerEvent::Lost { id, message } => {
                lose(app, id, &message);
                self.operation = None;
            }
        }
    }

    pub(super) async fn advance_cleanup(&mut self, app: &mut App) {
        let active = self
            .operation
            .as_mut()
            .expect("cleanup operation is active");
        let OperationStage::CleanupPreview(stage) = &active.stage else {
            return;
        };
        let result = parse_cleanup_preview(stage.confirmed.request.clone(), &stage.stdout)
            .and_then(|fresh| {
                MaintenanceSstateCommandSpec::cleanup_execution(
                    active.id,
                    stage.capability_request,
                    &stage.snapshot,
                    stage.operation_id,
                    &stage.confirmed,
                    &fresh,
                )
            });
        let (_, command) = match result {
            Ok(value) => value,
            Err(error) => {
                fail(app, active.id, &error.to_string());
                self.operation = None;
                return;
            }
        };
        let runner = active.runner.as_mut().expect("cleanup runner is owned");
        if let Err(error) = runner.start(command).await {
            fail(app, active.id, &error.to_string());
            self.operation = None;
            return;
        }
        active.stage = OperationStage::Execute(Box::new(EvidencePlan::None));
    }

    pub(super) fn capture_evidence(&mut self) -> Result<Vec<MaintenanceEvidence>, String> {
        let active = self
            .operation
            .as_ref()
            .expect("completed operation is active");
        let OperationStage::Execute(plan) = &active.stage else {
            return Ok(Vec::new());
        };
        let plan = plan.as_ref().clone();
        match plan {
            EvidencePlan::None => Ok(Vec::new()),
            EvidencePlan::Release(before) => {
                before.changed_evidence().map_err(|error| error.to_string())
            }
            EvidencePlan::GitArchive(request) => {
                let result =
                    GitArchiveLocalResult::capture(&request).map_err(|error| error.to_string())?;
                let evidence =
                    MaintenanceEvidence::new(result.head.clone(), "Git archive HEAD".into())
                        .map_err(str::to_owned)?;
                self.local_archive = Some(result);
                if request.push_remote.is_some() {
                    self.deferred_archive_push = Some(request);
                }
                Ok(vec![evidence])
            }
            EvidencePlan::PrExport(path) => Ok(vec![file_evidence(&path, "PR service export")?]),
        }
    }

    pub(crate) fn revalidate_evidence(
        &self,
        identity: &MaintenanceFileIdentity,
    ) -> Result<PathBuf, String> {
        let current = file_identity(&identity.path)?;
        if &current != identity {
            return Err(format!(
                "Maintenance evidence changed: {}",
                identity.path.display()
            ));
        }
        Ok(current.path)
    }
}
