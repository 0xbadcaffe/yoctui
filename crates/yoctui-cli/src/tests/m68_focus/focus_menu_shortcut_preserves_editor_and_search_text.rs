use super::*;

#[test]
fn focus_menu_shortcut_preserves_editor_and_search_text() {
    let mut app = App::new(10, 1024);
    app.screen = Screen::Recipes;
    app.focus = yoctui_model::FocusTarget::Workspace;
    app.metadata_searching = true;
    assert!(direct_menu_shortcut_action(&app, Input::Char('a'), false).is_none());
    assert!(pane_focus_route(&app, Input::Char('q')).is_none());
    assert_eq!(
        direct_menu_shortcut_action(&app, Input::F10, false),
        Some(Action::OpenApplicationMenu)
    );
}
