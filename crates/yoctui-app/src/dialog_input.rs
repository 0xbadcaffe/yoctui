//! Dialog input.
use super::*;

pub fn test_launch_dialog_action(editing: bool, key: Input) -> Option<Action> {
    if editing {
        return match key {
            Input::Char(character) => Some(Action::AppendTestLaunchField(character)),
            Input::Backspace => Some(Action::BackspaceTestLaunchField),
            Input::Enter => Some(Action::FinishTestLaunchFieldEdit),
            Input::Esc => Some(Action::CancelTestLaunch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectTestLaunchField { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectTestLaunchField { delta: 1 }),
        Input::Left | Input::Right | Input::Char('h') | Input::Char('l') | Input::Enter => {
            Some(Action::ActivateTestLaunchField)
        }
        Input::Char('p') => Some(Action::PreviewTestLaunch),
        Input::Esc => Some(Action::CancelTestLaunch),
        _ => None,
    }
}

pub fn test_launch_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmTestLaunch),
        Input::Esc => Some(Action::CancelTestLaunchPreview),
        _ => None,
    }
}

pub fn test_cancellation_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmTestSessionCancellation),
        Input::Esc => Some(Action::CancelTestSessionCancellation),
        _ => None,
    }
}

pub fn test_results_workspace_action(searching: bool, drilled: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendTestResultQuery(character)),
            Input::Backspace => Some(Action::BackspaceTestResultQuery),
            Input::CtrlU => Some(Action::ClearTestResultQuery),
            Input::Enter | Input::Esc => Some(Action::FinishTestResultSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(if drilled {
            Action::SelectTestCase { delta }
        } else {
            Action::SelectTestResult { delta }
        });
    }
    match key {
        Input::Tab => Some(Action::CycleTestView),
        Input::Up | Input::Char('k') if drilled => Some(Action::SelectTestCase { delta: -1 }),
        Input::Down | Input::Char('j') if drilled => Some(Action::SelectTestCase { delta: 1 }),
        Input::Up | Input::Char('k') => Some(Action::SelectTestResult { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectTestResult { delta: 1 }),
        Input::Enter if drilled => None,
        Input::Enter => Some(Action::DrillIntoSelectedTestResult),
        Input::Esc if drilled => Some(Action::LeaveTestResultCases),
        Input::Char('/') => Some(Action::BeginTestResultSearch),
        Input::CtrlU => Some(Action::ClearTestResultQuery),
        Input::Char('I') => Some(Action::BeginTestResultImport),
        Input::Char('R') => Some(Action::RefreshTestResults),
        Input::Char('c') => Some(Action::BeginTestComparison),
        Input::Char('J') => Some(Action::BeginTestJunitExport),
        Input::Char('o') => Some(Action::OpenSelectedTestResult),
        Input::Char('l') => Some(Action::OpenSelectedTestCaseLog),
        Input::Char('x') => Some(Action::BeginActiveTestSessionCancellation),
        _ => None,
    }
}

pub fn test_comparison_workspace_action(key: Input) -> Option<Action> {
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectTestComparisonTransition { delta });
    }
    match key {
        Input::Tab => Some(Action::CycleTestView),
        Input::Up | Input::Char('k') => Some(Action::SelectTestComparisonTransition { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectTestComparisonTransition { delta: 1 }),
        Input::Char('c') => Some(Action::BeginTestComparison),
        Input::Char('l') => Some(Action::OpenSelectedTestTransitionLog),
        Input::Char('x') => Some(Action::BeginActiveTestSessionCancellation),
        _ => None,
    }
}

pub fn test_result_import_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendTestResultImport(character)),
        Input::Backspace => Some(Action::BackspaceTestResultImport),
        Input::Enter => Some(Action::ConfirmTestResultImport),
        Input::Esc => Some(Action::CancelTestResultImport),
        _ => None,
    }
}

pub fn test_comparison_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectTestComparisonChoice { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectTestComparisonChoice { delta: 1 }),
        Input::Left | Input::Right | Input::Char('h') | Input::Char('l') => {
            Some(Action::CycleTestComparisonField)
        }
        Input::Enter => Some(Action::ActivateTestComparisonChoice),
        Input::Char('p') => Some(Action::PreviewTestComparison),
        Input::Esc => Some(Action::CancelTestComparison),
        _ => None,
    }
}

pub fn test_comparison_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmTestComparison),
        Input::Esc => Some(Action::CancelTestComparisonPreview),
        _ => None,
    }
}

pub fn test_junit_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendTestJunitDestination(character)),
        Input::Backspace => Some(Action::BackspaceTestJunitDestination),
        Input::Enter => Some(Action::PreviewTestJunitExport),
        Input::Esc => Some(Action::CancelTestJunitExport),
        _ => None,
    }
}

pub fn test_junit_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmTestJunitExport),
        Input::Esc => Some(Action::CancelTestJunitExportPreview),
        _ => None,
    }
}

pub fn wic_create_dialog_action(editing: bool, key: Input) -> Option<Action> {
    if editing {
        return match key {
            Input::Char(character) => Some(Action::AppendWicCreateField(character)),
            Input::Backspace => Some(Action::BackspaceWicCreateField),
            Input::Enter => Some(Action::FinishWicCreateFieldEdit),
            Input::Esc => Some(Action::CancelWicCreate),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectWicCreateField { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectWicCreateField { delta: 1 }),
        Input::Left | Input::Char('h') => Some(Action::CycleWicCreateChoice { backwards: true }),
        Input::Right | Input::Char('l') => Some(Action::CycleWicCreateChoice { backwards: false }),
        Input::Enter => Some(Action::ActivateWicCreateField),
        Input::Char('p') => Some(Action::PreviewWicCreate),
        Input::Esc => Some(Action::CancelWicCreate),
        _ => None,
    }
}

pub fn wic_create_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmWicCreate),
        Input::Esc => Some(Action::CancelWicCreatePreview),
        _ => None,
    }
}

pub fn wic_device_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectWicDevice { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectWicDevice { delta: 1 }),
        Input::Enter => Some(Action::ConfirmWicDeviceSelection),
        Input::Esc => Some(Action::CancelWicDevicePicker),
        _ => None,
    }
}

pub fn wic_write_phrase_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendWicWritePhrase(character)),
        Input::Backspace => Some(Action::BackspaceWicWritePhrase),
        Input::Enter => Some(Action::PreviewWicDeviceWrite),
        Input::Esc => Some(Action::CancelWicWritePhrase),
        _ => None,
    }
}

pub fn wic_write_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmWicDeviceWrite),
        Input::Esc => Some(Action::CancelWicWritePreview),
        _ => None,
    }
}

pub fn wic_cancellation_confirmation_action(
    id: WicSessionId,
    incomplete_device_warning: bool,
    key: Input,
) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmWicSessionCancellation {
            id,
            acknowledge_incomplete_device: incomplete_device_warning,
        }),
        Input::Esc => Some(Action::CancelWicSessionCancellation),
        _ => None,
    }
}

pub fn qemu_launch_dialog_action(editing: bool, key: Input) -> Option<Action> {
    if editing {
        return match key {
            Input::Char(character) => Some(Action::AppendQemuLaunchField(character)),
            Input::Backspace => Some(Action::BackspaceQemuLaunchField),
            Input::Enter => Some(Action::FinishQemuLaunchFieldEdit),
            Input::Esc => Some(Action::CancelQemuLaunch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectQemuLaunchField { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectQemuLaunchField { delta: 1 }),
        Input::Left | Input::Char('h') => Some(Action::CycleQemuLaunchChoice { backwards: true }),
        Input::Right | Input::Char('l') => Some(Action::CycleQemuLaunchChoice { backwards: false }),
        Input::Enter => Some(Action::ActivateQemuLaunchField),
        Input::Char('p') => Some(Action::PreviewQemuLaunch),
        Input::Esc => Some(Action::CancelQemuLaunch),
        _ => None,
    }
}

pub fn image_console_dialog_action(
    dialog: &yoctui_model::ImageConsoleDialog,
    key: Input,
) -> Option<Action> {
    match key {
        Input::Up => Some(Action::SelectImageConsoleField { delta: -1 }),
        Input::Down | Input::Tab => Some(Action::SelectImageConsoleField { delta: 1 }),
        Input::BackTab => Some(Action::SelectImageConsoleField { delta: -1 }),
        Input::Left => Some(Action::CycleImageConsoleChoice { backwards: true }),
        Input::Right => Some(Action::CycleImageConsoleChoice { backwards: false }),
        Input::Enter => Some(Action::ConfirmImageConsole),
        Input::Esc => Some(Action::CancelImageConsole),
        Input::Backspace if dialog.selected_field.is_text() => {
            Some(Action::BackspaceImageConsoleField)
        }
        Input::Char(character) if dialog.selected_field.is_text() => {
            Some(Action::AppendImageConsoleField(character))
        }
        Input::Char('h') => Some(Action::CycleImageConsoleChoice { backwards: true }),
        Input::Char('l') => Some(Action::CycleImageConsoleChoice { backwards: false }),
        _ => None,
    }
}

pub fn qemu_launch_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmQemuLaunchInTerminal),
        Input::Esc => Some(Action::CancelQemuLaunchPreview),
        _ => None,
    }
}

pub fn qemu_cancellation_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmQemuSessionCancellation),
        Input::Esc => Some(Action::CancelQemuSessionCancellation),
        _ => None,
    }
}

pub fn layer_tree_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::CtrlU => Some(Action::ClearMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectLayerBrowserEntry { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectLayerBrowserEntry { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectLayerBrowserEntry { delta: 1 }),
        Input::Enter => Some(Action::LayerBrowserEnter),
        Input::Right | Input::Char('l') => Some(Action::LayerBrowserExpand),
        Input::Left | Input::Char('h') => Some(Action::LayerBrowserUp),
        Input::Esc => Some(Action::CloseLayerBrowser),
        Input::Char('r') => Some(Action::RefreshLayerBrowser),
        Input::Char('e') => Some(Action::EditSelectedLayerBrowserFile),
        Input::Char('.') => Some(Action::ToggleLayerBrowserHidden),
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::CtrlU => Some(Action::ClearMetadataQuery),
        Input::Char('i') => Some(Action::SetLayerInspectorMode(LayerInspectorMode::Metadata)),
        Input::Char('[') => Some(Action::ScrollLayerBrowserPreview { delta: -10 }),
        Input::Char(']') => Some(Action::ScrollLayerBrowserPreview { delta: 10 }),
        Input::Char('m') => Some(Action::SetLayerInspectorMode(LayerInspectorMode::Metadata)),
        Input::Char('d') => Some(Action::SetLayerInspectorMode(
            LayerInspectorMode::Dependencies,
        )),
        _ => None,
    }
}
pub fn recipes_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::CtrlU => Some(Action::ClearMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectRecipe { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectRecipe { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectRecipe { delta: 1 }),
        Input::Char('[') => Some(Action::ScrollRecipePreview { delta: -10 }),
        Input::Char(']') => Some(Action::ScrollRecipePreview { delta: 10 }),
        Input::Enter => Some(Action::BeginSelectedRecipeMetadata),
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::CtrlU => Some(Action::ClearMetadataQuery),
        Input::Char('e') => Some(Action::OpenSelectedRecipeProvider),
        Input::Char('o') => Some(Action::BeginSelectedRecipeTaskLog),
        Input::Char('p') => Some(Action::BeginSelectedRecipePatchReview),
        Input::Char('A') => Some(Action::BeginSelectedRecipeDependencies),
        Input::Char('f') => Some(Action::BeginSelectedRecipeForceTask),
        Input::Char('v') => Some(Action::BeginSelectedRecipeDevshell),
        Input::Char('K') => Some(Action::BeginSelectedRecipeDiffconfig),
        Input::Char('z') => Some(Action::BeginSelectedRecipeDiffsigs),
        Input::Char('Z') => Some(Action::BeginSelectedRecipeSignatures),
        Input::Char('V') => Some(Action::BeginSelectedRecipeCveCheck),
        Input::Char('X') => Some(Action::BeginSelectedRecipeSpdx),
        Input::Char('d') => Some(Action::BeginSelectedRecipeDevtoolModify),
        Input::Char('t') => Some(Action::BeginSelectedRecipeDevtoolStatus),
        Input::Char('u') => Some(Action::BeginSelectedRecipeDevtoolUpdateRecipe),
        Input::Char('F') => Some(Action::BeginSelectedRecipeDevtoolFinish),
        Input::Char('P') => Some(Action::BeginSelectedRecipeDevtoolDeploy),
        Input::Char('D') => Some(Action::BeginSelectedRecipeDevtoolReset),
        Input::Char('s') => Some(Action::BeginSelectedRecipeDevtoolWorkspaceShell),
        Input::Char('E') => Some(Action::BeginSelectedRecipeDevtoolEditRecipe),
        _ => None,
    }
}

pub fn terminal_launch_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Left | Input::Char('k') | Input::Char('h') => {
            Some(Action::SelectTerminalLaunchDestination { delta: -1 })
        }
        Input::Down | Input::Right | Input::Char('j') | Input::Char('l') | Input::Tab => {
            Some(Action::SelectTerminalLaunchDestination { delta: 1 })
        }
        Input::Enter => Some(Action::ConfirmTerminalLaunch),
        Input::Esc => Some(Action::CancelTerminalLaunch),
        _ => None,
    }
}

pub fn dtc_compile_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') | Input::BackTab => {
            Some(Action::SelectDtcCompileOption { delta: -1 })
        }
        Input::Down | Input::Char('j') | Input::Tab => {
            Some(Action::SelectDtcCompileOption { delta: 1 })
        }
        Input::Left | Input::Char('h') => Some(Action::AdjustDtcCompileOption { delta: -1 }),
        Input::Right | Input::Char('l') | Input::Char(' ') => {
            Some(Action::AdjustDtcCompileOption { delta: 1 })
        }
        Input::Enter => Some(Action::ConfirmDtcCompileOptions),
        Input::Esc => Some(Action::CancelDtcCompileOptions),
        _ => None,
    }
}

pub fn devtool_modify_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolModify),
        Input::Esc => Some(Action::CancelDevtoolModify),
        _ => None,
    }
}

pub fn devtool_update_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolUpdateRecipe),
        Input::Esc => Some(Action::CancelDevtoolUpdateRecipe),
        _ => None,
    }
}

pub fn devtool_finish_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectDevtoolFinishLayer { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectDevtoolFinishLayer { delta: 1 }),
        Input::Enter => Some(Action::PreviewDevtoolFinish),
        Input::Esc => Some(Action::CancelDevtoolFinish),
        _ => None,
    }
}

pub fn devtool_finish_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolFinish),
        Input::Esc => Some(Action::CancelDevtoolFinishConfirmation),
        _ => None,
    }
}

pub fn devtool_deploy_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendDevtoolDeployTarget(character)),
        Input::Backspace => Some(Action::BackspaceDevtoolDeployTarget),
        Input::Enter => Some(Action::PreviewDevtoolDeploy),
        Input::Esc => Some(Action::CancelDevtoolDeploy),
        _ => None,
    }
}

pub fn devtool_deploy_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolDeploy),
        Input::Esc => Some(Action::CancelDevtoolDeployConfirmation),
        _ => None,
    }
}

pub fn devtool_reset_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolReset),
        Input::Esc => Some(Action::CancelDevtoolReset),
        _ => None,
    }
}

pub fn recipe_editor_action(editor: &yoctui_model::RecipeEditor, key: Input) -> Option<Action> {
    use yoctui_model::{RecipeEditorFocus as Focus, TextAreaMode};

    if editor.searching {
        return match key {
            Input::Char(character) => Some(Action::AppendRecipeEditorSearch(character)),
            Input::Backspace => Some(Action::BackspaceRecipeEditorSearch),
            Input::Enter | Input::Esc => Some(Action::FinishRecipeEditorSearch),
            _ => None,
        };
    }

    if editor.focus == Focus::Files {
        return match key {
            Input::Esc | Input::Char('q') => Some(Action::CloseRecipeEditor),
            Input::Up | Input::Char('k') => Some(Action::SelectRecipeEditorFile { delta: -1 }),
            Input::Down | Input::Char('j') => Some(Action::SelectRecipeEditorFile { delta: 1 }),
            Input::Enter | Input::Tab => Some(Action::FocusRecipeEditor(Focus::Document)),
            Input::Char('e') => Some(Action::OpenRecipeEditorExternal),
            Input::CtrlS => Some(Action::SaveRecipeEditor),
            Input::CtrlB => Some(Action::BeginRecipeEditorBuild),
            _ => None,
        };
    }

    match key {
        Input::CtrlS => Some(Action::SaveRecipeEditor),
        Input::CtrlB => Some(Action::BeginRecipeEditorBuild),
        Input::Char('e') if editor.document.mode() != TextAreaMode::Insert => {
            Some(Action::OpenRecipeEditorExternal)
        }
        Input::Tab | Input::BackTab => Some(Action::FocusRecipeEditor(Focus::Files)),
        Input::Char('/') if editor.document.mode() != TextAreaMode::Insert => {
            Some(Action::BeginRecipeEditorSearch)
        }
        Input::Char('n') if editor.document.mode() != TextAreaMode::Insert => {
            Some(Action::NextRecipeEditorMatch { backwards: false })
        }
        Input::Char('N') if editor.document.mode() != TextAreaMode::Insert => {
            Some(Action::NextRecipeEditorMatch { backwards: true })
        }
        Input::Esc if editor.document.mode() == TextAreaMode::Normal => {
            Some(Action::FocusRecipeEditor(Focus::Files))
        }
        input => {
            popup_editor_action(editor.document.editing, input).and_then(|action| match action {
                Action::EditActivePopup(command) => Some(Action::EditRecipeEditor(command)),
                _ => None,
            })
        }
    }
}

pub fn config_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::CtrlU => Some(Action::ClearMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    if let Some(delta) = collection_scroll_delta(key) {
        return Some(Action::SelectConfigVariable { delta });
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectConfigVariable { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectConfigVariable { delta: 1 }),
        Input::Enter => Some(Action::BeginSelectedConfigDetail),
        Input::Char('C') => Some(Action::CopySelectedConfigEffective),
        Input::Char('U') => Some(Action::CopySelectedConfigUnexpanded),
        Input::Char('s') => Some(Action::OpenConfigScopePicker),
        Input::Char('c') => Some(Action::OpenConfigComparison),
        Input::Char('E') => Some(Action::BeginConfigEdit),
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::CtrlU => Some(Action::ClearMetadataQuery),
        Input::Char('o') => Some(Action::OpenSelectedConfigSource),
        _ => None,
    }
}

pub fn config_source_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectConfigSource { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectConfigSource { delta: 1 }),
        Input::Enter => Some(Action::OpenSelectedConfigSourceChoice),
        Input::Esc => Some(Action::CancelConfigSourcePicker),
        _ => None,
    }
}

pub fn config_scope_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectConfigScope { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectConfigScope { delta: 1 }),
        Input::Enter => Some(Action::ConfirmConfigScope),
        Input::Esc => Some(Action::CancelConfigScopePicker),
        _ => None,
    }
}

pub fn config_compare_dialog_action(key: Input) -> Option<Action> {
    matches!(key, Input::Enter | Input::Esc).then_some(Action::CloseConfigComparison)
}

pub fn config_edit_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendConfigEdit(character)),
        Input::Backspace => Some(Action::BackspaceConfigEdit),
        Input::Enter => Some(Action::PreviewConfigEdit),
        Input::Esc => Some(Action::CancelConfigEdit),
        _ => None,
    }
}

pub fn config_edit_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmConfigEdit),
        Input::Esc => Some(Action::CancelConfigEditConfirmation),
        _ => None,
    }
}
