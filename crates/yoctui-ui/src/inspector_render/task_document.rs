pub(crate) fn task_inspector_primary(inspector: &TaskInspectorRef<'_>) -> String {
    match inspector {
        TaskInspectorRef::None => {
            "No task selected.\nTask facts appear as typed BitBake events arrive.".into()
        }
        TaskInspectorRef::Waiting { count } => format!(
            "Task        unavailable\nRecipe      unavailable\nPN          unavailable\nPV          unavailable\nPR          unavailable\nState       {}\nProgress    unavailable\n\n{count} waiting tasks are known only as an aggregate.",
            task_state_label(TaskState::Waiting)
        ),
        TaskInspectorRef::Task {
            task,
            state,
            version,
            revision,
            ..
        } => format!(
            "Task        {}\nRecipe      {}\nPN          {}\nPV          {}\nPR          {}\nState       {}\nProgress    {}",
            task.task,
            task.recipe,
            task.recipe,
            version.unwrap_or("unavailable"),
            revision.unwrap_or("unavailable"),
            task_state_label(*state),
            task.progress
                .map_or_else(|| "unknown".into(), |value| format!("{value}%")),
        ),
    }
}

pub(crate) fn task_inspector_context(inspector: &TaskInspectorRef<'_>, now: SystemTime) -> String {
    match inspector {
        TaskInspectorRef::None => "Secondary facts, paths, and dependencies unavailable.".into(),
        TaskInspectorRef::Waiting { .. } => {
            "Worker      unavailable\nPID         unavailable\nStarted     unavailable\nElapsed     unavailable\nWorkdir     unavailable\nLog file    unavailable\nDependencies unavailable".into()
        }
        TaskInspectorRef::Task {
            task,
            workdir,
            ..
        } => {
            let dependencies = if task.dependencies.is_empty() {
                "none reported".into()
            } else {
                task.dependencies
                    .iter()
                    .map(|dependency| dependency.0.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!(
                "Worker      {}\nPID         {}\nStarted     {}\nElapsed     {}\nWorkdir     {}\nLog file    {}\nDependencies {}",
                task.worker.as_deref().unwrap_or("unavailable"),
                task.pid
                    .map_or_else(|| "unavailable".into(), |pid| pid.to_string()),
                task.started
                    .map_or_else(|| "unavailable".into(), clock_text),
                task.elapsed_at(now)
                    .map_or_else(|| "unavailable".into(), format_duration),
                workdir.map_or_else(
                    || "unavailable".into(),
                    |path| path.display().to_string()
                ),
                task.log_path.as_ref().map_or_else(
                    || "unavailable".into(),
                    |path| path.display().to_string()
                ),
                dependencies,
            )
        }
    }
}

pub(crate) fn task_inspector_recent_lines<'a>(
    app: &App,
    inspector: &'a TaskInspectorRef<'a>,
) -> Vec<Line<'a>> {
    let TaskInspectorRef::Task { recent_logs, .. } = inspector else {
        return vec![Line::from("No task-specific log is available.")];
    };
    if recent_logs.is_empty() {
        return vec![Line::from(
            "Waiting for typed log entries for the selected task.",
        )];
    }
    recent_logs
        .iter()
        .map(|entry| {
            let marker = match entry.severity {
                Severity::Trace => "·",
                Severity::Info => "│",
                Severity::Warning => "!",
                Severity::Error => "✕",
            };
            Line::from(vec![
                Span::styled(format!("{marker} "), severity_style(app, entry.severity)),
                Span::styled(entry.message.as_str(), severity_style(app, entry.severity)),
            ])
        })
        .collect()
}

pub(crate) fn inspector_action_styles(app: &App) -> ActionListStyles {
    let palette = ThemePalette::for_app(app);
    ActionListStyles {
        enabled: palette.role(palette.primary_foreground, Modifier::empty()),
        disabled: palette.role(palette.disabled, Modifier::DIM),
        shortcut: palette.role(palette.accent, Modifier::BOLD),
        detail: palette.role(palette.secondary_foreground, Modifier::ITALIC),
    }
}

pub(crate) fn task_inspector_actions(app: &App) -> Vec<ActionListItem> {
    let active_build = matches!(
        app.build.status,
        BuildStatus::Running | BuildStatus::Parsing | BuildStatus::Cancelling
    );
    let mut actions = compatibility_workspace_actions(app, WorkspaceDestination::Tasks)
        .into_iter()
        .filter(|action| action.label != "Inspect task inventory")
        .map(|mut action| {
            if action.label == "Cancel active build" && !active_build {
                action.enabled = false;
                action.marker = "×";
                action.state = "Disabled".into();
                action
                    .details
                    .insert(0, "Reason: No active build can be cancelled.".into());
            }
            action
        })
        .collect::<Vec<_>>();
    actions.sort_by_key(|action| match action.label.as_str() {
        "Cancel active build" => 0,
        "Open Logs" => 1,
        "Build History" => 2,
        "Build options" => 3,
        _ => 4,
    });
    actions
}

pub(crate) fn bounded_status_line(value: String, width: u16) -> String {
    let width = usize::from(width);
    if value.chars().count() <= width {
        return value;
    }
    if width == 0 {
        return String::new();
    }
    value
        .chars()
        .take(width.saturating_sub(1))
        .chain(std::iter::once('…'))
        .collect()
}
