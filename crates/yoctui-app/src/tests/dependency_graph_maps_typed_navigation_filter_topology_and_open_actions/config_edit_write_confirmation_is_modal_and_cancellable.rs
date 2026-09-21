use super::*;

#[test]
fn config_edit_write_confirmation_is_modal_and_cancellable() {
    assert_eq!(
        config_edit_confirmation_action(Input::Enter),
        Some(Action::ConfirmConfigEdit)
    );
    assert_eq!(
        config_edit_confirmation_action(Input::Esc),
        Some(Action::CancelConfigEditConfirmation)
    );
    assert_eq!(config_edit_confirmation_action(Input::Char('E')), None);
}
