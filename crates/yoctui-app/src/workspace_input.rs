//! Workspace input.
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
pub fn errors_action(key: Input) -> Option<Action> {
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectError { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectError { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectError { delta: 1 }),
        Input::Enter => Some(Action::JumpToSelectedError),
        Input::Char('o') => Some(Action::OpenSelectedErrorSource),
        _ => None,
    }
}
pub fn dependency_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendDependencyGraphQuery(character)),
            Input::Backspace => Some(Action::BackspaceDependencyGraphQuery),
            Input::CtrlU => Some(Action::ClearDependencyGraphQuery),
            Input::Enter | Input::Esc => Some(Action::FinishDependencyGraphSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectDependencyGraphNode { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectDependencyGraphNode { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectDependencyGraphNode { delta: 1 }),
        Input::Enter => Some(Action::OpenSelectedDependencyRecipe),
        Input::Char('o') => Some(Action::OpenSelectedDependencyProvider),
        Input::Char('L') => Some(Action::OpenSelectedDependencyTaskLog),
        Input::Char('r') => Some(Action::RefreshDependencyGraph),
        Input::Char('/') => Some(Action::BeginDependencyGraphSearch),
        Input::CtrlU => Some(Action::ClearDependencyGraphQuery),
        Input::Char('v') => Some(Action::ToggleDependencyGraphReverse),
        Input::Left | Input::Char('h') => Some(Action::CollapseSelectedDependencyGraphNode),
        Input::Right | Input::Char('l') => Some(Action::ExpandSelectedDependencyGraphNode),
        Input::Char(' ') => Some(Action::ToggleSelectedDependencyGraphNode),
        _ => None,
    }
}
pub fn signature_task_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSignatureTask { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSignatureTask { delta: 1 }),
        Input::Enter => Some(Action::ConfirmSignatureTask),
        Input::Esc => Some(Action::CancelSignatureTaskPicker),
        _ => None,
    }
}
pub fn signature_workspace_action(key: Input) -> Option<Action> {
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectSignatureRecord { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSignatureRecord { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSignatureRecord { delta: 1 }),
        Input::Char('1') => Some(Action::SetSelectedSignatureComparisonSide(
            yoctui_model::SignatureComparisonSide::Left,
        )),
        Input::Char('2') => Some(Action::SetSelectedSignatureComparisonSide(
            yoctui_model::SignatureComparisonSide::Right,
        )),
        Input::Char('c') => Some(Action::BeginSignatureComparison),
        Input::Char('r') => Some(Action::RefreshSignatureDump),
        Input::Char('e') => Some(Action::OpenSignatureProvider),
        Input::Esc => Some(Action::LeaveSignatureWorkspace),
        _ => None,
    }
}
pub fn package_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendPackageQuery(character)),
            Input::Backspace => Some(Action::BackspacePackageQuery),
            Input::CtrlU => Some(Action::ClearPackageQuery),
            Input::Enter | Input::Esc => Some(Action::FinishPackageSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectPackage { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectPackage { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectPackage { delta: 1 }),
        Input::Enter => Some(Action::BeginSelectedPackageDetail),
        Input::Char('/') => Some(Action::BeginPackageSearch),
        Input::CtrlU => Some(Action::ClearPackageQuery),
        Input::Char('R') => Some(Action::RefreshPackageInventory),
        Input::Char('c') => Some(Action::CancelPackageOperation),
        Input::Char('D') => Some(Action::TogglePackageDependencyKind),
        Input::Char('[') => Some(Action::SelectPackageDependency { delta: -1 }),
        Input::Char(']') => Some(Action::SelectPackageDependency { delta: 1 }),
        Input::Char('d') => Some(Action::OpenSelectedPackageDependency),
        Input::Char('u') => Some(Action::BackPackageNavigation),
        Input::Char('o') => Some(Action::OpenSelectedPackageRecipe),
        Input::Char('e') => Some(Action::OpenSelectedPackageProvider),
        _ => None,
    }
}

pub fn images_workspace_action(searching: bool, key: Input) -> Option<Action> {
    images_workspace_action_for_view(searching, yoctui_model::ImagesView::Artifacts, key)
}

pub fn images_workspace_action_for_view(
    searching: bool,
    view: yoctui_model::ImagesView,
    key: Input,
) -> Option<Action> {
    if matches!(key, Input::Tab | Input::BackTab) {
        return Some(Action::ShiftImagesView {
            delta: if key == Input::Tab { 1 } else { -1 },
        });
    }
    if !(view == yoctui_model::ImagesView::Artifacts && searching)
        && let Input::Char(key @ ('1' | '2' | '3' | '4' | '5' | '6')) = key
    {
        let current = yoctui_model::ImagesView::ALL
            .iter()
            .position(|candidate| *candidate == view)
            .unwrap_or(0) as isize;
        let destination = key.to_digit(10).unwrap_or(1) as isize - 1;
        return Some(Action::ShiftImagesView {
            delta: destination - current,
        });
    }
    if view == yoctui_model::ImagesView::RootfsPackages {
        if let Some(delta) = collection_scroll_delta(key) {
            return Some(Action::SelectRootfsPackage { delta });
        }
        return match key {
            Input::Up | Input::Char('k') => Some(Action::SelectRootfsPackage { delta: -1 }),
            Input::Down | Input::Char('j') => Some(Action::SelectRootfsPackage { delta: 1 }),
            Input::Left | Input::Char('h') => Some(Action::SelectRootfsGroup { delta: -1 }),
            Input::Right | Input::Char('l') => Some(Action::SelectRootfsGroup { delta: 1 }),
            Input::Char('r') | Input::Char('R') => Some(Action::RefreshRootfsComposition),
            _ => None,
        };
    }
    if view == yoctui_model::ImagesView::RootfsFilesystem {
        if let Some(delta) = collection_scroll_delta(key) {
            return Some(Action::SelectRootfsEntry { delta });
        }
        return match key {
            Input::Up | Input::Char('k') => Some(Action::SelectRootfsEntry { delta: -1 }),
            Input::Down | Input::Char('j') => Some(Action::SelectRootfsEntry { delta: 1 }),
            Input::Enter | Input::Right => Some(Action::BrowseRootfsFilesystem),
            Input::Char('r') | Input::Char('R') => Some(Action::RefreshRootfsComposition),
            _ => None,
        };
    }
    if view == yoctui_model::ImagesView::UdevRules {
        if let Some(delta) = collection_scroll_delta(key) {
            return Some(Action::SelectRootfsUdevRule { delta });
        }
        return match key {
            Input::Char('[') => Some(Action::ScrollRootfsUdevPreview { delta: -1 }),
            Input::Char(']') => Some(Action::ScrollRootfsUdevPreview { delta: 1 }),
            Input::Enter | Input::Right => Some(Action::BrowseRootfsFilesystem),
            Input::Char('r') | Input::Char('R') => Some(Action::RefreshRootfsComposition),
            _ => None,
        };
    }
    if view == yoctui_model::ImagesView::SystemdServices {
        if let Some(delta) = collection_scroll_delta(key) {
            return Some(Action::SelectRootfsSystemdService { delta });
        }
        return match key {
            Input::Up | Input::Char('k') => Some(Action::SelectRootfsSystemdService { delta: -1 }),
            Input::Down | Input::Char('j') => Some(Action::SelectRootfsSystemdService { delta: 1 }),
            Input::Enter | Input::Right => Some(Action::BrowseRootfsFilesystem),
            Input::Char('e') => Some(Action::EditSelectedRootfsSystemFile),
            Input::Char('r') | Input::Char('R') => Some(Action::RefreshRootfsComposition),
            _ => None,
        };
    }
    if view == yoctui_model::ImagesView::SystemDbus {
        if let Some(delta) = collection_scroll_delta(key) {
            return Some(Action::SelectRootfsDbusService { delta });
        }
        return match key {
            Input::Up | Input::Char('k') => Some(Action::SelectRootfsDbusService { delta: -1 }),
            Input::Down | Input::Char('j') => Some(Action::SelectRootfsDbusService { delta: 1 }),
            Input::Enter | Input::Right => Some(Action::BrowseRootfsFilesystem),
            Input::Char('e') => Some(Action::EditSelectedRootfsSystemFile),
            Input::Char('r') | Input::Char('R') => Some(Action::RefreshRootfsComposition),
            _ => None,
        };
    }
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendImageArtifactQuery(character)),
            Input::Backspace => Some(Action::BackspaceImageArtifactQuery),
            Input::CtrlU => Some(Action::ClearImageArtifactQuery),
            Input::Enter | Input::Esc => Some(Action::FinishImageArtifactSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectImageArtifact { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectImageArtifact { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectImageArtifact { delta: 1 }),
        Input::Char('/') => Some(Action::BeginImageArtifactSearch),
        Input::CtrlU => Some(Action::ClearImageArtifactQuery),
        Input::Char('R') => Some(Action::RefreshImageArtifactInventory),
        Input::Char('c') => Some(Action::CancelImageArtifactOperation),
        Input::Char('b') => Some(Action::BeginSelectedImageArtifactBuild),
        Input::Char('T') => Some(Action::BeginSelectedImageConsole),
        Input::Char('Q') => Some(Action::BeginSelectedQemuLaunch),
        Input::Char('W') => Some(Action::BeginSelectedWicCreate),
        Input::Char('D') => Some(Action::BeginSelectedWicDeviceWrite),
        Input::Char('x') => Some(Action::BeginActiveImageRuntimeCancellation),
        Input::Char('[') => Some(Action::SelectWicOutput { delta: -1 }),
        Input::Char(']') => Some(Action::SelectWicOutput { delta: 1 }),
        Input::Char('O') => Some(Action::OpenSelectedWicOutput),
        Input::Char('o') => Some(Action::OpenSelectedImageArtifact),
        Input::Char('m') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Manifest,
        )),
        Input::Char('l') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::License,
        )),
        Input::Char('s') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Spdx,
        )),
        Input::Char('w') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Wic,
        )),
        Input::Char('p') | Input::Enter => Some(Action::BeginSelectedRootfsComposition),
        _ => None,
    }
}

pub fn sdk_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendSdkArtifactQuery(character)),
            Input::Backspace => Some(Action::BackspaceSdkArtifactQuery),
            Input::CtrlU => Some(Action::ClearSdkArtifactQuery),
            Input::Enter | Input::Esc => Some(Action::FinishSdkArtifactSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectSdkArtifact { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSdkArtifact { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSdkArtifact { delta: 1 }),
        Input::Char('/') => Some(Action::BeginSdkArtifactSearch),
        Input::CtrlU => Some(Action::ClearSdkArtifactQuery),
        Input::Char('R') => Some(Action::RefreshSdkArtifactInventory),
        Input::Char('s') => Some(Action::BeginSdkBuild(SdkBuildAction::Populate(
            SdkKind::Standard,
        ))),
        Input::Char('E') => Some(Action::BeginSdkBuild(SdkBuildAction::Populate(
            SdkKind::Extensible,
        ))),
        Input::Char('t') => Some(Action::BeginSdkBuild(SdkBuildAction::Test(
            SdkKind::Standard,
        ))),
        Input::Char('T') => Some(Action::BeginSdkBuild(SdkBuildAction::Test(
            SdkKind::Extensible,
        ))),
        Input::Char('P') => Some(Action::BeginSelectedSdkPublish),
        Input::Char('n') => Some(Action::BeginSdkNative),
        Input::Char('o') => Some(Action::OpenSelectedSdkArtifact),
        Input::Char('c') => Some(Action::BeginActiveSdkSessionCancellation),
        _ => None,
    }
}

pub fn sdk_build_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmSdkBuild),
        Input::Esc => Some(Action::CancelSdkBuild),
        _ => None,
    }
}

pub fn sdk_publish_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendSdkPublishDestination(character)),
        Input::Backspace => Some(Action::BackspaceSdkPublishDestination),
        Input::Enter => Some(Action::PreviewSdkPublish),
        Input::Esc => Some(Action::CancelSdkPublish),
        _ => None,
    }
}

pub fn sdk_publish_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmSdkPublish),
        Input::Esc => Some(Action::CancelSdkPublishPreview),
        _ => None,
    }
}

pub fn sdk_native_dialog_action(editing: bool, key: Input) -> Option<Action> {
    if editing {
        return match key {
            Input::Char(character) => Some(Action::AppendSdkNativeField(character)),
            Input::Backspace => Some(Action::BackspaceSdkNativeField),
            Input::Enter => Some(Action::FinishSdkNativeFieldEdit),
            Input::Esc => Some(Action::CancelSdkNative),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSdkNativeField { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSdkNativeField { delta: 1 }),
        Input::Left | Input::Right | Input::Char('h') | Input::Char('l') => {
            Some(Action::CycleSdkNativeMode)
        }
        Input::Enter => Some(Action::ActivateSdkNativeField),
        Input::Char('p') => Some(Action::PreviewSdkNative),
        Input::Esc => Some(Action::CancelSdkNative),
        _ => None,
    }
}

pub fn sdk_native_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmSdkNative),
        Input::Esc => Some(Action::CancelSdkNativePreview),
        _ => None,
    }
}

pub fn sdk_cancellation_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmSdkSessionCancellation),
        Input::Esc => Some(Action::CancelSdkSessionCancellation),
        _ => None,
    }
}

pub fn testing_workspace_action(key: Input) -> Option<Action> {
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectTestFamily { delta });
    }
    match key {
        Input::Tab => Some(Action::CycleTestView),
        Input::Up | Input::Char('k') => Some(Action::SelectTestFamily { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectTestFamily { delta: 1 }),
        Input::Enter | Input::Char('r') => Some(Action::BeginSelectedTestLaunch),
        Input::Char('x') => Some(Action::BeginActiveTestSessionCancellation),
        _ => None,
    }
}

pub fn security_workspace_action(
    view: SecurityView,
    drilled: bool,
    searching: bool,
    key: Input,
) -> Option<Action> {
    let security = |action| Some(Action::Security(action));
    if searching {
        return match key {
            Input::Char(character) => security(SecurityAction::AppendQuery(character)),
            Input::Backspace => security(SecurityAction::BackspaceQuery),
            Input::CtrlU => security(SecurityAction::ClearQuery),
            Input::Enter | Input::Esc => security(SecurityAction::FinishSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return security(if view == SecurityView::Cves {
            SecurityAction::SelectFinding(delta)
        } else if drilled {
            SecurityAction::SelectComponent(delta)
        } else {
            SecurityAction::SelectReport(delta)
        });
    }
    match key {
        Input::Tab => security(SecurityAction::CycleView),
        Input::Up | Input::Char('k') => security(if view == SecurityView::Cves {
            SecurityAction::SelectFinding(-1)
        } else if drilled {
            SecurityAction::SelectComponent(-1)
        } else {
            SecurityAction::SelectReport(-1)
        }),
        Input::Down | Input::Char('j') => security(if view == SecurityView::Cves {
            SecurityAction::SelectFinding(1)
        } else if drilled {
            SecurityAction::SelectComponent(1)
        } else {
            SecurityAction::SelectReport(1)
        }),
        Input::Enter => security(SecurityAction::Drill),
        Input::Esc if drilled => security(SecurityAction::LeaveDrill),
        Input::Char('s') => security(SecurityAction::CycleScope),
        Input::Char('/') => security(SecurityAction::BeginSearch),
        Input::CtrlU => security(SecurityAction::ClearQuery),
        Input::Char('f') => security(SecurityAction::CycleCveFilter),
        Input::Char('V') => security(SecurityAction::BeginCveCheck),
        Input::Char('M') => security(SecurityAction::BeginPackageMap),
        Input::Char('X') => security(SecurityAction::BeginSbomGeneration),
        Input::Char('I') => security(SecurityAction::BeginImport),
        Input::Char('R') => security(SecurityAction::RefreshReports),
        Input::Char('o') => security(SecurityAction::OpenSelectedReport),
        Input::Char('e') => security(SecurityAction::OpenSelectedRecipe),
        Input::Char('v') => security(SecurityAction::OpenSelectedAdvisory),
        Input::Char('c') => security(SecurityAction::BeginCancellation),
        _ => None,
    }
}

pub fn security_dialog_action(dialog: &SecurityDialog, key: Input) -> Option<Action> {
    let security = |action| Some(Action::Security(action));
    match dialog {
        SecurityDialog::Operation(preview) => match key {
            Input::Enter => security(SecurityAction::ConfirmOperation(preview.clone())),
            Input::Esc => security(SecurityAction::CancelDialog),
            _ => None,
        },
        SecurityDialog::Cancellation(id) => match key {
            Input::Enter => security(SecurityAction::ConfirmCancellation(*id)),
            Input::Esc => security(SecurityAction::CancelDialog),
            _ => None,
        },
        SecurityDialog::Import { editor, .. } => match key {
            Input::Enter => security(SecurityAction::ConfirmImport(editor.text.clone())),
            Input::Char('q') | Input::Esc if !editor.editing => {
                security(SecurityAction::CancelDialog)
            }
            input => popup_editor_action(editor.editing, input),
        },
    }
}

pub fn qa_workspace_action(
    view: QaView,
    drilled: bool,
    searching: bool,
    key: Input,
) -> Option<Action> {
    let qa = |action| Some(Action::Qa(action));
    if searching {
        return match key {
            Input::Char(character) => qa(QaAction::AppendQuery(character)),
            Input::Backspace => qa(QaAction::BackspaceQuery),
            Input::CtrlU => qa(QaAction::ClearQuery),
            Input::Enter | Input::Esc => qa(QaAction::FinishSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return qa(if drilled {
            QaAction::SelectFinding(delta)
        } else if view == QaView::LayerQa {
            QaAction::SelectLayer(delta)
        } else {
            QaAction::SelectCheck(delta)
        });
    }
    match key {
        Input::Tab => qa(QaAction::CycleView),
        Input::Up | Input::Char('k') => qa(if drilled {
            QaAction::SelectFinding(-1)
        } else if view == QaView::LayerQa {
            QaAction::SelectLayer(-1)
        } else {
            QaAction::SelectCheck(-1)
        }),
        Input::Down | Input::Char('j') => qa(if drilled {
            QaAction::SelectFinding(1)
        } else if view == QaView::LayerQa {
            QaAction::SelectLayer(1)
        } else {
            QaAction::SelectCheck(1)
        }),
        Input::Enter => qa(QaAction::Drill),
        Input::Esc if drilled => qa(QaAction::LeaveDrill),
        Input::Char('s') => qa(if view == QaView::LayerQa {
            QaAction::SelectLayer(1)
        } else {
            QaAction::CycleScope
        }),
        Input::Char('/') => qa(QaAction::BeginSearch),
        Input::CtrlU => qa(QaAction::ClearQuery),
        Input::Char('f') => qa(QaAction::CycleStatusFilter),
        Input::Char('r') => qa(if view == QaView::LayerQa {
            QaAction::BeginSelectedLayerCheck
        } else {
            QaAction::BeginSelectedCheck
        }),
        Input::Char('I') => qa(QaAction::BeginImport),
        Input::Char('R') => qa(QaAction::RefreshReports),
        Input::Char('o') => qa(QaAction::OpenSelectedReport),
        Input::Char('e') => qa(if view == QaView::LayerQa {
            QaAction::OpenSelectedLayerRoot
        } else {
            QaAction::OpenProvider
        }),
        Input::Char('l') => qa(QaAction::OpenSelectedSource),
        Input::Char('c') => qa(if view == QaView::LayerQa {
            QaAction::BeginLayerCancellation
        } else {
            QaAction::BeginCancellation
        }),
        _ => None,
    }
}

pub fn qa_dialog_action(dialog: &QaDialog, key: Input) -> Option<Action> {
    let qa = |action| Some(Action::Qa(action));
    match dialog {
        QaDialog::Operation(preview) => match key {
            Input::Enter => qa(QaAction::ConfirmOperation(preview.clone())),
            Input::Esc => qa(QaAction::CancelDialog),
            _ => None,
        },
        QaDialog::LayerOperation(preview) => match key {
            Input::Enter => qa(QaAction::ConfirmLayerOperation(preview.clone())),
            Input::Esc => qa(QaAction::CancelDialog),
            _ => None,
        },
        QaDialog::Cancellation { session, .. } => match key {
            Input::Enter => qa(QaAction::ConfirmCancellation(*session)),
            Input::Esc => qa(QaAction::CancelDialog),
            _ => None,
        },
        QaDialog::LayerCancellation(session) => match key {
            Input::Enter => qa(QaAction::ConfirmLayerCancellation(*session)),
            Input::Esc => qa(QaAction::CancelDialog),
            _ => None,
        },
        QaDialog::Import { editor, .. } => match key {
            Input::Enter => qa(QaAction::ConfirmImport(editor.text.clone())),
            Input::Char('q') | Input::Esc if !editor.editing => qa(QaAction::CancelDialog),
            input => popup_editor_action(editor.editing, input),
        },
    }
}

#[cfg(test)]
mod focus_flow_tests {
    use super::*;
    #[test]
    fn focus_arrows_enter_workspace_and_escape_unwinds_without_losing_selection() {
        let mut app = App::new(10, 1024);
        yoctui_model::update(&mut app, Action::Open(Screen::Recipes));
        for key in [Input::Right, Input::Esc, Input::Enter] {
            let action = focus_action_for_app(&app, key).unwrap();
            yoctui_model::update(&mut app, action);
        }
        assert_eq!(app.screen, Screen::Recipes);
        assert_eq!(app.focus, FocusTarget::Workspace);
        let selected = app.navigator_selection;
        yoctui_model::update(
            &mut app,
            focus_action(FocusTarget::Workspace, Input::Esc).unwrap(),
        );
        assert_eq!(app.focus, FocusTarget::Navigator);
        assert_eq!(app.navigator_selection, selected);
        yoctui_model::update(
            &mut app,
            focus_action(FocusTarget::Navigator, Input::Esc).unwrap(),
        );
        assert_eq!(app.screen, Screen::Dashboard);
    }
    #[test]
    fn focus_routes_preserve_search_and_terminal_mode_keys() {
        let mut app = App::new(10, 1024);
        app.screen = Screen::Recipes;
        app.focus = FocusTarget::Workspace;
        app.metadata_searching = true;
        for key in [Input::Esc, Input::Char('q'), Input::Tab, Input::Up] {
            assert!(focus_action_for_app(&app, key).is_none());
        }
        app.screen = Screen::TerminalSessions;
        app.terminal.mode = yoctui_model::TerminalWorkbenchMode::Search;
        for key in [Input::Esc, Input::Char('q'), Input::Tab] {
            assert!(focus_action_for_app(&app, key).is_none());
        }
        assert_eq!(
            terminal_workspace_action(&app, Input::Esc),
            Some(Action::TerminalFinishSearch)
        );
    }
}
