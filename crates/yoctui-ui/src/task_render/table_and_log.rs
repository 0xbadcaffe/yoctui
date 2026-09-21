pub(crate) fn render_task_table(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    rows: &[TaskRowRef<'_>],
    now: SystemTime,
) {
    let target = app.build.target.as_deref().unwrap_or("not selected");
    let title = if app.screen == Screen::Dashboard || (area.width == 89 && area.height == 17) {
        "Tasks: Build".into()
    } else {
        format!("Tasks: {target} · {}", task_filter_summary(app))
    };
    let block = pane_block(app, &title, app.focus == FocusTarget::Workspace);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let summary_height = if app.screen == Screen::Dashboard {
        inner.height.min(4)
    } else {
        inner.height.min(2)
    };
    let sections =
        Layout::vertical([Constraint::Length(summary_height), Constraint::Min(1)]).split(inner);
    render_build_summary(frame, app, sections[0], now);
    let table_area = sections[1];
    let visible_rows = usize::from(table_area.height.saturating_sub(1)).max(1);
    let selected = app.task_progress_scroll.min(rows.len().saturating_sub(1));
    let viewport_start = selected
        .saturating_sub(visible_rows / 2)
        .min(rows.len().saturating_sub(visible_rows));
    let columns = task_table_columns(area.width, rows);
    let table_rows = rows
        .iter()
        .enumerate()
        .skip(viewport_start)
        .take(visible_rows)
        .map(|(index, row)| {
            let state = match row {
                TaskRowRef::WaitingSummary(_) => TaskState::Waiting,
                TaskRowRef::Task { state, .. } => *state,
            };
            Row::new(
                columns
                    .iter()
                    .map(|column| task_table_cell(app, row, *column, now)),
            )
            .style(task_table_row_style(
                app,
                state,
                index == app.task_progress_scroll,
            ))
        });
    let constraints = columns
        .iter()
        .map(|column| column.constraint())
        .collect::<Vec<_>>();
    let headers = columns
        .iter()
        .map(|column| column.header())
        .collect::<Vec<_>>();
    frame.render_widget(
        Table::new(table_rows, constraints)
            .header(Row::new(headers).style(Style::default().add_modifier(Modifier::BOLD))),
        table_area,
    );
}

pub(crate) fn render_task_log(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    selected: Option<&TaskRowRef<'_>>,
) {
    let context = match selected {
        Some(TaskRowRef::Task { task, .. }) => {
            format!("Log Viewer — {} ({})", task.task, task.recipe)
        }
        Some(TaskRowRef::WaitingSummary(_)) => "Log Viewer — waiting tasks".into(),
        None => "Log Viewer — no task selected".into(),
    };
    let activity = compact_log_activity(app, area.width.saturating_sub(24));
    let title = format!("{context} · {activity}");
    let limit = usize::from(area.height.saturating_sub(2)).max(1);
    yocto_logs::render(
        frame,
        area,
        Text::from(matching_task_logs_ref(app, selected, limit)),
        Block::default().title(title).borders(Borders::ALL),
        true,
    );
}
