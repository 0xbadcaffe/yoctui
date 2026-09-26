//! Header.
use super::*;

pub(crate) fn daemon_status_label(status: yoctui_model::ClientReplicaStatus) -> &'static str {
    match status {
        yoctui_model::ClientReplicaStatus::Disconnected => "Disconnected",
        yoctui_model::ClientReplicaStatus::Synchronizing => "Syncing",
        yoctui_model::ClientReplicaStatus::Current => "Connected",
        yoctui_model::ClientReplicaStatus::Stale => "Stale",
    }
}

pub(crate) fn daemon_lifecycle_label(
    lifecycle: yoctui_model::ClientDaemonLifecycle,
) -> &'static str {
    match lifecycle {
        yoctui_model::ClientDaemonLifecycle::Disconnected => "Disconnected",
        yoctui_model::ClientDaemonLifecycle::Connecting => "Connecting",
        yoctui_model::ClientDaemonLifecycle::Running => "Running",
        yoctui_model::ClientDaemonLifecycle::Stopping => "Stopping",
        yoctui_model::ClientDaemonLifecycle::Exited => "Exited",
        yoctui_model::ClientDaemonLifecycle::Failed => "Failed",
        yoctui_model::ClientDaemonLifecycle::Lost => "Lost",
    }
}

pub(crate) fn daemon_status_tone(status: yoctui_model::ClientReplicaStatus) -> StatusTone {
    match status {
        yoctui_model::ClientReplicaStatus::Disconnected => StatusTone::Error,
        yoctui_model::ClientReplicaStatus::Synchronizing => StatusTone::Pending,
        yoctui_model::ClientReplicaStatus::Current => StatusTone::Success,
        yoctui_model::ClientReplicaStatus::Stale => StatusTone::Warning,
    }
}

pub(crate) fn daemon_lifecycle_tone(lifecycle: yoctui_model::ClientDaemonLifecycle) -> StatusTone {
    match lifecycle {
        yoctui_model::ClientDaemonLifecycle::Running => StatusTone::Success,
        yoctui_model::ClientDaemonLifecycle::Connecting
        | yoctui_model::ClientDaemonLifecycle::Stopping => StatusTone::Pending,
        yoctui_model::ClientDaemonLifecycle::Failed | yoctui_model::ClientDaemonLifecycle::Lost => {
            StatusTone::Error
        }
        yoctui_model::ClientDaemonLifecycle::Disconnected
        | yoctui_model::ClientDaemonLifecycle::Exited => StatusTone::Disabled,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HeaderMode {
    Full,
    Wide,
    Medium,
    Narrow,
}

pub(crate) fn header_mode(width: u16) -> HeaderMode {
    match width {
        180.. => HeaderMode::Full,
        130..=179 => HeaderMode::Wide,
        100..=129 => HeaderMode::Medium,
        _ => HeaderMode::Narrow,
    }
}

pub(crate) fn header_project_identity(app: &App) -> String {
    app.workspace
        .source_dir
        .as_deref()
        .or(app.workspace.build_dir.as_deref())
        .map(|path| {
            path.file_name().map_or_else(
                || path.display().to_string(),
                |name| name.to_string_lossy().into(),
            )
        })
        .unwrap_or_else(|| "unavailable".into())
}

pub(crate) fn build_status_tone(status: BuildStatus) -> StatusTone {
    match status {
        BuildStatus::Completed => StatusTone::Success,
        BuildStatus::Cancelled => StatusTone::Warning,
        BuildStatus::Failed => StatusTone::Error,
        BuildStatus::Lost => StatusTone::Warning,
        BuildStatus::LoadingWorkspace
        | BuildStatus::Parsing
        | BuildStatus::Running
        | BuildStatus::Cancelling => StatusTone::Running,
        BuildStatus::Idle => StatusTone::Disabled,
    }
}

pub(crate) fn header_separator(palette: &ThemePalette, compact: bool) -> Span<'static> {
    Span::styled(
        if compact { " • " } else { "  •  " },
        palette.role(palette.disabled, Modifier::DIM),
    )
}

pub(crate) fn header_identity_spans(
    palette: &ThemePalette,
    label: &'static str,
    compact_label: &'static str,
    value: String,
    compact: bool,
) -> [Span<'static>; 2] {
    [
        Span::styled(
            if compact { compact_label } else { label },
            palette.role(palette.muted, Modifier::DIM),
        ),
        Span::styled(value, palette.role(palette.informational, Modifier::BOLD)),
    ]
}

pub(crate) fn workbench_header(frame: &mut Frame, app: &App, area: Rect, now: SystemTime) {
    let palette = ThemePalette::for_app(app);
    let concept_geometry = area.height >= 5;
    let block = Block::default()
        .borders(if concept_geometry {
            Borders::ALL
        } else {
            Borders::TOP | Borders::LEFT | Borders::RIGHT
        })
        .style(palette.base())
        .border_style(Style::default().fg(palette.inactive_border));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }

    let mode = header_mode(area.width);
    let compact = area.width < 150 || matches!(mode, HeaderMode::Medium | HeaderMode::Narrow);
    let project = bounded_status_line(header_project_identity(app), 20);
    let target = bounded_status_line(
        app.build
            .target
            .clone()
            .unwrap_or_else(|| "not selected".into()),
        if mode == HeaderMode::Full { 30 } else { 22 },
    );
    let machine = app.workspace.variables.get("MACHINE");
    let distro = app.workspace.variables.get("DISTRO");
    let release = app.workspace.release.as_deref();
    let build_tone = build_status_tone(app.build.status);
    let mut left = vec![Span::styled(
        format!("yoctui v{}", env!("CARGO_PKG_VERSION")),
        palette.role(palette.progress, Modifier::BOLD),
    )];
    if let Some(mut git) = app.source_git_status.label() {
        if matches!(
            app.source_git_status,
            yoctui_model::SourceGitStatus::Scanning
        ) {
            git = format!("{} {git}", task_activity(app, None));
        }
        left.push(header_separator(&palette, true));
        left.push(Span::styled(
            git,
            palette.role(palette.informational, Modifier::BOLD),
        ));
    }
    if mode != HeaderMode::Narrow {
        left.push(header_separator(&palette, compact));
        left.extend(header_identity_spans(
            &palette,
            "Project: ",
            "Project: ",
            project,
            compact,
        ));
    }
    if !concept_geometry {
        left.push(header_separator(&palette, compact));
        left.push(status_label(
            build_tone,
            if app.is_offline() {
                "Offline".into()
            } else {
                app.build.status.to_string()
            },
            status_tone_style(&palette, build_tone),
        ));
    }
    left.push(header_separator(&palette, compact));
    left.extend(header_identity_spans(
        &palette, "Target: ", "T:", target, compact,
    ));
    if matches!(mode, HeaderMode::Full | HeaderMode::Wide)
        && let Some(machine) = machine
    {
        left.push(header_separator(&palette, false));
        left.extend(header_identity_spans(
            &palette,
            "Machine: ",
            "M:",
            machine.clone(),
            false,
        ));
    }
    if mode == HeaderMode::Full || concept_geometry {
        if let Some(distro) = distro {
            left.push(header_separator(&palette, false));
            left.extend(header_identity_spans(
                &palette,
                "Distro: ",
                "D:",
                distro.clone(),
                false,
            ));
        }
        if let Some(release) = release.filter(|_| !concept_geometry) {
            left.push(Span::raw(format!(" ({release})")));
        }
    }

    let daemon_tone = daemon_status_tone(app.daemon.status);
    let (bitbake_label, bitbake_tone) =
        if app.daemon.status == yoctui_model::ClientReplicaStatus::Current {
            (
                daemon_lifecycle_label(app.daemon.bitbake),
                daemon_lifecycle_tone(app.daemon.bitbake),
            )
        } else {
            ("Unavailable", StatusTone::Disabled)
        };
    let mut right = vec![Span::raw(if compact { "D:" } else { "Daemon: " })];
    right.push(status_label(
        daemon_tone,
        daemon_status_label(app.daemon.status),
        status_tone_style(&palette, daemon_tone),
    ));
    right.push(Span::styled(
        format!("/{}", app.client_access_origin.label()),
        palette.role(palette.informational, Modifier::BOLD),
    ));
    if mode != HeaderMode::Narrow {
        right.push(header_separator(&palette, compact));
        right.push(Span::raw(if compact { "BB:" } else { "BitBake: " }));
        right.push(status_label(
            bitbake_tone,
            bitbake_label,
            status_tone_style(&palette, bitbake_tone),
        ));
    }
    right.push(header_separator(&palette, true));
    right.push(Span::styled(
        if mode == HeaderMode::Narrow {
            local_clock_text(now)
        } else {
            clock_label(now)
        },
        palette.role(palette.primary_foreground, Modifier::BOLD),
    ));
    let right = Line::from(right);
    let right_width = u16::try_from(right.width())
        .unwrap_or(inner.width)
        .min(inner.width.saturating_sub(12));
    // Compact separators before clipping identity values when the release label
    // or backend context grows. Width tiers alone do not guarantee the text fits.
    let left_width = left.iter().map(Span::width).sum::<usize>();
    if left_width + usize::from(right_width) > usize::from(inner.width) {
        for span in &mut left {
            if span.content == "  •  " {
                span.content = " • ".into();
            }
        }
    }
    if !concept_geometry {
        let columns =
            Layout::horizontal([Constraint::Min(12), Constraint::Length(right_width)]).split(inner);
        frame.render_widget(Paragraph::new(Line::from(left)), columns[0]);
        frame.render_widget(
            Paragraph::new(right).alignment(Alignment::Right),
            columns[1],
        );
        return;
    }
    let header_rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);
    let primary_columns =
        Layout::horizontal([Constraint::Min(12), Constraint::Length(right_width)])
            .split(header_rows[0]);
    frame.render_widget(Paragraph::new(Line::from(left)), primary_columns[0]);
    frame.render_widget(
        Paragraph::new(right).alignment(Alignment::Right),
        primary_columns[1],
    );

    render_header_status(frame, app, header_rows[1], &palette);
    let workspace = app
        .workspace
        .source_dir
        .as_deref()
        .or(app.workspace.build_dir.as_deref())
        .map_or_else(|| "unavailable".into(), |path| path.display().to_string());
    let total = app
        .build
        .total
        .map_or_else(|| "—".into(), |total| total.to_string());
    let elapsed = app
        .build_summary_at(now)
        .elapsed
        .map_or_else(|| "--:--:--".into(), format_duration);
    let task = format!("{} / {total}", app.build.completed);
    let workers = app
        .active_worker_count()
        .map_or_else(|| "unavailable".into(), |count| count.to_string());
    let context = format!(
        "Workspace: {workspace}  |  Build: {}  |  Task: {task}  |  Elapsed: {elapsed}  |  Workers: {workers}",
        app.build.target.as_deref().unwrap_or("none")
    );
    frame.render_widget(
        Paragraph::new(bounded_cell_text(&context, header_rows[2].width)),
        header_rows[2],
    );
}

pub(crate) fn clock_text(now: SystemTime) -> String {
    let seconds = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
        % 86_400;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3_600,
        seconds / 60 % 60,
        seconds % 60
    )
}

pub(crate) fn clock_label(now: SystemTime) -> String {
    format!("Local {}", local_clock_text(now))
}

pub(crate) fn local_clock_text(now: SystemTime) -> String {
    let seconds = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let timestamp = libc::time_t::try_from(seconds).unwrap_or(libc::time_t::MAX);
    // SAFETY: `libc::tm` is a C data struct that permits all-zero initialization.
    let mut local = unsafe { std::mem::zeroed::<libc::tm>() };
    // SAFETY: both pointers are valid for the duration of this call.
    let result = unsafe { libc::localtime_r(&timestamp, &mut local) };
    if result.is_null() {
        return "--:--".into();
    }
    format!("{:02}:{:02}", local.tm_hour, local.tm_min)
}

fn render_header_status(frame: &mut Frame, app: &App, area: Rect, palette: &ThemePalette) {
    if area.is_empty() {
        return;
    }
    if let Some(status) = app.transient_status() {
        let tone = transient_status_tone(status.kind);
        let style = status_tone_style(palette, tone).add_modifier(Modifier::BOLD);
        let text = bounded_status_line(
            status.text.split_whitespace().collect::<Vec<_>>().join(" "),
            area.width.saturating_sub(2),
        );
        let spans = match status.kind {
            TransientStatusKind::Activity => vec![Span::styled(
                format!("{} {text}", task_activity(app, None)),
                style,
            )],
            TransientStatusKind::Notification => {
                vec![Span::styled(text, style)]
            }
            _ => vec![status_label(tone, text, style)],
        };
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
        return;
    }

    let daemon_tone = daemon_status_tone(app.daemon.status);
    let (bitbake, bitbake_tone) = if app.daemon.status == yoctui_model::ClientReplicaStatus::Current
    {
        (
            daemon_lifecycle_label(app.daemon.bitbake),
            daemon_lifecycle_tone(app.daemon.bitbake),
        )
    } else {
        ("Unavailable", StatusTone::Disabled)
    };
    let spans = vec![
        Span::styled(
            "Daemon health: ",
            palette.role(palette.heading, Modifier::BOLD),
        ),
        status_label(
            daemon_tone,
            daemon_status_label(app.daemon.status),
            status_tone_style(palette, daemon_tone).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("/{} · BitBake: ", app.client_access_origin.label()),
            palette.role(palette.informational, Modifier::BOLD),
        ),
        status_label(
            bitbake_tone,
            bitbake,
            status_tone_style(palette, bitbake_tone).add_modifier(Modifier::BOLD),
        ),
    ];
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

pub(crate) fn shortcut_rail<'a>(app: &App, shortcuts: &'a str) -> Line<'a> {
    let palette = ThemePalette::for_app(app);
    let mut spans = Vec::new();
    for (index, item) in shortcuts.split(" | ").enumerate() {
        if index > 0 {
            spans.push(Span::styled(
                "   ",
                palette.role(palette.disabled, Modifier::DIM),
            ));
        }
        let (key, action) = item.split_once(' ').unwrap_or((item, ""));
        if key == "…" {
            spans.push(Span::styled(
                key,
                palette.role(palette.muted, Modifier::DIM),
            ));
            continue;
        }
        spans.push(Span::styled(
            key,
            palette.role(palette.warning, Modifier::BOLD),
        ));
        if !action.is_empty() {
            spans.push(Span::raw(format!(" {action}")));
        }
    }
    Line::from(spans)
}

pub(crate) fn workbench_footer(frame: &mut Frame, app: &App, area: Rect, _now: SystemTime) {
    let palette = ThemePalette::for_app(app);
    let block = Block::default()
        .borders(if area.height >= 3 {
            Borders::ALL
        } else if area.width == LITERAL_REFERENCE_WIDTH && app.screen == Screen::Tasks {
            Borders::LEFT | Borders::RIGHT | Borders::BOTTOM
        } else {
            Borders::BOTTOM
        })
        .style(palette.base())
        .border_style(Style::default().fg(palette.inactive_border));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let shortcuts = if app.preferences.footer_shortcuts {
        footer_rail_shortcuts(app, inner.width)
    } else {
        "F1 Help | shortcuts hidden".into()
    };
    frame.render_widget(
        Paragraph::new(shortcut_rail(app, &shortcuts)).style(palette.base()),
        inner,
    );
}

pub(crate) fn transient_status_tone(kind: TransientStatusKind) -> StatusTone {
    match kind {
        TransientStatusKind::Error => StatusTone::Error,
        TransientStatusKind::Confirmation | TransientStatusKind::Warning => StatusTone::Warning,
        TransientStatusKind::Success => StatusTone::Success,
        TransientStatusKind::Notification => StatusTone::Info,
        TransientStatusKind::Reconnecting => StatusTone::Pending,
        TransientStatusKind::Activity => StatusTone::Running,
    }
}
