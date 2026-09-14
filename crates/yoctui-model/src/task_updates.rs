//! Task updates.
use super::*;

pub(crate) fn prepare_build(app: &mut App, target: Option<String>) {
    app.build.status = BuildStatus::LoadingWorkspace;
    app.build.target = target;
    app.build.started = None;
    app.build.completed = 0;
    app.build.total = None;
    app.build.parse_current = None;
    app.build.parse_total = None;
    app.build.warnings = 0;
    app.build.errors = 0;
    app.build.exit_code = None;
    app.dialogs
        .retain(|dialog| !matches!(dialog, Dialog::BuildCompletion));
    app.tasks.clear();
    app.completed_tasks.clear();
    app.task_progress_scroll = 0;
    app.invalidate_task_projection();
}

pub(crate) fn mark_build_running_from_task_activity(app: &mut App) {
    if matches!(
        app.build.status,
        BuildStatus::LoadingWorkspace | BuildStatus::Parsing | BuildStatus::Running
    ) {
        app.build.status = BuildStatus::Running;
        app.build.parse_current = None;
        app.build.parse_total = None;
    }
}

pub(crate) fn clamp_task_selection(app: &mut App) {
    app.invalidate_task_projection();
    app.task_progress_scroll = app
        .task_progress_scroll
        .min(app.visible_task_rows().len().saturating_sub(1));
}

pub(crate) fn apply_task_event(app: &mut App, event: TaskEvent) {
    mark_build_running_from_task_activity(app);
    let observed_start = matches!(&event, TaskEvent::ObservedStarted(_));
    let observed_timing = match &event {
        TaskEvent::ObservedCompleted { timing, .. } => Some(*timing),
        _ => None,
    };
    match event {
        TaskEvent::Started(mut task) | TaskEvent::ObservedStarted(mut task) => {
            if let Some(stats) = task.stats {
                app.build.completed = app.build.completed.max(stats.completed);
                app.build.total = (stats.total > 0).then_some(stats.total);
            }
            task.state = TaskState::Active;
            if !observed_start {
                task.started.get_or_insert_with(SystemTime::now);
            }
            if app.tasks.contains_key(&task.id) || app.tasks.len() < MAX_ACTIVE_TASKS {
                app.tasks.insert(task.id.clone(), task);
            } else {
                app.task_active_overflow = app.task_active_overflow.saturating_add(1);
            }
        }
        TaskEvent::Queued(mut task) => {
            if let Some(stats) = task.stats {
                app.build.completed = app.build.completed.max(stats.completed);
                app.build.total = (stats.total > 0).then_some(stats.total);
            }
            task.state = TaskState::Queued;
            task.started = None;
            task.finished = None;
            task.pid = None;
            if app.tasks.contains_key(&task.id) || app.tasks.len() < MAX_ACTIVE_TASKS {
                app.tasks.insert(task.id.clone(), task);
            } else {
                app.task_active_overflow = app.task_active_overflow.saturating_add(1);
            }
        }
        TaskEvent::Progress { id, progress } => {
            if let Some(task) = app.tasks.get_mut(&id) {
                task.progress = progress.map(|value| value.min(100));
            }
        }
        TaskEvent::Completed { id, success } | TaskEvent::ObservedCompleted { id, success, .. } => {
            let mut task = if let Some(task) = app.tasks.remove(&id) {
                task
            } else {
                if app
                    .completed_tasks
                    .iter()
                    .any(|completed| completed.task.id == id)
                {
                    return;
                }
                let (recipe, task) =
                    id.0.rsplit_once(':')
                        .map_or((id.0.as_str(), "unknown"), |(recipe, task)| (recipe, task));
                TaskInfo {
                    id: id.clone(),
                    recipe: recipe.into(),
                    task: task.into(),
                    ..TaskInfo::default()
                }
            };
            task.progress = Some(100);
            task.state = if success {
                TaskState::Completed
            } else {
                TaskState::Failed
            };
            task.finished = if let Some(timing) = observed_timing {
                task.started = timing.started;
                timing.finished
            } else {
                Some(SystemTime::now())
            };
            app.completed_tasks
                .push_back(CompletedTask { task, success });
            if app.completed_tasks.len() > MAX_COMPLETED_TASKS {
                app.completed_tasks.pop_front();
            }
            app.build.completed += 1;
        }
    }
}

pub(crate) fn flush_task_progress(
    app: &mut App,
    pending: &mut Vec<(TaskId, Option<u8>)>,
    indices: &mut HashMap<TaskId, usize>,
) {
    for (id, progress) in pending.drain(..) {
        apply_task_event(app, TaskEvent::Progress { id, progress });
    }
    indices.clear();
}

pub(crate) fn apply_task_batch(app: &mut App, events: Vec<TaskEvent>) {
    let mut pending_progress = Vec::<(TaskId, Option<u8>)>::new();
    let mut progress_indices = HashMap::<TaskId, usize>::new();
    for event in events {
        match event {
            TaskEvent::Progress { id, progress } => {
                app.task_progress_events = app.task_progress_events.saturating_add(1);
                if let Some(index) = progress_indices.get(&id).copied() {
                    pending_progress[index].1 = progress;
                    app.task_progress_coalesced = app.task_progress_coalesced.saturating_add(1);
                } else {
                    progress_indices.insert(id.clone(), pending_progress.len());
                    pending_progress.push((id, progress));
                }
            }
            transition => {
                flush_task_progress(app, &mut pending_progress, &mut progress_indices);
                apply_task_event(app, transition);
            }
        }
    }
    flush_task_progress(app, &mut pending_progress, &mut progress_indices);
    clamp_task_selection(app);
}

pub(crate) fn archive_unfinished_tasks(
    app: &mut App,
    state: TaskState,
    cancellation: Option<&str>,
) {
    let finished = SystemTime::now();
    for (_, mut task) in app.tasks.drain() {
        task.state = state;
        task.finished = Some(finished);
        task.cancellation = cancellation.map(str::to_owned);
        app.completed_tasks.push_back(CompletedTask {
            task,
            success: false,
        });
    }
    while app.completed_tasks.len() > MAX_COMPLETED_TASKS {
        app.completed_tasks.pop_front();
    }
    clamp_task_selection(app);
}

pub(crate) fn insert_log_batch(app: &mut App, entries: impl IntoIterator<Item = LogEntry>) {
    let build = app.build.target.clone();
    let entries = entries
        .into_iter()
        .map(|entry| prepare_log_entry(app, entry, &build))
        .collect::<Vec<_>>();
    app.logs.insert_batch(entries);
    app.error_selection = app
        .error_selection
        .min(app.logs.diagnostics().count().saturating_sub(1));
}

pub(crate) fn insert_log_entry(app: &mut App, entry: LogEntry) {
    let build = app.build.target.clone();
    let entry = prepare_log_entry(app, entry, &build);
    app.logs.insert(entry);
    app.error_selection = app
        .error_selection
        .min(app.logs.diagnostics().count().saturating_sub(1));
}

pub(crate) fn prepare_log_entry(
    app: &mut App,
    mut entry: LogEntry,
    build: &Option<String>,
) -> LogEntry {
    match entry.severity {
        Severity::Warning => app.build.warnings += 1,
        Severity::Error => app.build.errors += 1,
        Severity::Trace | Severity::Info => {}
    }
    entry.build = entry.build.or_else(|| build.clone());
    entry.protected |= matches!(entry.severity, Severity::Warning | Severity::Error);
    entry
}

pub(crate) fn insert_system_log(app: &mut App, severity: Severity, message: String) {
    let build = app.build.target.clone();
    app.logs.insert(LogEntry {
        id: 0,
        severity,
        message,
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::now(),
        build,
        protected: true,
        diagnostic: None,
    });
    app.error_selection = app
        .error_selection
        .min(app.logs.diagnostics().count().saturating_sub(1));
    if app.logs.follow {
        app.logs.selection = app.logs.filtered().count().saturating_sub(1);
        app.logs.scroll_offset = 0;
    }
}

pub fn format_log_details(entry: &LogEntry) -> String {
    format!(
        "Severity: {:?}\nBuild: {}\nRecipe: {}\nTask: {}\nSource: {}\n\n{}",
        entry.severity,
        entry.build.as_deref().unwrap_or("unavailable"),
        entry.recipe.as_deref().unwrap_or("unavailable"),
        entry.task.as_deref().unwrap_or("unavailable"),
        entry
            .path
            .as_ref()
            .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
        entry.message,
    )
}
