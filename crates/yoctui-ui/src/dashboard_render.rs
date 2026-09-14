//! Dashboard render.
use super::*;

#[allow(dead_code)]
pub(crate) fn dashboard_build_details(
    app: &App,
    projection: &DashboardProjection<'_>,
    width: u16,
) -> String {
    let parse_progress = app.build.parse_current.map_or_else(
        || "not parsing".into(),
        |current| {
            app.build
                .parse_total
                .map_or_else(|| current.to_string(), |total| format!("{current}/{total}"))
        },
    );
    let cpu = app
        .host_telemetry
        .cpu_utilization_percent
        .map_or_else(|| "unavailable".into(), |percent| format!("{percent}%"));
    let disk = if projection.health.build_filesystem_sample
        && utilization_percent(
            app.host_telemetry.disk_total_bytes,
            app.host_telemetry.disk_available_bytes,
        )
        .is_some()
    {
        app.host_telemetry
            .disk_available_bytes
            .map_or_else(|| "unavailable".into(), format_bytes)
    } else {
        "unavailable".into()
    };
    let total = app
        .build
        .total
        .map_or_else(|| "—".into(), |total| total.to_string());
    let exit = app
        .build
        .exit_code
        .map_or_else(|| "none".into(), |code| code.to_string());
    let machine = app
        .workspace
        .variables
        .get("MACHINE")
        .map_or("unknown", String::as_str);
    let distro = app
        .workspace
        .variables
        .get("DISTRO")
        .map_or("unknown", String::as_str);
    let sstate = &projection.progress.sstate;
    if width >= 70 {
        format!(
            "Target: {}  Backend: {}  Status: {}  Exit code: {exit}\nParse progress: {parse_progress}  Tasks: {}/{total} (active: {})  Warnings: {}  Errors: {}\nMachine: {machine}  Distro: {distro}  Release: {}\nHost CPU: {cpu}  Build disk free: {disk}  Environment: {}\nSstate reuse: {} — {}",
            app.build.target.as_deref().unwrap_or("none"),
            app.backend,
            app.build.status,
            app.build.completed,
            projection.summary.active,
            app.build.warnings,
            app.build.errors,
            app.workspace.release.as_deref().unwrap_or("unknown"),
            projection.health.environment.label(),
            sstate.state.label(),
            sstate.detail.as_deref().unwrap_or("no additional detail"),
        )
    } else {
        format!(
            "Target: {}  Status: {}  Exit code: {exit}\nBackend: {}  Parse progress: {parse_progress}\nTasks: {}/{total}  Active: {}  Warnings: {}  Errors: {}\nMachine: {machine}  Distro: {distro}  Release: {}\nCPU: {cpu}  Disk free: {disk}\nEnvironment: {}  Sstate reuse: {}",
            app.build.target.as_deref().unwrap_or("none"),
            app.build.status,
            app.backend,
            app.build.completed,
            projection.summary.active,
            app.build.warnings,
            app.build.errors,
            app.workspace.release.as_deref().unwrap_or("unknown"),
            projection.health.environment.label(),
            sstate.state.label(),
        )
    }
}

#[allow(dead_code)]
pub(crate) fn render_dashboard_build(
    frame: &mut Frame,
    app: &App,
    projection: &DashboardProjection<'_>,
    area: Rect,
    now: SystemTime,
) {
    let title = format!("Current Build · {}", app.build.status);
    let block = pane_block(app, &title, app.focus == FocusTarget::Workspace);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let summary_height = inner.height.min(2);
    let rows =
        Layout::vertical([Constraint::Length(summary_height), Constraint::Min(1)]).split(inner);
    render_build_summary(frame, app, rows[0], now);
    frame.render_widget(
        Paragraph::new(dashboard_build_details(app, projection, inner.width))
            .wrap(Wrap { trim: false }),
        rows[1],
    );
}

#[allow(dead_code)]
pub(crate) fn dashboard_task_rows(app: &App) -> Vec<(&yoctui_model::TaskInfo, Option<bool>)> {
    let mut active = app.tasks.values().collect::<Vec<_>>();
    active.sort_by(|left, right| {
        (left.recipe.as_str(), left.task.as_str())
            .cmp(&(right.recipe.as_str(), right.task.as_str()))
    });
    let mut rows = active
        .into_iter()
        .map(|task| (task, None))
        .collect::<Vec<_>>();
    rows.extend(
        app.completed_tasks
            .iter()
            .rev()
            .map(|completed| (&completed.task, Some(completed.success))),
    );
    rows
}

#[allow(dead_code)]
pub(crate) fn render_dashboard_tasks(frame: &mut Frame, app: &App, area: Rect) {
    let tasks = dashboard_task_rows(app);
    let block = Block::default()
        .title(format!(
            "Active Tasks · {} active · {} retained complete · F2 details",
            app.tasks.len(),
            app.completed_tasks.len()
        ))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    if tasks.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No active task events. Start a build with B or inspect recent work with F3.",
            ),
            inner,
        );
        return;
    }
    let start = app.task_progress_scroll.min(tasks.len().saturating_sub(1));
    let rows = Layout::vertical(
        tasks[start..]
            .iter()
            .take(usize::from(inner.height))
            .map(|_| Constraint::Length(1))
            .collect::<Vec<_>>(),
    )
    .split(inner);
    let palette = ThemePalette::for_app(app);
    for ((task, completed), row) in tasks[start..]
        .iter()
        .take(rows.len())
        .zip(rows.iter().copied())
    {
        let progress = if completed.is_some() {
            100
        } else {
            task.progress.unwrap_or(0).min(100)
        };
        let style = if *completed == Some(false) {
            palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED)
        } else if progress >= 100 {
            palette.role(palette.success, Modifier::BOLD)
        } else if progress >= 75 {
            palette.role(palette.warning, Modifier::BOLD)
        } else {
            palette.role(palette.progress, Modifier::BOLD)
        };
        let label = match completed {
            Some(success) => format!(
                " {} {}:{} · 100% {}",
                if *success { "✓" } else { "×" },
                task.recipe,
                task.task,
                if *success { "complete" } else { "failed" }
            ),
            None if task.progress.is_some() => format!(
                " {} {}:{} · {}",
                "›",
                task.recipe,
                task.task,
                task_progress_bar(progress)
            ),
            None => format!(
                " {} {}:{} · active",
                task_activity(app, None),
                task.recipe,
                task.task,
            ),
        };
        frame.render_widget(Paragraph::new(label).style(style), row);
    }
}

pub(crate) fn availability_state_word(state: WorkspaceAvailabilityState) -> &'static str {
    match state {
        WorkspaceAvailabilityState::Available => "available",
        WorkspaceAvailabilityState::AvailableWithLimitations => "limited",
        WorkspaceAvailabilityState::Unavailable => "unavailable",
        WorkspaceAvailabilityState::Unsupported => "unsupported",
        WorkspaceAvailabilityState::Unknown => "unknown",
    }
}

#[allow(dead_code)]
pub(crate) fn render_dashboard_actions(
    frame: &mut Frame,
    app: &App,
    projection: &DashboardProjection<'_>,
    area: Rect,
) {
    let action = &projection.next_action;
    let palette = ThemePalette::for_app(app);
    let tone = if action.enabled {
        palette.role(palette.running, Modifier::BOLD)
    } else {
        palette.role(palette.warning, Modifier::BOLD)
    };
    let mut lines = vec![
        Line::styled("Recommended", palette.role(palette.heading, Modifier::BOLD)),
        Line::styled(
            format!(
                "{} [{}] — {}",
                action.label,
                action.shortcut,
                availability_state_word(action.state)
            ),
            tone,
        ),
    ];
    if let Some(reason) = action.reason.as_deref() {
        lines.push(Line::styled(
            format!("Reason: {reason}"),
            palette.role(palette.warning, Modifier::DIM),
        ));
    }
    lines.extend([
        Line::default(),
        Line::styled(
            "Common actions",
            palette.role(palette.heading, Modifier::BOLD),
        ),
        Line::from("[F2] Tasks  [l] Logs  [e] Errors  [F3] Recent work"),
        Line::from("[F8] Artifacts  [f] Favorites  [t] Terminals"),
        Line::from("[E] Environment  [M] Sstate readiness"),
    ]);
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().title("Next Action").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        area,
    );
}

#[allow(dead_code)]
pub(crate) fn render_dashboard_attention(
    frame: &mut Frame,
    app: &App,
    projection: &DashboardProjection<'_>,
    area: Rect,
) {
    let palette = ThemePalette::for_app(app);
    let lines = if projection.failures.is_empty()
        && (app.build.warnings > 0 || app.build.errors > 0)
    {
        vec![Line::styled(
            format!(
                "! Build reports {} warning(s), {} error(s); no retained diagnostic rows. [e] opens diagnostics.",
                app.build.warnings, app.build.errors
            ),
            palette.role(palette.warning, Modifier::BOLD),
        )]
    } else if projection.failures.is_empty() {
        vec![Line::styled(
            "✓ No retained warnings or errors. [e] opens diagnostics.",
            palette.role(palette.success, Modifier::BOLD),
        )]
    } else {
        projection
            .failures
            .iter()
            .map(|entry| {
                let marker = if entry.severity == Severity::Error {
                    "✕"
                } else {
                    "!"
                };
                Line::styled(
                    format!(
                        "{marker} {}",
                        entry.message.lines().next().unwrap_or("diagnostic")
                    ),
                    severity_style(app, entry.severity),
                )
            })
            .collect()
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(format!(
                        "Attention · {} retained · e review",
                        projection.failures.len()
                    ))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

#[allow(dead_code)]
pub(crate) fn render_dashboard_recent_builds(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let block = pane_block(app, "Recent Builds", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let rows = app
        .build_history
        .iter()
        .rev()
        .take(usize::from(inner.height.saturating_sub(1)))
        .enumerate()
        .map(|(index, build)| {
            let result = if build.success {
                "✓ Succeeded"
            } else {
                "✕ Failed"
            };
            let style = if build.success {
                palette.role(palette.success, Modifier::BOLD)
            } else {
                palette.role(palette.error, Modifier::BOLD)
            };
            Row::new([
                (index + 1).to_string(),
                build.target.as_deref().unwrap_or("unknown").to_owned(),
                result.to_owned(),
                build.completed_tasks.to_string(),
                build.elapsed.map_or_else(|| "--".into(), format_duration),
            ])
            .style(style)
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        frame.render_widget(
            Table::new(
                [Row::new([
                    "–",
                    "No completed builds retained",
                    "–",
                    "–",
                    "Press B to build",
                ])],
                [
                    Constraint::Length(4),
                    Constraint::Percentage(36),
                    Constraint::Length(13),
                    Constraint::Length(8),
                    Constraint::Min(8),
                ],
            )
            .header(
                Row::new(["#", "Image", "Result", "Tasks", "Duration"])
                    .style(palette.role(palette.table_header, Modifier::BOLD)),
            ),
            inner,
        );
        return;
    }
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(4),
                Constraint::Percentage(36),
                Constraint::Length(13),
                Constraint::Length(8),
                Constraint::Min(8),
            ],
        )
        .header(
            Row::new(["#", "Image", "Result", "Tasks", "Duration"])
                .style(palette.role(palette.table_header, Modifier::BOLD)),
        ),
        inner,
    );
}

#[allow(dead_code)]
pub(crate) fn render_dashboard_quick_actions(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let block = pane_block(app, "Quick Actions", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let columns = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(inner);
    let actions = [
        ("1", "Build image", "Start the selected BitBake target"),
        ("2", "Open terminal", "Use the initialized Yocto shell"),
        ("3", "Verify environment", "Check layers and configuration"),
    ];
    for (index, ((key, label, detail), column)) in
        actions.into_iter().zip(columns.iter().copied()).enumerate()
    {
        let borders = if index + 1 < columns.len() {
            Borders::RIGHT
        } else {
            Borders::NONE
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("[{key}] "),
                        palette.role(palette.informational, Modifier::BOLD),
                    ),
                    Span::styled(label, palette.role(palette.heading, Modifier::BOLD)),
                ]),
                Line::default(),
                Line::from(detail),
            ])
            .block(Block::default().borders(borders))
            .wrap(Wrap { trim: false }),
            column,
        );
    }
}

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
    let concept_geometry = area.width == 86 && area.height == 42;
    if concept_geometry {
        let rows = Layout::vertical([
            Constraint::Length(9),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Length(9),
        ])
        .split(area);
        render_dashboard_build(frame, app, projection, rows[0], now);
        render_dashboard_recent_builds(frame, app, rows[1]);
        render_telemetry_strip(frame, app, rows[2]);
        render_dashboard_quick_actions(frame, app, rows[3]);
        return;
    }
    let show_telemetry = area.height >= 38
        && telemetry_available(app)
        && telemetry_strip_mode(Rect::new(area.x, area.y, area.width, 8))
            != TelemetryStripMode::Hidden;
    let sections = if show_telemetry {
        Layout::vertical([Constraint::Min(1), Constraint::Length(8)]).split(area)
    } else {
        Layout::vertical([Constraint::Min(1)]).split(area)
    };
    let content = sections[0];
    if content.height < 24 || content.width < 60 {
        render_dashboard_compact(frame, app, &center, content, now);
    } else {
        let remaining = content.height.saturating_sub(10);
        let middle_height = remaining.div_ceil(2).max(7);
        let rows = Layout::vertical([
            Constraint::Length(10),
            Constraint::Length(middle_height),
            Constraint::Min(7),
        ])
        .split(content);
        render_dashboard_build(frame, app, projection, rows[0], now);
        let middle = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(rows[1]);
        render_dashboard_tasks(frame, app, middle[0]);
        render_dashboard_actions(frame, app, projection, middle[1]);
        let bottom = Layout::horizontal([Constraint::Percentage(47), Constraint::Percentage(53)])
            .split(rows[2]);
        render_dashboard_attention(frame, app, projection, bottom[0]);
        render_workbench_center(frame, &center, bottom[1], now);
    }
    if show_telemetry {
        render_telemetry_strip(frame, app, sections[1]);
    }
}
