use super::*;

#[test]
fn error_workspace_maps_selection_log_jump_and_source_open() {
    let app = App::new(32, 4096);
    assert_eq!(
        errors_action(&app, Input::Up),
        Some(Action::SelectError { delta: -1 })
    );
    assert_eq!(
        errors_action(&app, Input::Enter),
        Some(Action::OpenSelectedErrorLog)
    );
    assert_eq!(
        errors_action(&app, Input::Char('o')),
        Some(Action::OpenSelectedErrorSource)
    );
    assert_eq!(
        errors_action(&app, Input::Char('l')),
        Some(Action::JumpToSelectedError)
    );
}
