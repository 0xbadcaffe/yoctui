use super::*;

#[test]
fn config_copy_shortcuts_are_typed() {
    assert_eq!(
        config_workspace_action(false, Input::Char('C')),
        Some(Action::CopySelectedConfigEffective)
    );
    assert_eq!(
        config_workspace_action(false, Input::Char('U')),
        Some(Action::CopySelectedConfigUnexpanded)
    );
}
