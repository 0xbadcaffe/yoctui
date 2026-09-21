use super::*;

#[test]
fn command_palette_global_shortcut_is_typed() {
    assert_eq!(key_action(Input::CtrlP), Some(Action::OpenCommandPalette));
    assert_eq!(focus_action(FocusTarget::CommandPalette, Input::Tab), None);
}
