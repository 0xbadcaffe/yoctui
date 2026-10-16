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

#[tokio::test]
async fn build_directory_navigator_loads_real_files_and_previews_configuration() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("conf")).unwrap();
    fs::write(
        directory.path().join("conf/local.conf"),
        "MACHINE = \"qemux86-64\"\n",
    )
    .unwrap();
    let mut app = App::new(10, 1000);
    app.workspace.build_dir = Some(directory.path().to_owned());
    app.navigator_selection = App::navigator_selection_for_destination(
        yoctui_model::WorkspaceDestination::BuildDirectory,
    )
    .unwrap();
    let action = pane_focus_route(&app, Input::Enter).unwrap();
    let effect = compatibility_workspace_action(&mut app, action).unwrap();
    let Effect::LoadLayerBrowserDirectory {
        layer,
        root,
        directory: scan,
    } = effect
    else {
        panic!("navigator activation must dispatch a lazy build-directory load");
    };
    assert_eq!(root, directory.path());
    load_layer_browser_directory(&mut app, layer, root, scan).await;
    assert!(app.layer_browser.as_ref().unwrap().is_build_directory());
    assert_eq!(app.layer_browser.as_ref().unwrap().entries.len(), 1);
    let Some(Effect::LoadLayerBrowserDirectory {
        layer,
        root,
        directory: scan,
    }) = compatibility_workspace_action(&mut app, Action::LayerBrowserExpand)
    else {
        panic!("conf expands using the shared Layers tree behavior");
    };
    load_layer_browser_directory(&mut app, layer, root, scan).await;
    let Some(Effect::LoadLayerBrowserPreview(path)) =
        compatibility_workspace_action(&mut app, Action::SelectLayerBrowserEntry { delta: 1 })
    else {
        panic!("selected configuration file must have a preview")
    };
    load_layer_browser_preview(&mut app, path).await;
    assert!(
        app.layer_browser
            .as_ref()
            .unwrap()
            .preview
            .contains("MACHINE = \"qemux86-64\"")
    );
}
