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
