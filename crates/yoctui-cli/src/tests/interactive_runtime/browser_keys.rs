use super::*;

#[test]
fn browser_global_keys_fall_through_and_edges_use_shared_mapping() {
    let mut app = App::new(100, 4096);
    app.screen = Screen::Layers;
    for input in [Input::F4, Input::F12, Input::Char('q')] {
        assert!(browser_workspace_action(&app, false, input).is_none());
    }
    assert_eq!(
        browser_workspace_action(&app, false, Input::End),
        Some(Action::SelectLayerBrowserEntry { delta: isize::MAX })
    );
    assert_eq!(
        browser_workspace_action(&app, true, Input::Home),
        Some(Action::ScrollLayerBrowserPreview { delta: isize::MIN })
    );
}
