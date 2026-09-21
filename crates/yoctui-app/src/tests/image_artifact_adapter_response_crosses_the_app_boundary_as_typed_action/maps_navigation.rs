use super::*;

#[test]
fn maps_navigation() {
    assert_eq!(
        key_action(Input::Char('l')),
        Some(Action::Open(Screen::Logs))
    );
    assert_eq!(
        key_action(Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
    assert_eq!(key_action(Input::F5), Some(Action::Open(Screen::Logs)));
    assert_eq!(
        key_action(Input::Char('x')),
        Some(Action::Open(Screen::Bbmask))
    );
}
