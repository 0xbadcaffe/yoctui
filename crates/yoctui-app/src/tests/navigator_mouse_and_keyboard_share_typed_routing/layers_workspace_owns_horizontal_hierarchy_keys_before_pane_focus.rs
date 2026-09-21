use super::*;

#[test]
fn layers_workspace_owns_horizontal_hierarchy_keys_before_pane_focus() {
    let mut app = yoctui_model::App::new(10, 1_000);
    app.screen = Screen::Layers;
    app.focus = FocusTarget::Workspace;
    assert_eq!(focus_action_for_app(&app, Input::Right), None);
    assert_eq!(focus_action_for_app(&app, Input::Left), None);

    let _ = yoctui_model::update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "meta-test".into(),
            root: "/tmp/meta-test".into(),
            directory: "/tmp/meta-test".into(),
            entries: Vec::new(),
        },
    );
    assert_eq!(focus_action_for_app(&app, Input::Right), None);
    assert_eq!(focus_action_for_app(&app, Input::Left), None);
    assert_eq!(
        focus_action_for_app(&app, Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
}
