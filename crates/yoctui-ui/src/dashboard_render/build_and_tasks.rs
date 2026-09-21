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
    let palette = ThemePalette::for_app(app);
    let block = pane_block(app, "Build Overview", app.focus == FocusTarget::Workspace);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let rows = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(inner);
    render_build_summary(frame, app, rows[0], now);
    if rows[1].width < 64 || area.height > 9 {
        frame.render_widget(
            Paragraph::new(dashboard_build_details(app, projection, rows[1].width)),
            rows[1],
        );
        return;
    }
    let columns =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[1]);
    let selected = app
        .tasks
        .values()
        .min_by_key(|task| (&task.recipe, &task.task));
    let task = selected.map_or_else(
        || "none".into(),
        |task| format!("{}:{}", task.recipe, task.task),
    );
    let current = vec![
        Line::from(format!(
            "Current Build : {}",
            app.build.target.as_deref().unwrap_or("none")
        )),
        Line::from(format!("Current Task  : {task}")),
        Line::from(format!(
            "Build Status  : {}",
            if app.is_offline() {
                "unavailable (offline)".to_owned()
            } else {
                app.build.status.to_string()
            }
        )),
        Line::from(format!(
            "Daemon Status : {}",
            header::daemon_status_label(app.daemon.status)
        )),
        Line::from(format!(
            "Warnings: {}  Errors: {}",
            app.build.warnings, app.build.errors
        )),
    ];
    frame.render_widget(
        Paragraph::new(current).block(
            Block::default()
                .borders(Borders::RIGHT)
                .border_style(palette.role(palette.inactive_border, Modifier::empty())),
        ),
        columns[0],
    );
    let recent = projection.recent_work.first().copied();
    let result = recent.map_or_else(
        || "No retained build or job".into(),
        |row| dashboard_recent_work_line(row, now),
    );
    let detail = recent.map_or_else(
        || "History appears after a build or job".into(),
        command_center_context_line,
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                "Last Build / Job",
                palette.role(palette.heading, Modifier::BOLD),
            ),
            Line::from(result),
            Line::from(detail),
        ])
        .wrap(Wrap { trim: false }),
        columns[1],
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
        let status = match completed {
            Some(true) => "complete".into(),
            Some(false) => "failed".into(),
            None if task.progress.is_some() => task_progress_bar(app, progress),
            None => format!("active {}", task_activity(app, None)),
        };
        let columns = Layout::horizontal([Constraint::Min(1), Constraint::Length(18)]).split(row);
        let identity = format!(" {}:{}", task.recipe, task.task);
        frame.render_widget(
            Paragraph::new(bounded_cell_text(&identity, columns[0].width)).style(style),
            columns[0],
        );
        frame.render_widget(Paragraph::new(status).style(style), columns[1]);
    }
}
