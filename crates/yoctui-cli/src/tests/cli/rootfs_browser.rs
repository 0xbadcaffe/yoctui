use super::*;

#[tokio::test]
async fn rootfs_browser_effects_load_nested_tree_preview_and_report_cleaned_root() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-cli-rootfs-browser-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("etc")).unwrap();
    fs::write(root.join("etc/os-release"), "NAME=fixture\n").unwrap();
    let root = fs::canonicalize(root).unwrap();
    let mut app = App::new(20, 2000);
    app.screen = Screen::Images;
    let layer = "Rootfs: fixture".to_owned();
    load_layer_browser_directory(&mut app, layer.clone(), root.clone(), root.clone()).await;
    assert!(
        app.layer_browser
            .as_ref()
            .unwrap()
            .selected_entry()
            .unwrap()
            .is_dir
    );
    let Some(Effect::LoadLayerBrowserDirectory { directory, .. }) =
        update(&mut app, Action::LayerBrowserExpand)
    else {
        panic!("expected lazy directory load")
    };
    load_layer_browser_directory(&mut app, layer.clone(), root.clone(), directory).await;
    let Some(Effect::LoadLayerBrowserPreview(path)) =
        update(&mut app, Action::SelectLayerBrowserEntry { delta: 1 })
    else {
        panic!("expected file preview")
    };
    load_layer_browser_preview(&mut app, path).await;
    assert_eq!(
        app.layer_browser.as_ref().unwrap().preview,
        "NAME=fixture\n"
    );
    assert!(
        app.layer_browser
            .as_ref()
            .unwrap()
            .selected_entry()
            .unwrap()
            .rootfs_metadata
            .is_some()
    );
    assert_eq!(app.screen, Screen::Images);
    fs::remove_dir_all(&root).unwrap();
    load_layer_browser_directory(&mut app, layer, root.clone(), root).await;
    assert!(
        app.notification
            .as_ref()
            .unwrap()
            .contains("Could not read")
    );
}
