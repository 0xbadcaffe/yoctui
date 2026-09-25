use super::*;

#[test]
fn devtool_workspace_is_a_distinct_navigator_screen_with_devtool_authority() {
    assert_eq!(NAVIGATOR_SCREENS[3], Screen::Recipes);
    assert_eq!(NAVIGATOR_SCREENS[19], Screen::Devtool);
    assert_eq!(
        workspace_screen_destination(Screen::Devtool),
        WorkspaceDestination::Devtool
    );

    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::Open(Screen::Devtool));
    assert_eq!(app.screen, Screen::Devtool);
    app.focus = FocusTarget::Workspace;
    assert_eq!(app.inspector_mode(), InspectorMode::Recipe);
}
