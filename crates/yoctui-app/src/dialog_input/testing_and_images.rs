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
