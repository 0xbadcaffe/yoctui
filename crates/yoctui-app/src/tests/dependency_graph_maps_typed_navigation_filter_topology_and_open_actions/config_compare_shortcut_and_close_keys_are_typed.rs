use super::*;

#[test]
fn config_compare_shortcut_and_close_keys_are_typed() {
    assert_eq!(
        config_workspace_action(false, Input::Char('c')),
        Some(Action::OpenConfigComparison)
    );
    assert_eq!(
        config_compare_dialog_action(Input::Enter),
        Some(Action::CloseConfigComparison)
    );
    assert_eq!(
        config_compare_dialog_action(Input::Esc),
        Some(Action::CloseConfigComparison)
    );
}
