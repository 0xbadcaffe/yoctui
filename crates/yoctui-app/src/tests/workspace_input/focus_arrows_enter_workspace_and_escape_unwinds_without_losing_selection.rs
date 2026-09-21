use super::*;

#[test]
fn focus_arrows_enter_workspace_and_escape_unwinds_without_losing_selection() {
    let mut app = App::new(10, 1024);
    yoctui_model::update(&mut app, Action::Open(Screen::Recipes));
    for key in [Input::Right, Input::Esc, Input::Enter] {
        let action = focus_action_for_app(&app, key).unwrap();
        yoctui_model::update(&mut app, action);
    }
    assert_eq!(app.screen, Screen::Recipes);
    assert_eq!(app.focus, FocusTarget::Workspace);
    let selected = app.navigator_selection;
    yoctui_model::update(
        &mut app,
        focus_action(FocusTarget::Workspace, Input::Esc).unwrap(),
    );
    assert_eq!(app.focus, FocusTarget::Navigator);
    assert_eq!(app.navigator_selection, selected);
    yoctui_model::update(
        &mut app,
        focus_action(FocusTarget::Navigator, Input::Esc).unwrap(),
    );
    assert_eq!(app.screen, Screen::Dashboard);
}
