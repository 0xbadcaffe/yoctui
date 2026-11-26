//! Header.
use super::*;

include!("header/fitting.rs");

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
        yoctui_model::ClientDaemonLifecycle::Cancelled => "Cancelled",
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
        | yoctui_model::ClientDaemonLifecycle::Exited
        | yoctui_model::ClientDaemonLifecycle::Cancelled => StatusTone::Disabled,
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
    let compact = mode != HeaderMode::Full;
    let version = Span::styled(
        format!("yoctui v{}", env!("CARGO_PKG_VERSION")),
        palette.role(palette.progress, Modifier::BOLD),
    );
    let daemon_tone = daemon_status_tone(app.daemon.status);
    let mut right = vec![
        Span::raw(if compact { "D:" } else { "Daemon: " }),
        status_label(
            daemon_tone,
            daemon_status_label(app.daemon.status),
            status_tone_style(&palette, daemon_tone),
        ),
        Span::styled(
            format!("/{}", app.client_access_origin.label()),
            palette.role(palette.informational, Modifier::BOLD),
        ),
    ];
    let fields = [
        ("Project: ", "P:", header_project_identity(app)),
        (
            "Machine: ",
            "M:",
            app.workspace
                .variables
                .get("MACHINE")
                .cloned()
                .unwrap_or_else(|| "--".into()),
        ),
        (
            "Target: ",
            "T:",
            app.build
                .target
                .clone()
                .unwrap_or_else(|| "not selected".into()),
        ),
    ];
    let wide_labels = matches!(mode, HeaderMode::Full | HeaderMode::Wide);
    let label_cost = fields
        .iter()
        .enumerate()
        .map(|(index, (full, short, _))| {
            3 + if wide_labels && (index < 2 || !compact) {
                full.len()
            } else {
                short.len()
            }
        })
        .sum::<usize>();
    let primary_width = version.width()
        + label_cost
        + fields
            .iter()
            .map(|(_, _, value)| Span::raw(value.as_str()).width())
            .sum::<usize>();
    let right_budget = usize::from(inner.width).saturating_sub(primary_width + 1);
    let clock = if mode == HeaderMode::Narrow {
        local_clock_text(now)
    } else {
        clock_label(now)
    };
    let daemon_width = right.iter().map(Span::width).sum::<usize>();
    if daemon_width + 3 + clock.len() <= right_budget || mode != HeaderMode::Narrow {
        right.push(header_separator(&palette, true));
        right.push(Span::raw(clock));
    }
    let git = app.source_git_status.label();
    let git_width = git
        .as_deref()
        .map_or(0, |label| 3 + Span::raw(label).width());
    let secondary_budget =
        if right.iter().map(Span::width).sum::<usize>() + git_width <= right_budget {
            right_budget - git_width
        } else {
            right_budget
        };
    let (bitbake, tone) = header_bitbake_status(app);
    let bitbake = status_label(tone, bitbake, status_tone_style(&palette, tone));
    let bitbake_prefix = if compact { "BB:" } else { "BitBake: " };
    if wide_labels
        && right.iter().map(Span::width).sum::<usize>() + 3 + bitbake_prefix.len() + bitbake.width()
            <= secondary_budget
    {
        right.push(header_separator(&palette, true));
        right.push(Span::raw(bitbake_prefix));
        right.push(bitbake);
    }
    let right = Line::from(right);
    let right_width = u16::try_from(right.width())
        .unwrap_or(u16::MAX)
        .min(inner.width.saturating_sub(12));
    let left_budget = usize::from(inner.width.saturating_sub(right_width + 1));
    let value_budget = left_budget.saturating_sub(version.width() + label_cost);
    let mut left = vec![version];
    let widths = header_value_widths(&fields, value_budget);
    for (index, ((full, short, value), width)) in fields.into_iter().zip(widths).enumerate() {
        left.push(header_separator(&palette, true));
        left.extend(header_identity_spans(
            &palette,
            full,
            short,
            bounded_cell_text(&value, width),
            !(wide_labels && (index < 2 || !compact)),
        ));
    }
    if mode == HeaderMode::Full
        && let Some(distro) = app.workspace.variables.get("DISTRO")
        && left.iter().map(Span::width).sum::<usize>() + 11 + Span::raw(distro.as_str()).width()
            <= left_budget
    {
        left.push(header_separator(&palette, true));
        left.extend(header_identity_spans(
            &palette,
            "Distro: ",
            "D:",
            distro.clone(),
            false,
        ));
    }
    if let Some(git) = git
        && left.iter().map(Span::width).sum::<usize>() + 3 + Line::from(git.as_str()).width()
            <= left_budget
    {
        left.push(header_separator(&palette, true));
        left.push(Span::styled(
            git,
            palette.role(palette.muted, Modifier::DIM),
        ));
    }
    let left = fit_header_spans(left, left_budget as u16);
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

#[cfg(test)]
pub(crate) fn local_clock_text(now: SystemTime) -> String {
    let seconds = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
        % 86_400;
    format!("{:02}:{:02}", seconds / 3_600, seconds / 60 % 60)
}

#[cfg(not(test))]
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
    let (bitbake, bitbake_tone) = header_bitbake_status(app);
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

include!("header/footer.rs");

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

fn header_bitbake_status(app: &App) -> (&'static str, StatusTone) {
    if app.daemon.status == yoctui_model::ClientReplicaStatus::Current {
        (
            daemon_lifecycle_label(app.daemon.bitbake),
            daemon_lifecycle_tone(app.daemon.bitbake),
        )
    } else {
        ("Unavailable", StatusTone::Disabled)
    }
}
