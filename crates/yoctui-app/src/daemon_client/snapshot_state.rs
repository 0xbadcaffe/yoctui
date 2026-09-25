use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonClientSnapshot {
    pub status: yoctui_model::ClientReplicaStatus,
    pub snapshot: Option<yoctui_protocol::daemon::DaemonSnapshot>,
    pub telemetry: Option<yoctui_protocol::daemon::DaemonTelemetry>,
    pub(crate) compatibility_reconciled: bool,
    pub(crate) reconciled_compatibility_generation: Option<u64>,
}

impl Default for DaemonClientSnapshot {
    fn default() -> Self {
        Self {
            status: yoctui_model::ClientReplicaStatus::Disconnected,
            snapshot: None,
            telemetry: None,
            compatibility_reconciled: false,
            reconciled_compatibility_generation: None,
        }
    }
}

impl DaemonClientSnapshot {
    pub fn begin_synchronization(&mut self) {
        self.status = yoctui_model::ClientReplicaStatus::Synchronizing;
    }

    pub fn replace(&mut self, snapshot: yoctui_protocol::daemon::DaemonSnapshot) {
        self.snapshot = Some(snapshot);
        self.telemetry = None;
        self.compatibility_reconciled = false;
        self.reconciled_compatibility_generation = None;
        self.status = yoctui_model::ClientReplicaStatus::Current;
    }

    pub fn replace_app(
        &mut self,
        app: &mut yoctui_model::App,
        snapshot: yoctui_protocol::daemon::DaemonSnapshot,
    ) {
        self.replace(snapshot);
        self.install_build_app(app);
        self.install_app(app);
    }

    pub fn apply_event(
        &mut self,
        event: &yoctui_protocol::daemon::SequencedEvent,
    ) -> Result<(), DaemonClientSyncError> {
        if let yoctui_protocol::daemon::DaemonEvent::Telemetry(telemetry) = &event.event {
            self.telemetry = Some(*telemetry);
        }
        let Some(snapshot) = self.snapshot.as_mut() else {
            self.status = yoctui_model::ClientReplicaStatus::Stale;
            return Err(DaemonClientSyncError::MissingSnapshot);
        };
        if let Err(error) = yoctui_protocol::daemon::apply_sequenced_event(snapshot, event) {
            self.status = yoctui_model::ClientReplicaStatus::Stale;
            return Err(DaemonClientSyncError::Protocol(error));
        }
        self.status = yoctui_model::ClientReplicaStatus::Current;
        Ok(())
    }

    pub fn apply_event_to_app(
        &mut self,
        app: &mut yoctui_model::App,
        event: &yoctui_protocol::daemon::SequencedEvent,
    ) -> Result<(), DaemonClientSyncError> {
        self.apply_event(event)?;
        match &event.event {
            yoctui_protocol::daemon::DaemonEvent::Build(build_event) => {
                apply_daemon_build_event(app, build_event.clone());
                install_daemon_build_environment(app);
            }
            yoctui_protocol::daemon::DaemonEvent::Log(record) => {
                let _ = yoctui_model::update(app, daemon_log_action(record));
            }
            _ => {}
        }
        self.install_app(app);
        Ok(())
    }

    pub fn apply_log_events_to_app(
        &mut self,
        app: &mut yoctui_model::App,
        events: &[yoctui_protocol::daemon::SequencedEvent],
    ) -> Result<(), DaemonClientSyncError> {
        let mut entries = Vec::with_capacity(events.len());
        for event in events {
            let yoctui_protocol::daemon::DaemonEvent::Log(record) = &event.event else {
                return Err(DaemonClientSyncError::NonLogEventInLogBatch);
            };
            self.apply_event(event)?;
            entries.push(daemon_log_entry(record));
        }
        if !entries.is_empty() {
            let _ = yoctui_model::update(app, yoctui_model::Action::Logs(entries));
            self.install_app(app);
        }
        Ok(())
    }

    pub fn apply_task_events_to_app(
        &mut self,
        app: &mut yoctui_model::App,
        events: &[yoctui_protocol::daemon::SequencedEvent],
    ) -> Result<(), DaemonClientSyncError> {
        let mut task_events = Vec::with_capacity(events.len());
        for event in events {
            let yoctui_protocol::daemon::DaemonEvent::Build(build_event) = &event.event else {
                return Err(DaemonClientSyncError::NonTaskEventInTaskBatch);
            };
            let Some(task_event) = model_task_event_from_daemon(build_event) else {
                return Err(DaemonClientSyncError::NonTaskEventInTaskBatch);
            };
            self.apply_event(event)?;
            task_events.push(task_event);
        }
        if !task_events.is_empty() {
            let _ = yoctui_model::update(app, yoctui_model::Action::TaskEvents(task_events));
            install_daemon_build_environment(app);
            self.install_app(app);
        }
        Ok(())
    }

    pub fn install_app(&mut self, app: &mut yoctui_model::App) {
        app.observe_daemon(
            self.status == yoctui_model::ClientReplicaStatus::Current,
            std::time::SystemTime::now(),
        );
        app.daemon = daemon_client_view(self.status, self.snapshot.as_ref(), self.telemetry);
        app.reconcile_platform_menuconfigs();
        if self.status == yoctui_model::ClientReplicaStatus::Current
            && let Some(progress) = self
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.build_progress)
        {
            let total = progress.total.filter(|total| *total > 0);
            app.build.cache = progress.cache;
            if (app.build.completed, app.build.total) != (progress.completed, total) {
                app.build.completed = progress.completed;
                app.build.total = total;
                app.invalidate_task_projection();
            }
        }
        let wire = (self.status == yoctui_model::ClientReplicaStatus::Current)
            .then(|| {
                self.snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.compatibility.as_ref())
            })
            .flatten();
        let wire_generation = wire.map(|wire| wire.generation);
        let reconcile_compatibility = self.status != yoctui_model::ClientReplicaStatus::Current
            || !self.compatibility_reconciled
            || self.reconciled_compatibility_generation != wire_generation;
        if reconcile_compatibility {
            match wire {
                Some(wire) => match compatibility_model_snapshot(wire) {
                    Ok(authority) => {
                        if let Err(error) =
                            yoctui_model::install_workspace_compatibility(app, authority)
                        {
                            app.notification = Some(format!(
                                "Compatibility authority update was rejected: {error}"
                            ));
                        }
                    }
                    Err(error) => {
                        yoctui_model::invalidate_workspace_compatibility(app);
                        app.notification = Some(format!(
                            "Compatibility authority could not be decoded and was invalidated: {error}"
                        ));
                    }
                },
                None => {
                    yoctui_model::invalidate_workspace_compatibility(app);
                }
            }
            self.compatibility_reconciled =
                self.status == yoctui_model::ClientReplicaStatus::Current;
            self.reconciled_compatibility_generation = wire_generation;
        }
        if self.status == yoctui_model::ClientReplicaStatus::Current {
            if let Some(snapshot) = &self.snapshot
                && let Err(error) = install_raw_execution_snapshots(app, &snapshot.raw_executions)
            {
                app.raw_mode.execution_states.clear();
                app.notification = Some(format!(
                    "Raw execution snapshot was rejected and invalidated: {error}"
                ));
            }
            if let Some(snapshot) = &self.snapshot
                && let Err(error) = install_raw_history_snapshots(app, &snapshot.raw_history)
            {
                app.raw_mode.history.clear();
                app.notification = Some(format!(
                    "Raw history snapshot was rejected and invalidated: {error}"
                ));
            }
        } else {
            app.raw_mode.execution_states.clear();
            app.raw_mode.history.clear();
        }
    }

    pub(crate) fn install_build_app(&self, app: &mut yoctui_model::App) {
        let Some(snapshot) = self.snapshot.as_ref() else {
            return;
        };
        let mut build = yoctui_model::App::new(app.logs.max_entries, app.logs.max_bytes);
        build.backend = "bridge".into();
        for event in snapshot.build_events.iter().cloned() {
            apply_daemon_build_event(&mut build, event);
        }
        if let Some(identity) = &snapshot.workspace {
            build
                .workspace
                .source_dir
                .get_or_insert_with(|| identity.canonical_source.clone().into());
            build
                .workspace
                .build_dir
                .get_or_insert_with(|| identity.canonical_build.clone().into());
        }
        for record in &snapshot.recent_logs {
            let _ = yoctui_model::update(&mut build, daemon_log_action(record));
        }
        preserve_log_presentation(&app.logs, &mut build.logs);
        let replaces_terminal_record = matches!(
            app.build.status,
            yoctui_model::BuildStatus::Completed
                | yoctui_model::BuildStatus::Cancelled
                | yoctui_model::BuildStatus::Failed
        ) && app.build.started == build.build.started
            && app.build.target == build.build.target;
        app.backend = build.backend;
        app.workspace = build.workspace;
        app.build = build.build;
        app.tasks = build.tasks;
        app.completed_tasks = build.completed_tasks;
        if let Some(record) = build.build_history.pop_back() {
            // Terminal summary timing belongs to this authoritative replay,
            // not an older client-local record (or its attachment clock).
            if replaces_terminal_record && !app.build_history.is_empty() {
                *app.build_history.back_mut().unwrap() = record;
            } else {
                app.build_history.push_back(record);
                while app.build_history.len() > yoctui_model::MAX_BUILD_HISTORY {
                    app.build_history.pop_front();
                }
            }
            app.build_history_selection = app
                .build_history_selection
                .min(app.build_history.len().saturating_sub(1));
        }
        app.invalidate_task_projection();
        app.logs = build.logs;
        install_daemon_build_environment(app);
    }

    pub fn resume_cursor(&self) -> Option<yoctui_protocol::daemon::ResumeCursor> {
        if self.status != yoctui_model::ClientReplicaStatus::Current {
            return None;
        }
        self.snapshot
            .as_ref()
            .map(|snapshot| yoctui_protocol::daemon::ResumeCursor {
                daemon_instance_id: snapshot.daemon_instance_id,
                last_sequence: snapshot.sequence,
            })
    }

    pub fn disconnect(&mut self) {
        self.status = yoctui_model::ClientReplicaStatus::Disconnected;
        self.compatibility_reconciled = false;
        self.reconciled_compatibility_generation = None;
    }

    pub fn disconnect_app(&mut self, app: &mut yoctui_model::App) {
        self.disconnect();
        self.install_app(app);
    }
}
