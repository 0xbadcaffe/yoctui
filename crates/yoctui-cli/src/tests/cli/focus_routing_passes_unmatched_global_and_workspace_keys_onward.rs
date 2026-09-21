use super::*;

#[test]
fn focus_routing_passes_unmatched_global_and_workspace_keys_onward() {
    use yoctui_model::FocusTarget;

    for focus in [
        FocusTarget::Navigator,
        FocusTarget::Workspace,
        FocusTarget::Inspector,
    ] {
        let mut app = App::new(10, 1_000);
        app.focus = focus;
        for input in [Input::CtrlP, Input::Char('?'), Input::F5, Input::Char('r')] {
            assert!(pane_focus_route(&app, input).is_none());
        }
        assert!(matches!(
            pane_focus_route(&app, Input::Tab),
            Some(Action::CycleFocus { backwards: false })
        ));
        assert!(matches!(
            pane_focus_route(&app, Input::Char('q')),
            Some(Action::Quit)
        ));
    }
    let mut navigator = App::new(10, 1_000);
    navigator.focus = FocusTarget::Navigator;
    assert!(matches!(
        pane_focus_route(&navigator, Input::Down),
        Some(Action::SelectNavigator { delta: 1 })
    ));
    let mut workspace = App::new(10, 1_000);
    workspace.focus = FocusTarget::Workspace;
    assert!(pane_focus_route(&workspace, Input::Down).is_none());
}

#[test]
fn layer_browser_escape_runs_before_pane_focus() {
    use yoctui_model::FocusTarget;

    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Workspace;
    app.screen = Screen::Layers;
    app.layer_browser = Some(yoctui_model::LayerBrowser::new(
        "romulus-layer".into(),
        "/tmp/romulus-layer".into(),
    ));
    assert!(workspace_owns_focus_key(&app, Input::Esc));
    assert_eq!(
        layer_tree_action(false, Input::Esc),
        Some(Action::CloseLayerBrowser)
    );
}

#[test]
fn image_tabs_run_before_pane_focus() {
    use yoctui_model::FocusTarget;

    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Workspace;
    app.screen = Screen::Images;
    assert!(workspace_owns_focus_key(&app, Input::Tab));
    assert!(workspace_owns_focus_key(&app, Input::BackTab));
}
