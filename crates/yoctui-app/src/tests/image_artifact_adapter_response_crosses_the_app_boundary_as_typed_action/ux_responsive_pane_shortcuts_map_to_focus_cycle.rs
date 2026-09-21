use super::*;

#[test]
fn ux_responsive_pane_shortcuts_map_to_focus_cycle() {
    assert_eq!(
        key_action(Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
    assert_eq!(
        key_action(Input::BackTab),
        Some(Action::CycleFocus { backwards: true })
    );
}
