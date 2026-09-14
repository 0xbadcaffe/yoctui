//! Inspector render.
use super::*;

pub(crate) fn task_inspector_primary(inspector: &TaskInspectorRef<'_>) -> String {
    match inspector {
        TaskInspectorRef::None => {
            "No task selected.\nTask facts appear as typed BitBake events arrive.".into()
        }
        TaskInspectorRef::Waiting { count } => format!(
            "Task        unavailable\nRecipe      unavailable\nPN          unavailable\nPV          unavailable\nPR          unavailable\nState       {}\nProgress    unavailable\n\n{count} waiting tasks are known only as an aggregate.",
            task_state_label(TaskState::Waiting)
        ),
        TaskInspectorRef::Task {
            task,
            state,
            version,
            revision,
            ..
        } => format!(
            "Task        {}\nRecipe      {}\nPN          {}\nPV          {}\nPR          {}\nState       {}\nProgress    {}",
            task.task,
            task.recipe,
            task.recipe,
            version.unwrap_or("unavailable"),
            revision.unwrap_or("unavailable"),
            task_state_label(*state),
            task.progress
                .map_or_else(|| "unknown".into(), |value| format!("{value}%")),
        ),
    }
}

pub(crate) fn task_inspector_context(inspector: &TaskInspectorRef<'_>, now: SystemTime) -> String {
    match inspector {
        TaskInspectorRef::None => "Secondary facts, paths, and dependencies unavailable.".into(),
        TaskInspectorRef::Waiting { .. } => {
            "Worker      unavailable\nPID         unavailable\nStarted     unavailable\nElapsed     unavailable\nWorkdir     unavailable\nLog file    unavailable\nDependencies unavailable".into()
        }
        TaskInspectorRef::Task {
            task,
            workdir,
            ..
        } => {
            let dependencies = if task.dependencies.is_empty() {
                "none reported".into()
            } else {
                task.dependencies
                    .iter()
                    .map(|dependency| dependency.0.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!(
                "Worker      {}\nPID         {}\nStarted     {}\nElapsed     {}\nWorkdir     {}\nLog file    {}\nDependencies {}",
                task.worker.as_deref().unwrap_or("unavailable"),
                task.pid
                    .map_or_else(|| "unavailable".into(), |pid| pid.to_string()),
                task.started
                    .map_or_else(|| "unavailable".into(), clock_text),
                task.elapsed_at(now)
                    .map_or_else(|| "unavailable".into(), format_duration),
                workdir.map_or_else(
                    || "unavailable".into(),
                    |path| path.display().to_string()
                ),
                task.log_path.as_ref().map_or_else(
                    || "unavailable".into(),
                    |path| path.display().to_string()
                ),
                dependencies,
            )
        }
    }
}

pub(crate) fn task_inspector_recent_lines<'a>(
    app: &App,
    inspector: &'a TaskInspectorRef<'a>,
) -> Vec<Line<'a>> {
    let TaskInspectorRef::Task { recent_logs, .. } = inspector else {
        return vec![Line::from("No task-specific log is available.")];
    };
    if recent_logs.is_empty() {
        return vec![Line::from(
            "Waiting for typed log entries for the selected task.",
        )];
    }
    recent_logs
        .iter()
        .map(|entry| {
            let marker = match entry.severity {
                Severity::Trace => "·",
                Severity::Info => "│",
                Severity::Warning => "!",
                Severity::Error => "✕",
            };
            Line::from(vec![
                Span::styled(format!("{marker} "), severity_style(app, entry.severity)),
                Span::styled(entry.message.as_str(), severity_style(app, entry.severity)),
            ])
        })
        .collect()
}

pub(crate) fn inspector_action_styles(app: &App) -> ActionListStyles {
    let palette = ThemePalette::for_app(app);
    ActionListStyles {
        enabled: palette.role(palette.primary_foreground, Modifier::empty()),
        disabled: palette.role(palette.disabled, Modifier::DIM),
        shortcut: palette.role(palette.accent, Modifier::BOLD),
        detail: palette.role(palette.secondary_foreground, Modifier::ITALIC),
    }
}

pub(crate) fn task_inspector_actions(app: &App) -> Vec<ActionListItem> {
    let active_build = matches!(
        app.build.status,
        BuildStatus::Running | BuildStatus::Parsing | BuildStatus::Cancelling
    );
    let mut actions = compatibility_workspace_actions(app, WorkspaceDestination::Tasks)
        .into_iter()
        .filter(|action| action.label != "Inspect task inventory")
        .map(|mut action| {
            if action.label == "Cancel active build" && !active_build {
                action.enabled = false;
                action.marker = "×";
                action.state = "Disabled".into();
                action
                    .details
                    .insert(0, "Reason: No active build can be cancelled.".into());
            }
            action
        })
        .collect::<Vec<_>>();
    actions.sort_by_key(|action| match action.label.as_str() {
        "Cancel active build" => 0,
        "Open Logs" => 1,
        "Build History" => 2,
        "Build options" => 3,
        _ => 4,
    });
    actions
}

pub(crate) fn bounded_status_line(value: String, width: u16) -> String {
    let width = usize::from(width);
    if value.chars().count() <= width {
        return value;
    }
    if width == 0 {
        return String::new();
    }
    value
        .chars()
        .take(width.saturating_sub(1))
        .chain(std::iter::once('…'))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SystemStatusLine {
    pub(crate) text: String,
    pub(crate) tone: StatusTone,
}

pub(crate) fn status_tone_style(palette: &ThemePalette, tone: StatusTone) -> Style {
    match tone {
        StatusTone::Success => palette.role(palette.success, Modifier::BOLD),
        StatusTone::Warning => palette.role(palette.warning, Modifier::BOLD),
        StatusTone::Error => palette.role(palette.error, Modifier::BOLD),
        StatusTone::Running => palette.role(palette.running, Modifier::BOLD),
        StatusTone::Pending => palette.role(palette.pending, Modifier::BOLD),
        StatusTone::Accent => palette.role(palette.accent, Modifier::BOLD),
        StatusTone::Muted => palette.role(palette.muted, Modifier::DIM),
        StatusTone::Info => palette.role(palette.informational, Modifier::BOLD),
        StatusTone::Disabled => palette.role(palette.disabled, Modifier::DIM),
    }
}

pub(crate) fn stronger_tone(left: StatusTone, right: StatusTone) -> StatusTone {
    let rank = |tone| match tone {
        StatusTone::Error => 4,
        StatusTone::Warning => 3,
        StatusTone::Pending => 2,
        StatusTone::Disabled | StatusTone::Muted => 1,
        StatusTone::Success | StatusTone::Running | StatusTone::Accent | StatusTone::Info => 0,
    };
    if rank(left) >= rank(right) {
        left
    } else {
        right
    }
}

pub(crate) fn system_status_projection(app: &App, width: u16) -> Vec<SystemStatusLine> {
    let access = app.client_access_origin.label();
    let current = app.daemon.status == yoctui_model::ClientReplicaStatus::Current;
    let uptime = if current {
        app.daemon.telemetry.as_ref().map_or_else(
            || "unavailable".into(),
            |telemetry| format_duration(Duration::from_secs(telemetry.uptime_seconds)),
        )
    } else {
        "unavailable".into()
    };
    let active_jobs = if current {
        app.daemon.telemetry.as_ref().map_or_else(
            || "unavailable".into(),
            |value| value.active_jobs.to_string(),
        )
    } else {
        "unavailable".into()
    };
    let sessions = if current {
        app.daemon.pty_sessions.len().to_string()
    } else {
        "unavailable".into()
    };
    let clients = if current {
        app.daemon.connected_clients.to_string()
    } else {
        "unavailable".into()
    };
    let pressure = if current {
        app.daemon
            .telemetry
            .as_ref()
            .map_or_else(String::new, |telemetry| {
                let pressure = telemetry.pressure;
                if pressure == yoctui_model::ClientDaemonPressureCounters::default() {
                    String::new()
                } else {
                    format!(
                        " · IPC Q {}/{} C{} D{} W{} R{} S{}",
                        pressure.current_queue_depth,
                        pressure.maximum_queue_depth,
                        pressure.cosmetic_coalesced,
                        pressure.cosmetic_dropped,
                        pressure.reliable_waits,
                        pressure.forced_resynchronizations,
                        pressure.slow_client_disconnects,
                    )
                }
            })
    } else {
        String::new()
    };
    let bitbake = if current {
        daemon_lifecycle_label(app.daemon.bitbake).to_owned()
    } else {
        "unavailable".into()
    };
    let bitbake_version = if current {
        app.workspace
            .bitbake_version
            .as_deref()
            .unwrap_or("unavailable")
    } else {
        "unavailable"
    };
    let workspace = app
        .workspace
        .build_dir
        .as_deref()
        .or(app.workspace.source_dir.as_deref())
        .map_or_else(|| "unavailable".into(), |path| path.display().to_string());
    let filesystem_sample = match (
        utilization_percent(
            app.host_telemetry.disk_total_bytes,
            app.host_telemetry.disk_available_bytes,
        ),
        app.host_telemetry.disk_total_bytes,
        app.host_telemetry.disk_available_bytes,
        app.workspace.build_dir.as_ref(),
    ) {
        (Some(percent), Some(total), Some(available), Some(_)) => {
            Some((percent, format_bytes_pair(available, total)))
        }
        _ => None,
    };
    let projection = app
        .compatibility_ui
        .project(&app.workspace_compatibility, app.daemon.status);
    let (compatibility, compatibility_tone) = match (current, projection.authority) {
        (false, _) => ("Unavailable".into(), StatusTone::Warning),
        (true, CompatibilityUiAuthorityStatus::Current { generation, mode }) => (
            format!(
                "{:?} g{generation} · A{} L{} U{} ?{}",
                mode,
                projection.summary.available,
                projection.summary.limited,
                projection.summary.unavailable,
                projection.summary.unknown + projection.summary.unsupported,
            ),
            match mode {
                yoctui_model::EnvironmentOperatingMode::Full => StatusTone::Success,
                yoctui_model::EnvironmentOperatingMode::Degraded
                | yoctui_model::EnvironmentOperatingMode::Diagnostic => StatusTone::Warning,
            },
        ),
        (true, CompatibilityUiAuthorityStatus::Unavailable { .. }) => {
            ("Unavailable".into(), StatusTone::Warning)
        }
    };
    let (daemon, daemon_tone) = match app.daemon.status {
        yoctui_model::ClientReplicaStatus::Disconnected => ("Disconnected", StatusTone::Error),
        yoctui_model::ClientReplicaStatus::Synchronizing => ("Synchronizing", StatusTone::Pending),
        yoctui_model::ClientReplicaStatus::Current => ("Connected", StatusTone::Success),
        yoctui_model::ClientReplicaStatus::Stale => ("Stale", StatusTone::Warning),
    };
    let bitbake_tone = if current {
        match app.daemon.bitbake {
            yoctui_model::ClientDaemonLifecycle::Running => StatusTone::Success,
            yoctui_model::ClientDaemonLifecycle::Connecting
            | yoctui_model::ClientDaemonLifecycle::Stopping => StatusTone::Pending,
            yoctui_model::ClientDaemonLifecycle::Failed
            | yoctui_model::ClientDaemonLifecycle::Lost => StatusTone::Error,
            yoctui_model::ClientDaemonLifecycle::Disconnected
            | yoctui_model::ClientDaemonLifecycle::Exited => StatusTone::Disabled,
        }
    } else {
        StatusTone::Disabled
    };
    let (filesystem, filesystem_tone) = filesystem_sample.map_or_else(
        || ("unavailable".into(), StatusTone::Warning),
        |(percent, pair)| {
            let tone = if percent >= 90 {
                StatusTone::Error
            } else if percent >= 70 {
                StatusTone::Warning
            } else {
                StatusTone::Success
            };
            (format!("{percent}% · {pair} free"), tone)
        },
    );
    let log_tone = if app.logs.dropped_errors > 0 {
        StatusTone::Error
    } else if app.logs.dropped > 0 {
        StatusTone::Warning
    } else {
        StatusTone::Success
    };
    let workspace_tone = if workspace == "unavailable" {
        StatusTone::Warning
    } else {
        StatusTone::Success
    };
    let host_tone = stronger_tone(stronger_tone(filesystem_tone, log_tone), workspace_tone);
    let host = if workspace == "unavailable" {
        format!("Workspace unknown · Build FS {filesystem}")
    } else if app.logs.dropped > 0 {
        format!(
            "Logs {} evicted ({} warning/{} error) · Build FS {filesystem} · Workspace {workspace}",
            app.logs.dropped, app.logs.dropped_warnings, app.logs.dropped_errors,
        )
    } else {
        format!("Build FS {filesystem} · Workspace {workspace}")
    };

    let lines = if width >= 40 {
        vec![
            (
                format!("Daemon {daemon} · {access} · version unavailable · up {uptime}"),
                daemon_tone,
            ),
            (
                format!("BitBake {bitbake} · v{bitbake_version} · Jobs {active_jobs}"),
                bitbake_tone,
            ),
            (
                format!("Compat {compatibility} · PTY {sessions} · Clients {clients}{pressure}"),
                compatibility_tone,
            ),
            (host, host_tone),
        ]
    } else {
        vec![
            (
                format!("Daemon {daemon} · {access} · version unavailable"),
                daemon_tone,
            ),
            (
                format!("Up {uptime} · BitBake {bitbake} · v{bitbake_version}"),
                bitbake_tone,
            ),
            (
                format!(
                    "Compat {compatibility} · Jobs {active_jobs} · PTY {sessions} · Clients {clients}{pressure}"
                ),
                compatibility_tone,
            ),
            (host, host_tone),
        ]
    };
    lines
        .into_iter()
        .map(|(text, tone)| SystemStatusLine {
            text: bounded_status_line(format!("{} {text}", tone.marker()), width),
            tone,
        })
        .collect()
}

pub(crate) fn system_status_text(app: &App, width: u16) -> String {
    system_status_projection(app, width)
        .into_iter()
        .map(|line| line.text)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn system_status_document(app: &App, width: u16) -> Text<'static> {
    let palette = ThemePalette::for_app(app);
    Text::from(
        system_status_projection(app, width)
            .into_iter()
            .map(|line| Line::styled(line.text, status_tone_style(&palette, line.tone)))
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn push_inspector_section(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    title: &'static str,
    body: &str,
) {
    if body.trim().is_empty() {
        return;
    }
    if !lines.is_empty() {
        lines.push(Line::default());
    }
    let palette = ThemePalette::for_app(app);
    lines.push(Line::styled(
        format!("▾ {title}"),
        palette.role(palette.heading, Modifier::BOLD),
    ));
    lines.extend(body.lines().map(|line| Line::from(line.to_owned())));
}

pub(crate) struct InspectorDocumentSections<'a> {
    pub(crate) primary: &'a str,
    pub(crate) secondary: Option<&'a str>,
    pub(crate) related_paths: &'a [String],
    pub(crate) recent_output: Option<&'a str>,
    pub(crate) actions: Option<&'a [ActionListItem]>,
    pub(crate) status: Option<&'a str>,
}

pub(crate) fn inspector_document(
    app: &App,
    sections: InspectorDocumentSections<'_>,
    width: u16,
) -> Text<'static> {
    let mut lines = Vec::new();
    push_inspector_section(&mut lines, app, "PRIMARY FACTS", sections.primary);
    if let Some(secondary) = sections.secondary {
        push_inspector_section(&mut lines, app, "SECONDARY FACTS", secondary);
    }
    if !sections.related_paths.is_empty() {
        push_inspector_section(
            &mut lines,
            app,
            "RELATED PATHS",
            &sections.related_paths.join("\n"),
        );
    }
    if let Some(output) = sections.recent_output {
        push_inspector_section(&mut lines, app, "RECENT OUTPUT", output);
    }
    if let Some(actions) = sections.actions.filter(|actions| !actions.is_empty()) {
        if !lines.is_empty() {
            lines.push(Line::default());
        }
        let palette = ThemePalette::for_app(app);
        lines.push(Line::styled(
            "▾ CONTEXTUAL ACTIONS",
            palette.role(palette.heading, Modifier::BOLD),
        ));
        lines.extend(action_list(actions, width, inspector_action_styles(app)).lines);
    }
    if let Some(status) = sections.status {
        push_inspector_section(&mut lines, app, "SYSTEM / COMPATIBILITY", status);
    }
    Text::from(lines)
}

pub(crate) fn inspector_related_paths(app: &App) -> Vec<String> {
    let path = match app.screen {
        Screen::Recipes => app
            .workspace
            .recipes
            .get(app.recipe_selection)
            .and_then(|recipe| recipe.file.clone()),
        Screen::Layers => app
            .layer_browser
            .as_ref()
            .and_then(|browser| {
                browser.selected_entry().map(|entry| {
                    if entry.path.is_absolute() {
                        entry.path.clone()
                    } else {
                        browser.root.join(&entry.path)
                    }
                })
            })
            .or_else(|| {
                app.workspace
                    .layers
                    .get(app.layer_selection)
                    .map(|layer| layer.path.clone())
            }),
        Screen::Logs if app.log_workspace_view == LogWorkspaceView::BitBake => {
            app.logs.selected().and_then(|entry| entry.path.clone())
        }
        Screen::Errors => app
            .logs
            .diagnostics()
            .nth(app.error_selection)
            .and_then(|entry| entry.path.clone()),
        Screen::Images => app
            .selected_image_artifact()
            .map(|artifact| artifact.identity.path.clone()),
        Screen::Kernel => app.kernel.selected_file().map(|file| file.path.clone()),
        Screen::Firmware => app.firmware.selected_file().map(|file| file.path.clone()),
        Screen::Sdk => app
            .selected_sdk_artifact()
            .map(|artifact| artifact.identity.path.clone()),
        Screen::BuildHistory => app
            .job_history_rows()
            .get(app.build_history_selection)
            .and_then(|row| match row {
                JobHistoryRowRef::Daemon(_) => None,
                JobHistoryRowRef::Background(job) => job.context.path.clone(),
                JobHistoryRowRef::Build(_) => None,
            }),
        _ => None,
    };
    path.into_iter()
        .map(|path| path.display().to_string())
        .collect()
}
