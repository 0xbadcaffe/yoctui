use super::*;

#[test]
fn responsive_pane_focus_cycle_cannot_escape_modal_focus() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Dialog;
    let _ = update(&mut app, Action::CycleFocus { backwards: false });
    assert_eq!(app.focus, FocusTarget::Dialog);

    app.focus = FocusTarget::CommandPalette;
    let _ = update(&mut app, Action::CycleFocus { backwards: true });
    assert_eq!(app.focus, FocusTarget::CommandPalette);

    app.focus = FocusTarget::Workspace;
    let _ = update(&mut app, Action::CycleFocus { backwards: false });
    assert_eq!(app.focus, FocusTarget::Navigator);
    let _ = update(&mut app, Action::CycleFocus { backwards: false });
    assert_eq!(app.focus, FocusTarget::Workspace);
}
