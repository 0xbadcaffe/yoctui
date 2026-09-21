use super::*;

#[test]
fn enter_activates_contextual_notification() {
    assert_eq!(key_action(Input::Enter), Some(Action::ActivateNotification));
}
