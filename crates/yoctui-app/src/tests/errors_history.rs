use super::*;

#[test]
fn errors_history_routes_tabs_viewer_and_resolved_cleanup_keys() {
    let mut app = yoctui_model::App::new(32, 4096);
    assert_eq!(
        errors_action(&app, Input::Char('2')),
        Some(Action::SetErrorWorkspaceView(
            yoctui_model::ErrorWorkspaceView::History
        ))
    );
    app.error_workspace.view = yoctui_model::ErrorWorkspaceView::History;
    assert_eq!(
        errors_action(&app, Input::Down),
        Some(Action::SelectHistoricalError { delta: 1 })
    );
    assert_eq!(
        errors_action(&app, Input::Delete),
        Some(Action::RequestResolvedBuildRemoval)
    );
    app.error_workspace.viewer = Some(yoctui_model::ErrorLogViewer {
        title: "log".into(),
        path: None,
        content: "line".into(),
        scroll: 0,
        loading: false,
        error: None,
    });
    assert_eq!(
        errors_action(&app, Input::PageDown),
        Some(Action::ScrollErrorLog { delta: 20 })
    );
    assert_eq!(errors_action(&app, Input::Esc), Some(Action::CloseErrorLog));
    app.screen = yoctui_model::Screen::Errors;
    app.focus = FocusTarget::Workspace;
    assert_eq!(
        focus_action_for_app(&app, Input::Esc),
        Some(Action::CloseErrorLog)
    );
    let _ = yoctui_model::update(&mut app, Action::CloseErrorLog);
    assert!(app.error_workspace.viewer.is_none());
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(
        app.error_workspace.view,
        yoctui_model::ErrorWorkspaceView::History
    );
    assert_eq!(
        focus_action_for_app(&app, Input::Esc),
        Some(Action::Focus(FocusTarget::Navigator))
    );
}

#[test]
fn errors_resolved_cleanup_confirmation_traps_yes_and_no() {
    assert_eq!(
        resolved_build_removal_confirmation_action(Input::Enter),
        Some(Action::ConfirmResolvedBuildRemoval)
    );
    assert_eq!(
        resolved_build_removal_confirmation_action(Input::Esc),
        Some(Action::CancelResolvedBuildRemoval)
    );
}
