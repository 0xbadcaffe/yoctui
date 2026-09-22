use super::*;

#[test]
fn command_palette_global_shortcut_is_typed() {
    assert_eq!(key_action(Input::CtrlP), Some(Action::OpenCommandPalette));
    assert_eq!(focus_action(FocusTarget::CommandPalette, Input::Tab), None);
}

#[test]
fn command_palette_pages_move_ten_results() {
    assert_eq!(
        command_palette_navigation_action(Input::PageUp),
        Some(Action::SelectCommandPalette { delta: -10 })
    );
    assert_eq!(
        command_palette_navigation_action(Input::PageDown),
        Some(Action::SelectCommandPalette { delta: 10 })
    );
}
