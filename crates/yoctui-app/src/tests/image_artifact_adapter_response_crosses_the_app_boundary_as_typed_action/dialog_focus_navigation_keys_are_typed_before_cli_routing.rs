use super::*;

#[test]
fn dialog_focus_navigation_keys_are_typed_before_cli_routing() {
    assert_eq!(
        key_action(Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
    assert_eq!(
        key_action(Input::BackTab),
        Some(Action::CycleFocus { backwards: true })
    );
    assert_eq!(
        key_action(Input::Esc),
        Some(Action::Open(Screen::Dashboard))
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Up),
        Some(Action::SelectNavigator { delta: -1 })
    );
    assert_eq!(
        focus_action(FocusTarget::Inspector, Input::Up),
        None,
        "inspector arrows must not leak into workspace actions"
    );
    for focus in [FocusTarget::Workspace, FocusTarget::Inspector] {
        assert_eq!(focus_action(focus, Input::Left), None);
        assert_eq!(focus_action(focus, Input::Right), None);
    }
    assert_eq!(
        focus_action(FocusTarget::Dialog, Input::Tab),
        None,
        "modal input is handled only by the active dialog"
    );
}
