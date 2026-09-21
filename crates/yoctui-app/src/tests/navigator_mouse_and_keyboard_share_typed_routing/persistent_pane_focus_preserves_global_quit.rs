use super::*;

#[test]
fn persistent_pane_focus_preserves_global_quit() {
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Char('q')),
        Some(Action::Quit)
    );
    assert_eq!(
        focus_action(FocusTarget::Workspace, Input::Char('q')),
        Some(Action::Quit)
    );
    assert_eq!(
        focus_action(FocusTarget::Workspace, Input::CtrlC),
        Some(Action::Quit)
    );
    assert_eq!(
        focus_action(FocusTarget::Inspector, Input::CtrlC),
        Some(Action::Quit)
    );
}
