use super::*;

#[test]
fn focus_activation_enters_actionable_workspace_but_keeps_passive_dashboard_out() {
    let mut app = App::new(10, 1024);
    for screen in [Screen::Dashboard, Screen::Recipes, Screen::Tasks] {
        app.navigator_selection = NAVIGATOR_SCREENS.iter().position(|s| *s == screen).unwrap();
        update(&mut app, Action::ActivateNavigator);
        assert_eq!(
            app.focus,
            if screen == Screen::Dashboard {
                FocusTarget::Navigator
            } else {
                FocusTarget::Workspace
            }
        );
    }
}
