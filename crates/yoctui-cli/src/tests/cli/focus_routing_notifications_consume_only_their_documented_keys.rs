use super::*;

#[test]
fn focus_routing_notifications_consume_only_their_documented_keys() {
    assert!(matches!(
        notification_input_action(true, true, false, Input::Enter),
        Some(Action::ActivateNotification)
    ));
    assert!(matches!(
        notification_input_action(true, false, false, Input::Esc),
        Some(Action::DismissNotification)
    ));
    assert!(notification_input_action(true, false, false, Input::Enter).is_none());
    for input in [Input::CtrlP, Input::Char('?'), Input::F5, Input::Char('r')] {
        assert!(notification_input_action(true, false, false, input).is_none());
    }
    assert!(notification_input_action(true, false, true, Input::Char('r')).is_none());
}
