use super::*;

#[test]
fn maps_log_controls() {
    assert_eq!(key_action(Input::Char('f')), Some(Action::ToggleLogFollow));
    assert_eq!(key_action(Input::Char('w')), Some(Action::ToggleLogWrap));
    assert_eq!(key_action(Input::Up), Some(Action::ScrollLogs { delta: 1 }));
}
