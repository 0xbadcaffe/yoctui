use super::*;

pub fn key_action(key: Input) -> Option<Action> {
    match key {
        Input::Char('b') => None,
        Input::Char('c') => Some(Action::Cancel),
        Input::Char('f') => Some(Action::ToggleLogFollow),
        Input::Char('w') => Some(Action::ToggleLogWrap),
        Input::Char('s') => Some(Action::CycleLogSeverity),
        Input::Char('/') => Some(Action::BeginLogSearch),
        Input::Char('n') => Some(Action::NextLogMatch),
        Input::Char('N') => Some(Action::PreviousLogMatch),
        Input::CtrlU => Some(Action::ClearLogQuery),
        Input::Char('R') => Some(Action::CycleLogRecipeFilter),
        Input::Char('T') => Some(Action::CycleLogTaskFilter),
        Input::Backspace => Some(Action::BackspaceLogQuery),
        Input::Up => Some(Action::ScrollLogs { delta: 1 }),
        Input::Down => Some(Action::ScrollLogs { delta: -1 }),
        Input::Left => Some(Action::ScrollLogsHorizontally { delta: -8 }),
        Input::Right => Some(Action::ScrollLogsHorizontally { delta: 8 }),
        Input::Char('l') => Some(Action::Open(Screen::Logs)),
        Input::Char('h') => Some(Action::Open(Screen::BuildHistory)),
        Input::Char('e') => Some(Action::Open(Screen::Errors)),
        Input::Char('E') => Some(Action::Open(Screen::BuildEnvironment)),
        Input::Char('M') => Some(Action::Open(Screen::Maintenance)),
        Input::Char('r') => Some(Action::Open(Screen::Recipes)),
        Input::Char('y') => Some(Action::Open(Screen::Layers)),
        Input::Char('v') => Some(Action::Open(Screen::Configuration)),
        Input::Char('x') => Some(Action::Open(Screen::Bbmask)),
        Input::Char('B') => Some(Action::OpenBuildOptions),
        Input::Char('a') => Some(Action::OpenContextMenu),
        Input::Char('?') => Some(Action::Open(Screen::Help)),
        Input::Char('q') | Input::CtrlC => Some(Action::Quit),
        Input::CtrlP => Some(Action::OpenCommandPalette),
        Input::F1 => Some(yoctui_model::function_shortcut_action(FunctionKey::F1)),
        Input::F2 => Some(yoctui_model::function_shortcut_action(FunctionKey::F2)),
        Input::F3 => Some(yoctui_model::function_shortcut_action(FunctionKey::F3)),
        Input::F4 => Some(yoctui_model::function_shortcut_action(FunctionKey::F4)),
        Input::F5 => Some(yoctui_model::function_shortcut_action(FunctionKey::F5)),
        Input::F6 => Some(yoctui_model::function_shortcut_action(FunctionKey::F6)),
        Input::F7 => Some(yoctui_model::function_shortcut_action(FunctionKey::F7)),
        Input::F8 => Some(yoctui_model::function_shortcut_action(FunctionKey::F8)),
        Input::F9 => Some(yoctui_model::function_shortcut_action(FunctionKey::F9)),
        Input::F10 => Some(yoctui_model::function_shortcut_action(FunctionKey::F10)),
        Input::Tab => Some(Action::CycleFocus { backwards: false }),
        Input::BackTab => Some(Action::CycleFocus { backwards: true }),
        Input::Char('Y') => Some(Action::ConfirmQuit),
        Input::Enter => Some(Action::ActivateNotification),
        Input::Esc => Some(Action::Open(Screen::Dashboard)),
        _ => None,
    }
}

pub fn build_cancellation_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Char('y') | Input::Char('Y') | Input::Enter => {
            Some(Action::ConfirmBuildCancellation)
        }
        Input::Char('n') | Input::Char('N') | Input::Esc => Some(Action::CancelBuildCancellation),
        _ => None,
    }
}

pub fn quit_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Char('y') | Input::Char('Y') | Input::Enter => Some(Action::ConfirmQuit),
        Input::Char('n') | Input::Char('N') | Input::Esc => Some(Action::CancelQuit),
        _ => None,
    }
}

pub fn focus_action(focus: FocusTarget, key: Input) -> Option<Action> {
    if focus == FocusTarget::Navigator
        && let Some(delta) = collection_scroll_delta(key)
    {
        return Some(Action::SelectNavigator { delta });
    }
    match (focus, key) {
        (
            FocusTarget::Navigator | FocusTarget::Workspace | FocusTarget::Inspector,
            Input::Char('q') | Input::CtrlC,
        ) => Some(Action::Quit),
        (FocusTarget::Navigator, Input::Up | Input::Char('k')) => {
            Some(Action::SelectNavigator { delta: -1 })
        }
        (FocusTarget::Navigator, Input::Down | Input::Char('j')) => {
            Some(Action::SelectNavigator { delta: 1 })
        }
        (FocusTarget::Navigator, Input::Esc) => Some(Action::Open(Screen::Dashboard)),
        (FocusTarget::Navigator, Input::Enter) => Some(Action::ActivateNavigator),
        (FocusTarget::Navigator, Input::Left | Input::Char('h')) => {
            Some(Action::CollapseNavigatorGroup)
        }
        (FocusTarget::Navigator, Input::Right | Input::Char('l')) => {
            Some(Action::ExpandNavigatorGroup)
        }
        (FocusTarget::Navigator | FocusTarget::Workspace | FocusTarget::Inspector, Input::Tab) => {
            Some(Action::CycleFocus { backwards: false })
        }
        (
            FocusTarget::Navigator | FocusTarget::Workspace | FocusTarget::Inspector,
            Input::BackTab,
        ) => Some(Action::CycleFocus { backwards: true }),
        (FocusTarget::Workspace | FocusTarget::Inspector, Input::Esc) => {
            Some(Action::Focus(FocusTarget::Navigator))
        }
        _ => None,
    }
}

pub fn focus_action_for_app(app: &yoctui_model::App, key: Input) -> Option<Action> {
    if terminal_owns_input(app)
        || (app.focus == FocusTarget::Workspace && workspace_text_input_active(app))
    {
        return None;
    }
    if matches!(app.focus, FocusTarget::Dialog | FocusTarget::CommandPalette) {
        return focus_action(app.focus, key);
    }
    if key == Input::Esc {
        if app.zoomed_pane.is_some() {
            return Some(Action::TogglePaneZoom);
        }
        if app.focus == FocusTarget::Workspace
            && app.workspace_subfocus != yoctui_model::WorkspaceSubfocus::Main
        {
            return Some(Action::ResetPaneSubfocus);
        }
        if app.focus == FocusTarget::Inspector
            && app.inspector_subfocus != yoctui_model::InspectorSubfocus::Facts
        {
            return Some(Action::ResetPaneSubfocus);
        }
    }
    focus_action(app.focus, key)
}

pub fn settings_action(key: Input) -> Option<Action> {
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectSetting { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSetting { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSetting { delta: 1 }),
        Input::Left => Some(Action::ChangeSelectedSetting { backwards: true }),
        Input::Right | Input::Enter => Some(Action::ChangeSelectedSetting { backwards: false }),
        Input::Char('r') => Some(Action::RetrySettingsPersistence),
        Input::Char('R') => Some(Action::ResetPreferences),
        _ => None,
    }
}

pub fn compatibility_ui_inspector_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Up | Input::Char('k') => {
                Some(Action::SelectCompatibilityCapability { delta: -1 })
            }
            Input::Down | Input::Char('j') => {
                Some(Action::SelectCompatibilityCapability { delta: 1 })
            }
            Input::Char(character) => Some(Action::AppendCompatibilityQuery(character)),
            Input::Backspace => Some(Action::BackspaceCompatibilityQuery),
            Input::CtrlU => Some(Action::ClearCompatibilityQuery),
            Input::Enter | Input::Esc => Some(Action::FinishCompatibilitySearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectCompatibilityCapability { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectCompatibilityCapability { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectCompatibilityCapability { delta: 1 }),
        Input::Char('1') => Some(Action::SetCompatibilityFilter(
            yoctui_model::CompatibilityUiFilter::All,
        )),
        Input::Char('2') => Some(Action::SetCompatibilityFilter(
            yoctui_model::CompatibilityUiFilter::Available,
        )),
        Input::Char('3') => Some(Action::SetCompatibilityFilter(
            yoctui_model::CompatibilityUiFilter::Limited,
        )),
        Input::Char('4') => Some(Action::SetCompatibilityFilter(
            yoctui_model::CompatibilityUiFilter::Unavailable,
        )),
        Input::Char('5') => Some(Action::SetCompatibilityFilter(
            yoctui_model::CompatibilityUiFilter::Attention,
        )),
        Input::Char('/') => Some(Action::BeginCompatibilitySearch),
        Input::CtrlU => Some(Action::ClearCompatibilityQuery),
        _ => None,
    }
}

pub fn build_environment_action(key: Input) -> Option<Action> {
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectBuildEnvironmentField { delta });
    }
    match key {
        Input::Enter | Input::Char('e') => Some(Action::EnvironmentSetup(
            yoctui_model::EnvironmentSetupAction::Open { browse: false },
        )),
        Input::Char('b') => Some(Action::EnvironmentSetup(
            yoctui_model::EnvironmentSetupAction::Open { browse: true },
        )),
        Input::Char('A') => Some(Action::OpenBuildEnvironmentEditor),
        Input::Char('c') => Some(Action::OpenBuildEnvironmentCloneEditor),
        Input::Up | Input::Char('k') => Some(Action::SelectBuildEnvironmentField { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectBuildEnvironmentField { delta: 1 }),
        Input::Char('s') => Some(Action::ApplyBuildEnvironmentProfile),
        Input::Char('V') => Some(Action::BeginBuildEnvironmentVerification),
        Input::Char('n') => Some(Action::SelectProjectProfileItem { delta: 1 }),
        Input::Char('N') => Some(Action::SelectProjectProfileItem { delta: -1 }),
        Input::Char('p') => Some(Action::ActivateProjectProfileItem),
        Input::Esc => Some(Action::Open(Screen::Dashboard)),
        _ => None,
    }
}
pub fn tasks_action(editing: bool, key: Input) -> Option<Action> {
    if editing {
        return match key {
            Input::Char(character) => Some(Action::AppendTaskFilter(character)),
            Input::Backspace => Some(Action::BackspaceTaskFilter),
            Input::Enter | Input::Esc => Some(Action::FinishTaskFilterEdit),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::ScrollBuildTasks { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::ScrollBuildTasks { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::ScrollBuildTasks { delta: 1 }),
        Input::Char('f') => Some(Action::CycleTaskStateFilter),
        Input::Char('F') => Some(Action::CycleTaskFilterField),
        Input::Char('/') => Some(Action::BeginTaskFilterEdit),
        Input::Char('d') => Some(Action::CycleTaskDurationFilter),
        _ => None,
    }
}
pub fn logs_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendLogQuery(character)),
            Input::Backspace => Some(Action::BackspaceLogQuery),
            Input::CtrlU => Some(Action::ClearLogQuery),
            Input::Enter | Input::Esc => Some(Action::FinishLogSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::ScrollLogs {
            delta: delta.saturating_neg(),
        });
    }
    match key {
        Input::Char('v') => Some(Action::CycleLogWorkspaceView),
        Input::Up | Input::Char('k') => Some(Action::ScrollLogs { delta: 1 }),
        Input::Down | Input::Char('j') => Some(Action::ScrollLogs { delta: -1 }),
        Input::Left => Some(Action::ScrollLogsHorizontally { delta: -8 }),
        Input::Right => Some(Action::ScrollLogsHorizontally { delta: 8 }),
        Input::Char('f') => Some(Action::ToggleLogFollow),
        Input::Char('w') => Some(Action::ToggleLogWrap),
        Input::Char('s') => Some(Action::CycleLogSeverity),
        Input::Char('/') => Some(Action::BeginLogSearch),
        Input::CtrlU => Some(Action::ClearLogQuery),
        Input::Char('n') => Some(Action::NextLogMatch),
        Input::Char('N') => Some(Action::PreviousLogMatch),
        Input::Char('R') => Some(Action::CycleLogRecipeFilter),
        Input::Char('T') => Some(Action::CycleLogTaskFilter),
        Input::Char('B') => Some(Action::CycleLogBuildFilter),
        Input::Char('S') => Some(Action::CycleLogSourceFilter),
        Input::Char('I') => Some(Action::CycleLogTimeRange),
        Input::Char('m') => Some(Action::ToggleSelectedLogBookmark),
        Input::Char(']') => Some(Action::NextLogBookmark),
        Input::Char('[') => Some(Action::PreviousLogBookmark),
        Input::Char('o') => Some(Action::OpenSelectedLogSource),
        Input::Char('C') => Some(Action::CopySelectedLog),
        Input::Char('E') => Some(Action::ExportFilteredLogs),
        _ => None,
    }
}

pub fn internal_logs_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendInternalLogQuery(character)),
            Input::Backspace => Some(Action::BackspaceInternalLogQuery),
            Input::CtrlU => Some(Action::ClearInternalLogQuery),
            Input::Enter | Input::Esc => Some(Action::FinishInternalLogSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::ScrollInternalLogs { delta });
    }
    match key {
        Input::Char('v') => Some(Action::CycleLogWorkspaceView),
        Input::Char('f') => Some(Action::ToggleInternalLogFollow),
        Input::Char('s') => Some(Action::CycleInternalLogLevelFilter),
        Input::Char('T') => Some(Action::CycleInternalLogTargetFilter),
        Input::Char('/') => Some(Action::BeginInternalLogSearch),
        Input::CtrlU => Some(Action::ClearInternalLogQuery),
        Input::Char('c') => Some(Action::ClearInternalLogs),
        Input::Char('E') => Some(Action::ExportInternalLogs),
        _ => None,
    }
}

pub fn log_workspace_action(app: &yoctui_model::App, key: Input) -> Option<Action> {
    match app.log_workspace_view {
        yoctui_model::LogWorkspaceView::BitBake => logs_action(app.logs.searching, key),
        yoctui_model::LogWorkspaceView::Yoctui => {
            internal_logs_action(app.internal_logs.searching, key)
        }
    }
}
