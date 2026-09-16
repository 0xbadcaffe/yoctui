//! Log export.
use super::*;

pub const MAX_LOG_COPY_BYTES: usize = 64 * 1024;
pub const MAX_LOG_EXPORT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogExport {
    pub content: String,
    pub included: usize,
    pub omitted: usize,
    pub truncated: bool,
}

pub fn format_log_details_bounded(entry: &LogEntry) -> String {
    let header = format!(
        "Severity: {:?}\nBuild: {}\nRecipe: {}\nTask: {}\nSource: {}\n\n",
        entry.severity,
        entry.build.as_deref().unwrap_or("unavailable"),
        entry.recipe.as_deref().unwrap_or("unavailable"),
        entry.task.as_deref().unwrap_or("unavailable"),
        entry
            .path
            .as_ref()
            .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
    );
    let marker = "\n[copy truncated at 64 KiB]";
    let content_limit = MAX_LOG_COPY_BYTES.saturating_sub(marker.len());
    let mut output =
        String::with_capacity(MAX_LOG_COPY_BYTES.min(header.len() + entry.message.len()));
    let complete = push_bounded(&mut output, &header, content_limit)
        && push_bounded(&mut output, &entry.message, content_limit);
    if !complete {
        append_truncation_marker(&mut output, marker, MAX_LOG_COPY_BYTES);
    }
    output
}

pub fn format_log_export(logs: &LogState) -> LogExport {
    let total = logs.visible_count();
    let mut content = String::with_capacity(MAX_LOG_EXPORT_BYTES.min(logs.retained_bytes));
    let header = format!(
        "Yoctui bounded log export\nVisible entries: {total}\nEvicted: {} [warnings {} errors {}]\nCoalesced: {}\n\n",
        logs.dropped, logs.dropped_warnings, logs.dropped_errors, logs.coalesced
    );
    let marker = "\n[export truncated at 256 KiB]";
    let content_limit = MAX_LOG_EXPORT_BYTES.saturating_sub(marker.len());
    let mut truncated = !push_bounded(&mut content, &header, content_limit);
    let mut included = 0;
    if !truncated {
        for entry in logs.filtered() {
            let entry_header = format!(
                "--- Log {} · {:?} ---\nBuild: {}\nRecipe: {}\nTask: {}\nSource: {}\n",
                entry.id,
                entry.severity,
                entry.build.as_deref().unwrap_or("unavailable"),
                entry.recipe.as_deref().unwrap_or("unavailable"),
                entry.task.as_deref().unwrap_or("unavailable"),
                entry
                    .path
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
            );
            if !push_bounded(&mut content, &entry_header, content_limit)
                || !push_bounded(&mut content, &entry.message, content_limit)
                || !push_bounded(&mut content, "\n\n", content_limit)
            {
                truncated = true;
                break;
            }
            included += 1;
        }
    }
    if truncated {
        append_truncation_marker(&mut content, marker, MAX_LOG_EXPORT_BYTES);
    }
    LogExport {
        content,
        included,
        omitted: total.saturating_sub(included),
        truncated,
    }
}

pub(crate) fn selected_correlated_log_id(app: &App) -> Option<u64> {
    let (build, recipe, task) = match app.screen {
        Screen::Tasks => match app
            .visible_task_row_refs_at(SystemTime::now())
            .get(app.task_progress_scroll)
            .copied()
        {
            Some(TaskRowRef::Task { task, .. }) => {
                (None, Some(task.recipe.as_str()), Some(task.task.as_str()))
            }
            Some(TaskRowRef::WaitingSummary(_)) | None => return None,
        },
        Screen::BuildHistory => match app
            .job_history_rows()
            .get(app.build_history_selection)
            .copied()
        {
            Some(JobHistoryRowRef::Daemon(_)) => (None, None, None),
            Some(JobHistoryRowRef::Background(job)) => (
                job.context.target.as_deref(),
                job.context.recipe.as_deref(),
                job.context.task.as_deref(),
            ),
            Some(JobHistoryRowRef::Build(record)) => (record.target.as_deref(), None, None),
            None => return None,
        },
        _ => return None,
    };
    if build.is_none() && recipe.is_none() && task.is_none() {
        return None;
    }
    app.logs
        .entries
        .iter()
        .rev()
        .find(|entry| {
            build.is_none_or(|build| entry.build.as_deref() == Some(build))
                && recipe.is_none_or(|recipe| entry.recipe.as_deref() == Some(recipe))
                && task.is_none_or(|task| entry.task.as_deref() == Some(task))
        })
        .map(|entry| entry.id)
}

pub(crate) fn is_pane_focus(target: FocusTarget) -> bool {
    matches!(
        target,
        FocusTarget::Navigator | FocusTarget::Workspace | FocusTarget::Inspector
    )
}

pub(crate) fn dialog_is_open(app: &App) -> bool {
    !app.dialogs.is_empty()
}

pub(crate) fn open_dialog(app: &mut App, dialog: Dialog) {
    if app.dialogs.is_empty() {
        app.dialogs.push_front(dialog);
    }
}

pub(crate) fn replace_dialog(app: &mut App, dialog: Dialog) {
    if let Some(active) = app.dialogs.front_mut() {
        *active = dialog;
    } else {
        app.dialogs.push_front(dialog);
    }
}

pub(crate) fn close_dialog(app: &mut App) {
    app.dialogs.pop_front();
}

pub(crate) fn enqueue_build_completion(app: &mut App) {
    if !app
        .dialogs
        .iter()
        .any(|dialog| matches!(dialog, Dialog::BuildCompletion))
    {
        app.dialogs.push_back(Dialog::BuildCompletion);
    }
}

pub(crate) fn modal_focus(app: &App) -> Option<FocusTarget> {
    if app.command_palette_open {
        Some(FocusTarget::CommandPalette)
    } else if app.menu.is_open()
        || app.onboarding.open
        || app.keymap_preferences_ui.open
        || dialog_is_open(app)
        || (app.screen == Screen::RawMode
            && matches!(app.raw_mode.view, RawModeView::Form | RawModeView::Preview))
    {
        Some(FocusTarget::Dialog)
    } else {
        None
    }
}

pub(crate) fn synchronize_focus(app: &mut App) {
    if let Some(target) = modal_focus(app) {
        if app.focus_return.is_none() && is_pane_focus(app.focus) {
            app.focus_return = Some(app.focus);
        }
        app.focus = target;
    } else {
        let target = app.focus_return.take().unwrap_or(app.focus);
        app.focus = if is_pane_focus(target) && focus_target_is_relevant(app, target) {
            target
        } else {
            FocusTarget::Navigator
        };
    }
}

pub(crate) fn cycle_theme(theme: Theme, backwards: bool) -> Theme {
    const THEMES: [Theme; 8] = [
        Theme::DarkPro,
        Theme::WhiteClassic,
        Theme::MatrixGreen,
        Theme::VscodeDark,
        Theme::VscodeLight,
        Theme::AccessibleDark,
        Theme::SoftLight,
        Theme::HighContrast,
    ];
    let current = THEMES
        .iter()
        .position(|candidate| *candidate == theme)
        .unwrap_or_default();
    let next = if backwards {
        (current + THEMES.len() - 1) % THEMES.len()
    } else {
        (current + 1) % THEMES.len()
    };
    THEMES[next]
}

pub const THEMES: [Theme; 8] = [
    Theme::DarkPro,
    Theme::WhiteClassic,
    Theme::MatrixGreen,
    Theme::VscodeDark,
    Theme::VscodeLight,
    Theme::AccessibleDark,
    Theme::SoftLight,
    Theme::HighContrast,
];

pub fn command_action(app: &App, id: CommandId) -> Action {
    match id {
        CommandId::BuildImage => Action::OpenBuildOptions,
        CommandId::SelectImage => Action::OpenImagePicker(
            app.workspace
                .recipes
                .iter()
                .map(|recipe| recipe.name.as_str())
                .filter(|name| name.contains("image"))
                .map(str::to_owned)
                .collect(),
        ),
        CommandId::BuildSelectedRecipe => Action::BeginSelectedRecipeBuild,
        CommandId::EditBbmask => Action::BeginBbmaskEdit,
        CommandId::OpenDashboard => Action::Open(Screen::Dashboard),
        CommandId::OpenLayers => Action::Open(Screen::Layers),
        CommandId::OpenRecipes => Action::Open(Screen::Recipes),
        CommandId::OpenPackages => Action::Open(Screen::Packages),
        CommandId::OpenImages => Action::Open(Screen::Images),
        CommandId::OpenSdk => Action::Open(Screen::Sdk),
        CommandId::OpenDependencies => Action::Open(Screen::Dependencies),
        CommandId::OpenTesting => Action::Open(Screen::Testing),
        CommandId::OpenSecurity => Action::Open(Screen::Security),
        CommandId::OpenQa => Action::Open(Screen::Qa),
        CommandId::OpenTasks => Action::Open(Screen::Tasks),
        CommandId::OpenLogs => Action::Open(Screen::Logs),
        CommandId::OpenErrors => Action::Open(Screen::Errors),
        CommandId::OpenConfiguration => Action::Open(Screen::Configuration),
        CommandId::OpenRawMode => Action::Open(Screen::RawMode),
        CommandId::OpenGitUi => Action::OpenGitUi,
        CommandId::OpenTerminalSessions => Action::Open(Screen::TerminalSessions),
        CommandId::OpenMaintenance => Action::Open(Screen::Maintenance),
        CommandId::OpenBuildEnvironment => Action::Open(Screen::BuildEnvironment),
        CommandId::OpenCompatibility => Action::Open(Screen::Compatibility),
        CommandId::OpenSettings => Action::Open(Screen::Settings),
        CommandId::ChooseTheme => Action::OpenThemePicker,
        CommandId::FocusNavigator => Action::Focus(FocusTarget::Navigator),
        CommandId::FocusWorkspace => Action::Focus(FocusTarget::Workspace),
        CommandId::FocusInspector => Action::Focus(FocusTarget::Inspector),
        CommandId::PreviousSubfocus => Action::CyclePaneSubfocus { backwards: true },
        CommandId::NextSubfocus => Action::CyclePaneSubfocus { backwards: false },
        CommandId::TogglePaneZoom => Action::TogglePaneZoom,
        CommandId::ScrollFirst => Action::ScrollCurrent { to_end: false },
        CommandId::ScrollLast => Action::ScrollCurrent { to_end: true },
        CommandId::OpenOnboarding => Action::OpenOnboarding,
        CommandId::OpenHelp => Action::Open(Screen::Help),
    }
}
