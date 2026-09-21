use super::*;

#[test]
fn error_workspace_maps_selection_log_jump_and_source_open() {
    assert_eq!(
        errors_action(Input::Up),
        Some(Action::SelectError { delta: -1 })
    );
    assert_eq!(
        errors_action(Input::Enter),
        Some(Action::JumpToSelectedError)
    );
    assert_eq!(
        errors_action(Input::Char('o')),
        Some(Action::OpenSelectedErrorSource)
    );
}
