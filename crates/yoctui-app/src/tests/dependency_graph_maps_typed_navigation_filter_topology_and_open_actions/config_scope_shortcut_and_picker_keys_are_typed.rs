use super::*;

#[test]
fn config_scope_shortcut_and_picker_keys_are_typed() {
    assert_eq!(
        config_workspace_action(false, Input::Char('s')),
        Some(Action::OpenConfigScopePicker)
    );
    assert_eq!(
        config_scope_picker_action(Input::Down),
        Some(Action::SelectConfigScope { delta: 1 })
    );
    assert_eq!(
        config_scope_picker_action(Input::Enter),
        Some(Action::ConfirmConfigScope)
    );
    assert_eq!(
        config_scope_picker_action(Input::Esc),
        Some(Action::CancelConfigScopePicker)
    );
}
