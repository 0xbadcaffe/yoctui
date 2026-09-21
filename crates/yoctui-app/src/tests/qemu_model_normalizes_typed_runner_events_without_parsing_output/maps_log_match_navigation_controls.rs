use super::*;

#[test]
fn maps_log_match_navigation_controls() {
    assert_eq!(key_action(Input::Char('n')), Some(Action::NextLogMatch));
    assert_eq!(key_action(Input::Char('N')), Some(Action::PreviousLogMatch));
}
