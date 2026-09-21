use super::*;

#[test]
fn config_edit_preview_shortcut_and_dialog_keys_are_typed() {
    assert_eq!(
        config_workspace_action(false, Input::Char('E')),
        Some(Action::BeginConfigEdit)
    );
    assert_eq!(
        config_edit_dialog_action(Input::Char('x')),
        Some(Action::AppendConfigEdit('x'))
    );
    assert_eq!(
        config_edit_dialog_action(Input::Enter),
        Some(Action::PreviewConfigEdit)
    );
    assert_eq!(
        config_edit_confirmation_action(Input::Enter),
        Some(Action::ConfirmConfigEdit)
    );
}
