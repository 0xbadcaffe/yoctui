use super::*;

#[test]
fn navigator_selection_and_focus_cycle_are_bounded() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::Focus(FocusTarget::Navigator));
    let _ = update(&mut app, Action::SelectNavigator { delta: 100 });
    assert_eq!(app.navigator_selection, NAVIGATOR_SCREENS.len() - 1);
    let _ = update(&mut app, Action::ActivateNavigator);
    assert_eq!(app.screen, Screen::Settings);
    assert_eq!(app.focus, FocusTarget::Workspace);
    let _ = update(&mut app, Action::CycleFocus { backwards: false });
    assert_eq!(app.focus, FocusTarget::Navigator);
    let _ = update(&mut app, Action::CycleFocus { backwards: true });
    assert_eq!(app.focus, FocusTarget::Workspace);
}
