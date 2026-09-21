use super::*;

impl MaintenanceCliCoordinator {
    pub(super) fn next_id(&mut self) -> MaintenanceSessionId {
        let id = MaintenanceSessionId(self.next_preview_id);
        self.next_preview_id = self.next_preview_id.wrapping_add(1).max(1);
        id
    }

    pub(super) fn exact_snapshot(
        &self,
        app: &App,
        capability_request: u64,
        fresh: &MaintenanceCapabilitySnapshot,
    ) -> bool {
        app.maintenance.capability.request() == Some(capability_request)
            && self.snapshot.as_ref() == Some(fresh)
    }

    pub(super) fn preview_readiness(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: SstateReadinessRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification = Some(
                "Maintenance capability changed; refresh and reopen the readiness form".into(),
            );
            return;
        }
        let id = self.next_id();
        match MaintenanceSstateCommandSpec::readiness(
            id,
            capability_request,
            &snapshot,
            id.0,
            request,
        ) {
            Ok((preview, _)) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                );
            }
            Err(error) => app.notification = Some(error.to_string()),
        }
    }

    pub(super) fn preview_pr_service(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: yoctui_model::PrServiceRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification = Some(
                "Maintenance capability changed; refresh and reopen the PR service form".into(),
            );
            return;
        }
        let id = self.next_id();
        match pr_service_command(id, capability_request, &snapshot, id.0, request) {
            Ok((preview, _)) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                );
            }
            Err(error) => app.notification = Some(error.to_string()),
        }
    }

    pub(super) fn preview_locked_signature_cache(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: LockedSignatureCacheRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification = Some(
                "Maintenance capability changed; refresh and reopen the locked-cache form".into(),
            );
            return;
        }
        let id = self.next_id();
        match locked_signature_command(id, capability_request, &snapshot, id.0, request) {
            Ok((preview, _, _)) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                );
            }
            Err(error) => app.notification = Some(error.to_string()),
        }
    }

    pub(super) fn preview_buildhistory(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: BuildComparisonRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification = Some(
                "Maintenance capability changed; refresh and reopen the build-history form".into(),
            );
            return;
        }
        let id = self.next_id();
        match buildhistory_command(id, capability_request, &snapshot, id.0, request) {
            Ok((preview, _)) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                );
            }
            Err(error) => app.notification = Some(error.to_string()),
        }
    }

    pub(super) fn preview_git_archive(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: GitArchiveRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification =
                Some("Maintenance capability changed; refresh and reopen the archive form".into());
            return;
        }
        let id = self.next_id();
        match git_archive_local_command(id, capability_request, &snapshot, id.0, &request) {
            Ok((preview, _)) => {
                self.archive_intent = Some((preview.id, request));
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                );
            }
            Err(error) => app.notification = Some(error.to_string()),
        }
    }

    pub(super) fn preview_git_archive_push(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: GitArchiveRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification = Some(
                "Maintenance capability changed; refresh before confirming archive push".into(),
            );
            return;
        }
        let Some(local) = self.local_archive.clone() else {
            app.notification = Some("local Git archive evidence is unavailable".into());
            return;
        };
        let id = self.next_id();
        match git_archive_push_command(id, capability_request, &snapshot, id.0, request, &local) {
            Ok((preview, _)) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                );
            }
            Err(error) => app.notification = Some(error.to_string()),
        }
    }

    pub(super) async fn begin_cleanup_preview(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: SstateCleanupRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification =
                Some("Maintenance capability changed; refresh and reopen the cleanup form".into());
            return;
        }
        let id = self.next_id();
        let command =
            match MaintenanceSstateCommandSpec::cleanup_preview(id, &snapshot, request.clone()) {
                Ok(command) => command,
                Err(error) => {
                    app.notification = Some(error.to_string());
                    return;
                }
            };
        let mut runner = MaintenanceSstateJobRunner::new();
        if let Err(error) = runner.start(command).await {
            app.notification = Some(error.to_string());
            return;
        }
        self.cleanup_preview = Some(MaintenanceCleanupPreviewOperation {
            id,
            capability_request,
            snapshot,
            request,
            runner,
            stdout: Vec::new(),
            last_stderr: None,
        });
    }

    pub(super) async fn poll_cleanup_preview(&mut self, app: &mut App) {
        let Some(active) = self.cleanup_preview.as_mut() else {
            return;
        };
        let event = match tokio::time::timeout(Duration::from_millis(1), active.runner.next_event())
            .await
        {
            Ok(Ok(event)) => event,
            Ok(Err(error)) => {
                app.notification = Some(format!("sstate cleanup preview was lost: {error}"));
                self.cleanup_preview = None;
                return;
            }
            Err(_) => return,
        };
        match event {
            MaintenanceSstateRunnerEvent::Started { .. }
            | MaintenanceSstateRunnerEvent::CancellationRequested { .. } => {}
            MaintenanceSstateRunnerEvent::Output {
                stream,
                line,
                truncated,
                ..
            } => match stream {
                yoctui_model::MaintenanceOutputStream::Stdout
                    if !truncated && active.stdout.len() <= MAX_MAINTENANCE_PATHS =>
                {
                    active.stdout.push(line);
                }
                yoctui_model::MaintenanceOutputStream::Stderr => {
                    active.last_stderr = Some(line);
                }
                _ => {}
            },
            MaintenanceSstateRunnerEvent::Completed { .. } => {
                let active = self.cleanup_preview.take().expect("preview was checked");
                if !self.exact_snapshot(app, active.capability_request, &active.snapshot) {
                    app.notification = Some(
                        "Maintenance capability changed during cleanup discovery; reopen the form"
                            .into(),
                    );
                    return;
                }
                let preview =
                    parse_cleanup_preview(active.request, &active.stdout).and_then(|fresh| {
                        MaintenanceSstateCommandSpec::cleanup_execution(
                            active.id,
                            active.capability_request,
                            &active.snapshot,
                            active.id.0,
                            &fresh,
                            &fresh,
                        )
                        .map(|(preview, _)| preview)
                    });
                match preview {
                    Ok(preview) => {
                        app.notification = None;
                        let _ = update(
                            app,
                            Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                        );
                    }
                    Err(error) => app.notification = Some(error.to_string()),
                }
            }
            MaintenanceSstateRunnerEvent::Failed { exit_code, .. } => {
                let stderr = active
                    .last_stderr
                    .clone()
                    .unwrap_or_else(|| "no stderr".into());
                app.notification = Some(format!(
                    "sstate cleanup preview failed with status {exit_code:?}: {stderr}"
                ));
                self.cleanup_preview = None;
            }
            MaintenanceSstateRunnerEvent::Cancelled { .. } => {
                app.notification = Some("sstate cleanup preview cancelled".into());
                self.cleanup_preview = None;
            }
            MaintenanceSstateRunnerEvent::CancellationRejected { message, .. }
            | MaintenanceSstateRunnerEvent::Lost { message, .. } => {
                app.notification = Some(message);
                self.cleanup_preview = None;
            }
            MaintenanceSstateRunnerEvent::TimedOut { .. } => {
                app.notification = Some("sstate cleanup preview timed out".into());
                self.cleanup_preview = None;
            }
        }
    }
}
