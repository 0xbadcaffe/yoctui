#[allow(dead_code)]
pub(crate) fn dashboard_recent_work_line(row: JobHistoryRowRef<'_>, now: SystemTime) -> String {
    match row {
        JobHistoryRowRef::Daemon(job) => format!(
            "{} {} · daemon",
            daemon_job_status_label(job.lifecycle),
            job.label,
        ),
        JobHistoryRowRef::Background(job) => format!(
            "{} {} · {}",
            job_status_label(job.status),
            job.title,
            job_elapsed(row, now).map_or_else(|| "--".into(), format_duration),
        ),
        JobHistoryRowRef::Build(record) => format!(
            "{} Build {} · {} tasks · {}",
            if record.success { "✓" } else { "✕" },
            record.target.as_deref().unwrap_or("unknown target"),
            record.completed_tasks,
            record.elapsed.map_or_else(|| "--".into(), format_duration),
        ),
    }
}

pub(crate) fn command_center_context_line(row: JobHistoryRowRef<'_>) -> String {
    match row {
        JobHistoryRowRef::Daemon(job) => format!("Daemon · {}", job.label),
        JobHistoryRowRef::Build(record) => format!(
            "Build · {}",
            record.target.as_deref().unwrap_or("unknown target")
        ),
        JobHistoryRowRef::Background(job) => {
            let context = &job.context;
            let workspace = context.workspace.map_or("Work", |screen| {
                yoctui_model::workspace_screen_destination(screen).label()
            });
            let identity = context
                .recipe
                .as_deref()
                .map(|recipe| {
                    context
                        .task
                        .as_deref()
                        .map_or_else(|| recipe.to_owned(), |task| format!("{recipe}:{task}"))
                })
                .or_else(|| context.image.clone())
                .or_else(|| context.target.clone())
                .or_else(|| {
                    context.path.as_ref().map(|path| {
                        path.file_name().map_or_else(
                            || path.display().to_string(),
                            |name| name.to_string_lossy().into_owned(),
                        )
                    })
                })
                .unwrap_or_else(|| job.title.clone());
            format!("{workspace} · {identity}")
        }
    }
}

#[allow(dead_code)]
pub(crate) fn render_workbench_center(
    frame: &mut Frame,
    center: &CommandCenterProjection<'_>,
    area: Rect,
    now: SystemTime,
) {
    let block = Block::default()
        .title("Workbench Center · live source projections")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let width = inner.width.max(1);
    let dashboard = &center.dashboard;
    let action = &dashboard.next_action;
    let context = center.recent_contexts.first().copied().map_or_else(
        || "[F3] Context: none retained".into(),
        |row| format!("[F3] Context: {}", command_center_context_line(row)),
    );
    let active = center.active_jobs.first().map_or_else(
        || {
            format!(
                "[F2/F3] Active: tasks {} · jobs 0",
                dashboard.summary.active
            )
        },
        |_| {
            format!(
                "[F2/F3] Active: tasks {} · jobs {}",
                dashboard.summary.active,
                center.active_jobs.len(),
            )
        },
    );
    let failure = dashboard.failures.first().map_or_else(
        || "[e] Failure: none retained".into(),
        |entry| {
            format!(
                "[e] Failure: {}",
                entry.message.lines().next().unwrap_or("diagnostic")
            )
        },
    );
    let artifact = dashboard.artifacts.first().map_or_else(
        || "[F8] Artifact: none retained".into(),
        |artifact| {
            let name = artifact.path.file_name().map_or_else(
                || artifact.path.display().to_string(),
                |name| name.to_string_lossy().into_owned(),
            );
            format!("[F8] Artifact: {} · {name}", artifact.source.label())
        },
    );
    let favorite = center.favorite_commands.first().map_or_else(
        || "[f] Favorite: none".into(),
        |favorite| {
            format!(
                "[f] Favorite: {} · {}{}",
                favorite.favorite.name,
                raw_availability_label(favorite.projection.availability.state),
                if favorite.projection.stale {
                    " · STALE"
                } else {
                    ""
                }
            )
        },
    );
    let terminal = center.terminals.first().map_or_else(
        || "[t] Terminal: none".into(),
        |terminal| {
            format!(
                "[t] Terminal: {} · {} · {} viewer(s)",
                terminal.name,
                daemon_lifecycle_label(terminal.lifecycle),
                terminal.viewers
            )
        },
    );
    let recent = dashboard.recent_work.first().copied().map_or_else(
        || "[F3] Recent: none retained".into(),
        |row| format!("[F3] Recent: {}", dashboard_recent_work_line(row, now)),
    );
    let lines = [
        format!(
            "Next: {} [{}] · {}",
            action.label,
            action.shortcut,
            availability_state_word(action.state)
        ),
        context,
        active,
        failure,
        artifact,
        favorite,
        terminal,
        recent,
    ]
    .into_iter()
    .map(|line| Line::from(bounded_cell_text(&line, width)))
    .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

#[allow(dead_code)]
pub(crate) fn render_dashboard_compact(
    frame: &mut Frame,
    app: &App,
    center: &CommandCenterProjection<'_>,
    area: Rect,
    now: SystemTime,
) {
    let projection = &center.dashboard;
    let top_height = area.height.saturating_sub(7).clamp(9, 11);
    let rows = Layout::vertical([Constraint::Length(top_height), Constraint::Min(1)]).split(area);
    render_dashboard_build(frame, app, projection, rows[0], now);
    let action = &projection.next_action;
    let attention = projection.failures.first().map_or_else(
        || {
            if app.build.warnings > 0 || app.build.errors > 0 {
                format!(
                    "Attention: build reports {} warning(s), {} error(s); no diagnostic rows retained",
                    app.build.warnings, app.build.errors
                )
            } else {
                "Attention: none retained".into()
            }
        },
        |entry| {
            format!(
                "Attention: {}",
                entry.message.lines().next().unwrap_or("diagnostic")
            )
        },
    );
    let artifact = projection.artifacts.first().map_or_else(
        || "Artifact: none retained".into(),
        |artifact| format!("Artifact: {}", artifact.path.display()),
    );
    let context = center
        .recent_contexts
        .first()
        .copied()
        .map_or_else(|| "none retained".into(), command_center_context_line);
    let favorite = center
        .favorite_commands
        .first()
        .map_or("none", |favorite| favorite.favorite.name.as_str());
    let terminal = center
        .terminals
        .first()
        .map_or("none", |terminal| terminal.name.as_str());
    let block = Block::default()
        .title("Operational Command Center")
        .borders(Borders::ALL);
    let inner = block.inner(rows[1]);
    frame.render_widget(block, rows[1]);
    let lines = [
        format!(
            "Next: {} [{}] — {}",
            action.label,
            action.shortcut,
            availability_state_word(action.state),
        ),
        format!(
            "Active: {} task(s) · {} job(s) | {attention}",
            projection.summary.active,
            center.active_jobs.len(),
        ),
        format!("Context: {context}"),
        artifact,
        format!("Favorite: {favorite} [f]"),
        format!("Terminal: {terminal} [t]"),
    ]
    .into_iter()
    .map(|line| Line::from(bounded_cell_text(&line, inner.width.max(1))))
    .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), inner);
}

#[allow(dead_code)]
pub(crate) fn dashboard(frame: &mut Frame, app: &App, area: Rect, now: SystemTime) {
    let center = app.command_center_projection_at(now);
    let projection = &center.dashboard;
    let concept_geometry = area.width >= 64 && area.height >= 34;
    if concept_geometry {
        let rows = Layout::vertical([
            Constraint::Length(9),
            Constraint::Min(8),
            Constraint::Length(8),
            Constraint::Length(7),
        ])
        .split(area);
        render_dashboard_build(frame, app, projection, rows[0], now);
        render_dashboard_recent_builds(frame, app, rows[1], now);
        render_telemetry_strip(frame, app, rows[2]);
        render_dashboard_quick_actions(frame, app, rows[3]);
        return;
    }
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(4)]).split(area);
    render_dashboard_compact(frame, app, &center, rows[0], now);
    render_compact_telemetry_strip(frame, app, rows[1]);
}
