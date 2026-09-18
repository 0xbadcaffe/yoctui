use super::*;

#[test]
fn layers_list_enter_right_and_l_open_the_selected_hierarchy() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Layers;
    for input in [Input::Enter, Input::Right, Input::Char('l')] {
        assert_eq!(
            layer_list_open_action(&app, input),
            Some(Action::BeginSelectedLayerBrowser)
        );
    }
    assert!(layer_list_open_action(&app, Input::Left).is_none());
    app.layer_browser = Some(yoctui_model::LayerBrowser::new(
        "meta-test".into(),
        "/tmp/meta-test".into(),
    ));
    assert!(layer_list_open_action(&app, Input::Enter).is_none());
}
