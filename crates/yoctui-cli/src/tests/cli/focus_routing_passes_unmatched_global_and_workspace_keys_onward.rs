use super::*;

#[test]
fn bitbake_setup_catalogue_escape_returns_to_profile_before_navigator() {
    let mut app = App::new(8, 1024);
    app.screen = Screen::BuildEnvironment;
    app.focus = yoctui_model::FocusTarget::Workspace;
    app.bitbake_setup.catalogue = true;
    assert!(workspace_owns_focus_key(&app, Input::Esc));
    assert_eq!(
        yoctui_app::bitbake_setup_action(&app, Input::Esc),
        Some(Action::BitBakeSetup(
            yoctui_model::BitBakeSetupAction::Environment
        ))
    );
    app.bitbake_setup.catalogue = false;
    assert!(!workspace_owns_focus_key(&app, Input::Esc));
}

#[test]
fn raw_execution_escape_uses_workspace_route_so_detach_effect_is_submitted() {
    let mut app = App::new(8, 1024);
    app.screen = Screen::RawMode;
    app.raw_mode.view = yoctui_model::RawModeView::Execution;
    for focus in [
        yoctui_model::FocusTarget::Workspace,
        yoctui_model::FocusTarget::Navigator,
    ] {
        app.focus = focus;
        assert!(workspace_owns_focus_key(&app, Input::Esc));
        assert_eq!(
            raw_mode_input(&app, Input::Esc),
            Some(yoctui_model::RawModeAction::CloseExecution)
        );
    }
}

#[test]
fn raw_history_favorites_and_filter_escape_stay_local_before_navigator() {
    let mut app = App::new(8, 1024);
    app.screen = Screen::RawMode;
    app.focus = yoctui_model::FocusTarget::Workspace;
    for view in [
        yoctui_model::RawModeView::History,
        yoctui_model::RawModeView::Favorites,
    ] {
        app.raw_mode.view = view;
        assert!(workspace_owns_focus_key(&app, Input::Esc));
        assert!(pane_focus_route(&app, Input::Esc).is_none());
        assert_eq!(
            raw_mode_input(&app, Input::Esc),
            Some(yoctui_model::RawModeAction::Back)
        );
    }
    app.raw_mode.view = yoctui_model::RawModeView::Browser;
    app.raw_mode.search.query = "--version".into();
    assert!(workspace_owns_focus_key(&app, Input::Esc));
    assert_eq!(
        raw_mode_input(&app, Input::Esc),
        Some(yoctui_model::RawModeAction::ClearSearch)
    );
    app.raw_mode.search.query.clear();
    app.raw_mode.browser_column = yoctui_model::RawBrowserColumn::Categories;
    assert!(!workspace_owns_focus_key(&app, Input::Esc));
}

#[test]
fn testing_local_tabs_back_and_import_win_only_in_workspace() {
    use yoctui_model::FocusTarget;
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Testing;
    app.focus = FocusTarget::Workspace;
    app.test_view = TestWorkspaceView::Results;
    app.test_result_drilled = true;
    assert!(workspace_owns_focus_key(&app, Input::Tab));
    assert!(workspace_owns_focus_key(&app, Input::Esc));
    assert_eq!(
        testing_screen_action(&app, Input::Esc),
        Some(Action::LeaveTestResultCases)
    );
    assert_eq!(
        yoctui_app::inspector_toggle_action(&app, Input::Alt('i')),
        None
    );
    assert_eq!(
        testing_screen_action(&app, Input::Alt('i')),
        Some(Action::BeginTestResultImport)
    );
    app.focus = FocusTarget::Navigator;
    assert!(!workspace_owns_focus_key(&app, Input::Tab));
    assert!(!workspace_owns_focus_key(&app, Input::Esc));
    assert_eq!(
        yoctui_app::inspector_toggle_action(&app, Input::Alt('i')),
        Some(Action::ToggleInspector)
    );
}

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

#[test]
fn platform_tabs_run_before_pane_focus() {
    use yoctui_model::FocusTarget;

    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Workspace;
    for screen in [Screen::Kernel, Screen::Firmware] {
        app.screen = screen;
        assert!(workspace_owns_focus_key(&app, Input::Tab));
        assert!(workspace_owns_focus_key(&app, Input::BackTab));
    }
}
