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
