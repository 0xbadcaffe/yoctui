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
