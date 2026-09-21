use super::*;

#[test]
fn focus_routes_preserve_search_and_terminal_mode_keys() {
    let mut app = App::new(10, 1024);
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Workspace;
    app.metadata_searching = true;
    for key in [Input::Esc, Input::Char('q'), Input::Tab, Input::Up] {
        assert!(focus_action_for_app(&app, key).is_none());
    }
    app.screen = Screen::TerminalSessions;
    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::Search;
    for key in [Input::Esc, Input::Char('q'), Input::Tab] {
        assert!(focus_action_for_app(&app, key).is_none());
    }
    assert_eq!(
        terminal_workspace_action(&app, Input::Esc),
        Some(Action::TerminalFinishSearch)
    );
}
