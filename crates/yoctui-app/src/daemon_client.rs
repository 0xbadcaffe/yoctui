//! Daemon client.
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

pub(crate) fn preserve_log_presentation(
    previous: &yoctui_model::LogState,
    replacement: &mut yoctui_model::LogState,
) {
    replacement.follow = previous.follow;
    replacement.paused_len = (!previous.follow).then_some(replacement.entries.len());
    replacement.wrap = previous.wrap;
    replacement.filter = previous.filter;
    replacement.recipe_filter = previous.recipe_filter.clone();
    replacement.task_filter = previous.task_filter.clone();
    replacement.build_filter = previous.build_filter.clone();
    replacement.source_filter = previous.source_filter.clone();
    replacement.time_range = previous.time_range.clone();
    replacement.query.clone_from(&previous.query);
    replacement.searching = previous.searching;
    replacement.horizontal_offset = previous
        .horizontal_offset
        .min(replacement.maximum_horizontal_offset());
    let count = replacement.filtered().count();
    replacement.selection = if previous.follow {
        count.saturating_sub(1)
    } else {
        previous.selection.min(count.saturating_sub(1))
    };
    replacement.scroll_offset = count.saturating_sub(replacement.selection.saturating_add(1));
}

pub(crate) fn daemon_log_action(
    record: &yoctui_protocol::daemon::LogRecord,
) -> yoctui_model::Action {
    yoctui_model::Action::Log(daemon_log_entry(record))
}

pub(crate) fn daemon_log_entry(
    record: &yoctui_protocol::daemon::LogRecord,
) -> yoctui_model::LogEntry {
    use std::time::{Duration, SystemTime};
    let severity = match record.severity {
        yoctui_protocol::daemon::LogSeverity::Trace => yoctui_model::Severity::Trace,
        yoctui_protocol::daemon::LogSeverity::Info => yoctui_model::Severity::Info,
        yoctui_protocol::daemon::LogSeverity::Warning => yoctui_model::Severity::Warning,
        yoctui_protocol::daemon::LogSeverity::Error => yoctui_model::Severity::Error,
    };
    yoctui_model::LogEntry {
        id: 0,
        severity,
        message: record.message.clone(),
        recipe: record.recipe.clone(),
        task: record.task.clone(),
        path: record.path.clone().map(Into::into),
        timestamp: SystemTime::UNIX_EPOCH + Duration::from_millis(record.unix_ms),
        build: record.build.clone(),
        protected: matches!(
            severity,
            yoctui_model::Severity::Warning | yoctui_model::Severity::Error
        ),
        diagnostic: None,
    }
}

pub(crate) fn apply_daemon_build_event(
    app: &mut yoctui_model::App,
    event: yoctui_protocol::daemon::DaemonBuildEvent,
) {
    use yoctui_protocol::daemon::DaemonBuildEvent;
    if matches!(
        &event,
        DaemonBuildEvent::TaskQueued { .. }
            | DaemonBuildEvent::TaskStarted { .. }
            | DaemonBuildEvent::TaskProgress { .. }
            | DaemonBuildEvent::TaskCompleted { .. }
    ) {
        if let Some(task) = model_task_event_from_daemon(&event) {
            let _ = yoctui_model::update(app, yoctui_model::Action::TaskEvents(vec![task]));
        }
        return;
    }
    let started = match &event {
        DaemonBuildEvent::Started { started_unix_ms } => {
            Some(observed_daemon_time(*started_unix_ms))
        }
        _ => None,
    };
    let finished = match &event {
        DaemonBuildEvent::Completed {
            finished_unix_ms, ..
        } => Some(observed_daemon_time(*finished_unix_ms)),
        _ => None,
    };
    let disconnected = matches!(&event, DaemonBuildEvent::Disconnected);
    let action = match event {
        DaemonBuildEvent::Reset { targets } => yoctui_model::Action::BuildRequested {
            target: targets.into_iter().next(),
        },
        DaemonBuildEvent::Completed {
            success: false,
            exit_code,
            ..
        } if app.build.status == yoctui_model::BuildStatus::Cancelling => {
            yoctui_model::Action::BuildCancelled { exit_code }
        }
        event => match backend_event_from_daemon(event) {
            Some(event) => match model_action_from_backend_event(event) {
                Some(action) => action,
                None => return,
            },
            None => return,
        },
    };
    let _ = yoctui_model::update(app, action);
    if let Some(started) = started {
        app.build.started = started;
    }
    if let Some(finished) = finished {
        if let Some(record) = app.build_history.back_mut() {
            record.elapsed = app
                .build
                .started
                .and_then(|start| finished?.duration_since(start).ok());
        }
        for completed in &mut app.completed_tasks {
            if matches!(
                completed.task.cancellation.as_deref(),
                Some("build ended" | "cancelled")
            ) {
                completed.task.finished = finished;
            }
        }
        app.invalidate_task_projection();
    } else if disconnected && app.build.status == yoctui_model::BuildStatus::Lost {
        for completed in &mut app.completed_tasks {
            if completed.task.cancellation.as_deref() == Some("build authority lost") {
                completed.task.finished = None;
            }
        }
        app.invalidate_task_projection();
    }
}

pub(crate) fn observed_daemon_time(unix_ms: Option<u64>) -> Option<SystemTime> {
    // The UI's date formatter supports years through 9999. An unsupported
    // numeric timestamp is missing timing, never an overflowing clock value.
    let unix_ms = unix_ms.filter(|value| *value <= 253_402_300_799_999)?;
    SystemTime::UNIX_EPOCH.checked_add(std::time::Duration::from_millis(unix_ms))
}

pub(crate) fn model_task_event_from_daemon(
    event: &yoctui_protocol::daemon::DaemonBuildEvent,
) -> Option<yoctui_model::TaskEvent> {
    match model_action_from_backend_event(backend_event_from_daemon(event.clone())?)? {
        yoctui_model::Action::TaskStarted(mut task) => {
            let yoctui_protocol::daemon::DaemonBuildEvent::TaskStarted {
                started_unix_ms, ..
            } = event
            else {
                return None;
            };
            task.started = observed_daemon_time(*started_unix_ms);
            Some(yoctui_model::TaskEvent::ObservedStarted(task))
        }
        yoctui_model::Action::TaskQueued(task) => Some(yoctui_model::TaskEvent::Queued(task)),
        yoctui_model::Action::TaskProgress { id, progress } => {
            Some(yoctui_model::TaskEvent::Progress { id, progress })
        }
        yoctui_model::Action::TaskCompleted { id, success } => {
            let yoctui_protocol::daemon::DaemonBuildEvent::TaskCompleted {
                started_unix_ms,
                finished_unix_ms,
                ..
            } = event
            else {
                return None;
            };
            Some(yoctui_model::TaskEvent::ObservedCompleted {
                id,
                success,
                timing: yoctui_model::ObservedTaskTiming {
                    started: observed_daemon_time(*started_unix_ms),
                    finished: observed_daemon_time(*finished_unix_ms),
                },
            })
        }
        _ => None,
    }
}

pub(crate) fn backend_event_from_daemon(
    event: yoctui_protocol::daemon::DaemonBuildEvent,
) -> Option<BackendEvent> {
    use yoctui_protocol::daemon::DaemonBuildEvent;
    Some(match event {
        DaemonBuildEvent::Reset { .. } => return None,
        DaemonBuildEvent::Workspace { data } => BackendEvent::Workspace(yoctui_model::Workspace {
            build_dir: data.build_dir.map(Into::into),
            source_dir: data.source_dir.map(Into::into),
            variables: data.variables,
            variable_provenance: data.variable_provenance,
            variable_provenance_chain: data.variable_provenance_chain,
            bitbake_version: data.bitbake_version,
            release: data.release,
            layers: data
                .layers
                .into_iter()
                .map(|layer| yoctui_model::Layer {
                    name: layer.name,
                    path: layer.path.into(),
                    priority: layer.priority,
                })
                .collect(),
            recipes: data
                .recipes
                .into_iter()
                .map(|recipe| yoctui_model::Recipe {
                    name: recipe.name,
                    version: recipe.version,
                    layer: recipe.layer,
                    preferred_version: recipe.preferred_version,
                    file: recipe.file.map(Into::into),
                    append_count: recipe.append_count,
                })
                .collect(),
        }),
        DaemonBuildEvent::Started { .. } => BackendEvent::BuildStarted,
        DaemonBuildEvent::ParseProgress { current, total } => {
            BackendEvent::ParseProgress { current, total }
        }
        DaemonBuildEvent::SstateSummary { summary } => BackendEvent::SstateSummary(summary),
        DaemonBuildEvent::TaskQueued {
            recipe,
            task,
            worker,
            stats,
        } => BackendEvent::TaskQueued {
            recipe,
            task,
            worker,
            stats: stats.map(daemon_task_stats),
        },
        DaemonBuildEvent::TaskStarted {
            recipe,
            task,
            pid,
            worker,
            log_path,
            stats,
            ..
        } => BackendEvent::TaskStarted {
            recipe,
            task,
            pid,
            worker,
            log_path: log_path.map(Into::into),
            stats: stats.map(daemon_task_stats),
        },
        DaemonBuildEvent::TaskProgress {
            recipe,
            task,
            progress,
        } => BackendEvent::TaskProgress {
            recipe,
            task,
            progress,
        },
        DaemonBuildEvent::TaskCompleted {
            recipe,
            task,
            success,
            ..
        } => BackendEvent::TaskCompleted {
            recipe,
            task,
            success,
        },
        DaemonBuildEvent::Completed {
            success, exit_code, ..
        } => BackendEvent::BuildCompleted { success, exit_code },
        DaemonBuildEvent::CommandFailed { code, message } => {
            BackendEvent::CommandFailed { code, message }
        }
        DaemonBuildEvent::Disconnected => BackendEvent::Disconnected,
    })
}

pub(crate) fn daemon_task_stats(stats: yoctui_protocol::TaskStatsData) -> yoctui_model::TaskStats {
    yoctui_model::TaskStats {
        completed: stats.completed,
        total: stats.total,
        active: stats.active,
        failed: stats.failed,
    }
}

pub(crate) fn install_daemon_build_environment(app: &mut yoctui_model::App) {
    let (Some(source_dir), Some(build_dir)) = (
        app.workspace.source_dir.clone(),
        app.workspace.build_dir.clone(),
    ) else {
        return;
    };
    app.build_environment =
        yoctui_model::BuildEnvironmentState::Connected(yoctui_model::BuildEnvironmentProfile {
            init_script: source_dir.join("oe-init-build-env"),
            source_dir,
            build_dir,
        });
}
