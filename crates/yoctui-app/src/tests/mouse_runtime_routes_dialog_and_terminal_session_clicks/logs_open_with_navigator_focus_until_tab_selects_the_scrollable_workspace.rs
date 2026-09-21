use super::*;

#[test]
fn logs_open_with_navigator_focus_until_tab_selects_the_scrollable_workspace() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.focus = FocusTarget::Navigator;
    let _ = yoctui_model::update(&mut app, Action::Open(Screen::Logs));
    assert_eq!(app.focus, FocusTarget::Navigator);
    let tab = focus_action_for_app(&app, Input::Tab).unwrap();
    let _ = yoctui_model::update(&mut app, tab);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(
        focus_action_for_app(&app, Input::PageUp),
        None,
        "workspace-owned log scrolling must reach the collection router"
    );
    assert_eq!(
        workspace_collection_action(&app, Input::PageUp),
        Some(Action::ScrollLogs { delta: 10 })
    );
}
