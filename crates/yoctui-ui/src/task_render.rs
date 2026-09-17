//! Task render.
use super::*;

pub(crate) fn task_state_label(state: TaskState) -> &'static str {
    match state {
        TaskState::Queued => "· Queued",
        TaskState::Waiting => "▫ Waiting",
        TaskState::Active => "▶ Running",
        TaskState::Completed => "✓ Succeeded",
        TaskState::Failed => "✕ Failed",
        TaskState::Cancelled => "■ Cancelled",
        TaskState::Lost => "? Lost",
    }
}

pub(crate) fn task_state_style(app: &App, state: TaskState) -> Style {
    let palette = ThemePalette::for_app(app);
    match state {
        TaskState::Active => palette.role(palette.running, Modifier::BOLD),
        TaskState::Completed => palette.role(palette.success, Modifier::BOLD),
        TaskState::Queued => palette.role(palette.pending, Modifier::DIM),
        TaskState::Waiting => palette.role(palette.pending, Modifier::BOLD),
        TaskState::Cancelled => palette.role(palette.warning, Modifier::BOLD),
        TaskState::Failed | TaskState::Lost => {
            palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED)
        }
    }
}

pub(crate) fn matching_task_logs_ref(
    app: &App,
    row: Option<&TaskRowRef<'_>>,
    limit: usize,
) -> Vec<Line<'static>> {
    let task = match row {
        Some(TaskRowRef::Task { task, .. }) => Some(*task),
        Some(TaskRowRef::WaitingSummary(_)) | None => None,
    };
    matching_task_logs_for_task(app, task, limit)
}

pub(crate) fn matching_task_logs_for_task(
    app: &App,
    task: Option<&yoctui_model::TaskInfo>,
    limit: usize,
) -> Vec<Line<'static>> {
    let Some(task) = task else {
        return vec![Line::from("No task-specific log is available.")];
    };
    let entries = app
        .logs
        .entries
        .iter()
        .filter(|entry| {
            entry.recipe.as_deref() == Some(task.recipe.as_str())
                && entry.task.as_deref() == Some(task.task.as_str())
        })
        .rev()
        .take(limit)
        .collect::<Vec<_>>();
    if entries.is_empty() {
        return vec![Line::from(
            "Waiting for typed log entries for the selected task.",
        )];
    }
    entries
        .into_iter()
        .rev()
        .map(|entry| {
            let marker = match entry.severity {
                Severity::Trace => "·",
                Severity::Info => "│",
                Severity::Warning => "!",
                Severity::Error => "✕",
            };
            Line::from(vec![
                Span::styled(format!("{marker} "), severity_style(app, entry.severity)),
                Span::styled(entry.message.clone(), severity_style(app, entry.severity)),
            ])
        })
        .collect()
}

pub(crate) fn task_filter_summary(app: &App) -> String {
    let duration = app
        .task_filters
        .minimum_duration
        .map_or_else(|| "all".into(), |value| format!("≥{}s", value.as_secs()));
    let mut filters = vec![format!("{:?}", app.task_filters.state)];
    if !app.task_filters.recipe.is_empty() {
        filters.push(format!("recipe={}", app.task_filters.recipe));
    }
    if !app.task_filters.task.is_empty() {
        filters.push(format!("task={}", app.task_filters.task));
    }
    if !app.task_filters.worker.is_empty() {
        filters.push(format!("worker={}", app.task_filters.worker));
    }
    if duration != "all" {
        filters.push(format!("duration={duration}"));
    }
    filters.join(" · ")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskTableColumn {
    Task,
    Recipe,
    State,
    Elapsed,
    Progress,
    Worker,
    Pid,
}

impl TaskTableColumn {
    pub(crate) const ALL: [Self; 7] = [
        Self::Task,
        Self::Recipe,
        Self::State,
        Self::Elapsed,
        Self::Progress,
        Self::Worker,
        Self::Pid,
    ];

    pub(crate) const fn header(self) -> &'static str {
        match self {
            Self::Task => "Task",
            Self::Recipe => "Recipe",
            Self::State => "Status",
            Self::Elapsed => "Time",
            Self::Progress => "Progress",
            Self::Worker => "Worker",
            Self::Pid => "PID",
        }
    }

    pub(crate) const fn constraint(self) -> Constraint {
        match self {
            Self::Task | Self::Recipe => Constraint::Fill(1),
            Self::State => Constraint::Length(13),
            Self::Elapsed => Constraint::Length(9),
            Self::Progress => Constraint::Length(24),
            Self::Worker => Constraint::Length(12),
            Self::Pid => Constraint::Length(8),
        }
    }
}

pub(crate) fn task_table_columns(area_width: u16, rows: &[TaskRowRef<'_>]) -> Vec<TaskTableColumn> {
    const DEFINITIONS: [ResponsiveColumn; 7] = [
        ResponsiveColumn {
            minimum_width: 14,
            priority: 0,
        },
        ResponsiveColumn {
            minimum_width: 18,
            priority: 1,
        },
        ResponsiveColumn {
            minimum_width: 14,
            priority: 0,
        },
        ResponsiveColumn {
            minimum_width: 10,
            priority: 2,
        },
        ResponsiveColumn {
            minimum_width: 25,
            priority: 0,
        },
        ResponsiveColumn {
            minimum_width: 13,
            priority: 3,
        },
        ResponsiveColumn {
            minimum_width: 9,
            priority: 4,
        },
    ];
    let has_worker = rows
        .iter()
        .any(|row| matches!(row, TaskRowRef::Task { task, .. } if task.worker.is_some()));
    let has_pid = rows
        .iter()
        .any(|row| matches!(row, TaskRowRef::Task { task, .. } if task.pid.is_some()));
    let mut visible = responsive_columns(area_width.saturating_sub(2), &DEFINITIONS);
    if area_width < 84 {
        for index in [1, 3, 5, 6] {
            visible[index] = false;
        }
    } else if area_width < 110 {
        visible[5] = false;
        visible[6] = false;
    }
    visible[5] &= has_worker;
    visible[6] &= has_pid;
    TaskTableColumn::ALL
        .into_iter()
        .zip(visible)
        .filter_map(|(column, visible)| visible.then_some(column))
        .collect()
}

pub(crate) fn task_table_cell(
    app: &App,
    row: &TaskRowRef<'_>,
    column: TaskTableColumn,
    now: SystemTime,
) -> Cell<'static> {
    match (row, column) {
        (TaskRowRef::WaitingSummary(count), TaskTableColumn::Task) => {
            Cell::from(format!("{count} waiting tasks"))
        }
        (TaskRowRef::WaitingSummary(_), TaskTableColumn::Recipe) => Cell::from("unavailable"),
        (TaskRowRef::WaitingSummary(_), TaskTableColumn::State) => Cell::from(Span::styled(
            task_state_label(TaskState::Waiting),
            task_state_style(app, TaskState::Waiting),
        )),
        (TaskRowRef::WaitingSummary(_), TaskTableColumn::Elapsed) => Cell::from("--"),
        (TaskRowRef::WaitingSummary(_), TaskTableColumn::Progress) => {
            Cell::from("metadata unavailable")
        }
        (TaskRowRef::WaitingSummary(_), TaskTableColumn::Worker | TaskTableColumn::Pid) => {
            Cell::from("--")
        }
        (TaskRowRef::Task { task, state }, TaskTableColumn::Task) => {
            Cell::from(if *state == TaskState::Active {
                format!("{} {}", task_activity(app, task.progress), task.task)
            } else {
                format!("  {}", task.task)
            })
        }
        (TaskRowRef::Task { task, .. }, TaskTableColumn::Recipe) => Cell::from(task.recipe.clone()),
        (TaskRowRef::Task { state, .. }, TaskTableColumn::State) => Cell::from(Span::styled(
            task_state_label(*state),
            task_state_style(app, *state),
        )),
        (TaskRowRef::Task { task, .. }, TaskTableColumn::Elapsed) => Cell::from(
            task.elapsed_at(now)
                .map(format_duration)
                .unwrap_or_else(|| "--".into()),
        ),
        (TaskRowRef::Task { task, state }, TaskTableColumn::Progress) => Cell::from(Span::styled(
            match (*state, task.progress) {
                (TaskState::Active, None) => {
                    format!("progress unknown {}", task_activity(app, None))
                }
                (_, Some(progress)) => task_progress_bar(app, progress),
                _ => "--".into(),
            },
            task_state_style(app, *state),
        )),
        (TaskRowRef::Task { task, .. }, TaskTableColumn::Worker) => {
            Cell::from(task.worker.clone().unwrap_or_else(|| "--".into()))
        }
        (TaskRowRef::Task { task, .. }, TaskTableColumn::Pid) => {
            Cell::from(task.pid.map_or_else(|| "--".into(), |pid| pid.to_string()))
        }
    }
}

pub(crate) fn task_table_row_style(app: &App, state: TaskState, selected: bool) -> Style {
    if selected {
        selected_style(app, true)
    } else if state == TaskState::Active {
        task_state_style(app, state).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

pub(crate) fn render_build_summary(frame: &mut Frame, app: &App, area: Rect, now: SystemTime) {
    if area.is_empty() {
        return;
    }
    let summary = app.build_summary_at(now);
    let progress = app.progress_hierarchy_at(now);
    let rows = if app.screen == Screen::Dashboard && area.height >= 4 {
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area)
    } else if area.height >= 2 {
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area)
    } else {
        Layout::vertical([Constraint::Length(1)]).split(area)
    };
    let palette = ThemePalette::for_app(app);
    if let Some(fraction) = progress.build.fraction {
        let percent = fraction.percent();
        let total = fraction.total;
        let label = format!(
            "Overall  {percent}%  {}/{}",
            fraction.current.min(total),
            total
        );
        if app.preferences.symbols == SymbolPreference::Ascii {
            frame.render_widget(
                Paragraph::new(format!("{label}  {}", task_progress_bar(app, percent)))
                    .style(build_status_style(app)),
                rows[0],
            );
        } else {
            frame.render_widget(
                ratatui::widgets::Gauge::default()
                    .gauge_style(build_status_style(app).bg(palette.inactive_border))
                    .ratio(f64::from(percent) / 100.0)
                    .use_unicode(true)
                    .label(label),
                rows[0],
            );
        }
    } else if matches!(app.build.status, BuildStatus::Idle)
        && summary.completed == 0
        && app.tasks.is_empty()
    {
        frame.render_widget(
            Paragraph::new("Overall  build not started · 0%")
                .style(palette.role(palette.pending, Modifier::BOLD)),
            rows[0],
        );
    } else {
        frame.render_widget(
            Paragraph::new(format!(
                "Overall  progress unknown {}  {}/—",
                task_activity(app, None),
                summary.completed
            ))
            .style(
                palette
                    .role(palette.running, Modifier::BOLD)
                    .add_modifier(Modifier::BOLD),
            ),
            rows[0],
        );
    }
    if let Some(row) = rows.get(1) {
        let elapsed = summary.elapsed.map_or_else(|| "--".into(), format_duration);
        let text = if area.width >= 84 {
            let mut text = format!(
                "Active {}  Waiting {}  Warnings {}  Errors {}  Elapsed {elapsed}",
                summary.active, summary.waiting, summary.warnings, summary.errors
            );
            if area.width >= 96 {
                text.push_str("  ");
                text.push_str(&build_pace_at(app, now));
            }
            text
        } else {
            format!(
                "A{} W{} !{} ✕{} {elapsed}",
                summary.active, summary.waiting, summary.warnings, summary.errors
            )
        };
        frame.render_widget(
            Paragraph::new(text).style(palette.role(palette.secondary_foreground, Modifier::DIM)),
            *row,
        );
    }
    if let Some(row) = rows.get(2) {
        let exit = app
            .build
            .exit_code
            .map_or_else(|| "none".into(), |code| code.to_string());
        frame.render_widget(
            Paragraph::new(format!(
                "Target: {}  Backend: {}  Status: {}  Exit code: {exit}",
                app.build.target.as_deref().unwrap_or("not selected"),
                app.backend,
                app.build.status,
            )),
            *row,
        );
    }
    if let Some(row) = rows.get(3) {
        let parse = app.build.parse_current.map_or_else(
            || "not parsing".into(),
            |current| {
                app.build
                    .parse_total
                    .map_or_else(|| current.to_string(), |total| format!("{current}/{total}"))
            },
        );
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
        let cpu = app
            .host_telemetry
            .cpu_utilization_percent
            .map_or_else(|| "unavailable".into(), |value| format!("{value}%"));
        let disk = app
            .host_telemetry
            .disk_available_bytes
            .map_or_else(|| "unavailable".into(), format_bytes);
        let mut facts = format!(
            "Parse progress: {parse}  Tasks: {}/{}  Warnings: {}  Errors: {}  Release: {}",
            app.build.completed,
            app.build
                .total
                .map_or_else(|| "—".into(), |value| value.to_string()),
            app.build.warnings,
            app.build.errors,
            app.workspace.release.as_deref().unwrap_or("unknown"),
        );
        if area.width >= 100 {
            facts.push_str(&format!(
                "  Machine: {machine}  Distro: {distro}  Host CPU: {cpu}  Build disk free: {disk}"
            ));
        }
        frame.render_widget(Paragraph::new(facts), *row);
    }
}

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
