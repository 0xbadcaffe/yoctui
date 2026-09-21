use super::*;

#[test]
fn ux_command_center_routes_to_existing_typed_workspaces_and_terminal_command() {
    let mut app = yoctui_model::App::new(16, 4_096);
    for (input, screen) in [
        (Input::F2, Screen::Tasks),
        (Input::F3, Screen::BuildHistory),
        (Input::F8, Screen::Images),
    ] {
        assert_eq!(key_action(input), Some(Action::Open(screen)));
    }
    assert_eq!(
        dashboard_workspace_action(Input::Char('f')),
        Some(Action::OpenRawFavorites)
    );
    let _ = yoctui_model::update(&mut app, Action::OpenRawFavorites);
    assert_eq!(app.screen, Screen::RawMode);
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Favorites);
    assert_eq!(
        dashboard_workspace_action(Input::Char('t')),
        Some(Action::Open(Screen::TerminalSessions))
    );
    assert_eq!(
        context_menu_activation_input("dashboard.errors"),
        Some(Input::Char('e'))
    );
    assert_eq!(
        context_menu_activation_input("dashboard.artifacts"),
        Some(Input::F8)
    );
    assert_eq!(
        context_menu_activation_input("dashboard.favorites"),
        Some(Input::Char('f'))
    );
    assert_eq!(
        context_menu_activation_input("dashboard.terminals"),
        Some(Input::Char('t'))
    );
}
