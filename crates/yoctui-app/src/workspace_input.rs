//! Workspace input.

include!("workspace_input/global_and_logs.rs");

include!("workspace_input/metadata_and_images.rs");

include!("workspace_input/sdk_and_testing.rs");

include!("workspace_input/security_and_qa.rs");

#[cfg(test)]
mod focus_flow_tests {
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
}
