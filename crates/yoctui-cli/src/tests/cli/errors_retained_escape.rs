use super::*;

#[test]
fn errors_retained_escape_reaches_error_viewer_before_pane_navigation() {
    let mut app = App::new(32, 8192);
    app.screen = Screen::Errors;
    app.focus = yoctui_model::FocusTarget::Workspace;
    app.error_workspace.viewer = Some(yoctui_model::ErrorLogViewer {
        title: "Retained diagnostics".into(),
        path: None,
        content: "error".into(),
        scroll: 0,
        loading: false,
        error: None,
    });
    assert!(workspace_owns_focus_key(&app, Input::Esc));
    assert_eq!(errors_action(&app, Input::Esc), Some(Action::CloseErrorLog));
    update(&mut app, Action::CloseErrorLog);
    assert_eq!(app.focus, yoctui_model::FocusTarget::Workspace);
    assert!(!workspace_owns_focus_key(&app, Input::Esc));
}
