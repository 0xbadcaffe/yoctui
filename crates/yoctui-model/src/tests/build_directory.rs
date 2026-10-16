use super::*;

fn build_app() -> App {
    let mut app = App::new(20, 4096);
    app.workspace.build_dir = Some("/work/poky-build".into());
    app.navigator_selection = NAVIGATOR_COMPATIBILITY_DESTINATIONS
        .iter()
        .position(|destination| *destination == WorkspaceDestination::BuildDirectory)
        .unwrap();
    app
}

#[test]
fn build_directory_navigator_uses_active_build_and_not_source_or_layer() {
    let mut app = build_app();
    app.workspace.source_dir = Some("/work/poky".into());
    let group = NAVIGATOR_GROUPS[app.navigator_group_index()];
    assert_eq!(group.label, "BUILD");
    let effect = update(&mut app, Action::ActivateNavigator);
    assert_eq!(
        effect,
        Some(Effect::LoadLayerBrowserDirectory {
            layer: "Build directory".into(),
            root: "/work/poky-build".into(),
            directory: "/work/poky-build".into(),
        })
    );
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(NAVIGATOR_SCREENS[1], Screen::Tasks);
    assert_eq!(NAVIGATOR_SCREENS[2], Screen::Insights);
}

#[test]
fn build_directory_reuses_tree_expand_preview_and_returns_to_navigator() {
    let mut app = build_app();
    update(&mut app, Action::ActivateNavigator);
    update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "Build directory".into(),
            root: "/work/poky-build".into(),
            directory: "/work/poky-build".into(),
            entries: vec![LayerBrowserEntry {
                path: "/work/poky-build/conf".into(),
                is_dir: true,
                ..Default::default()
            }],
        },
    );
    assert_eq!(
        update(&mut app, Action::LayerBrowserExpand),
        Some(Effect::LoadLayerBrowserDirectory {
            layer: "Build directory".into(),
            root: "/work/poky-build".into(),
            directory: "/work/poky-build/conf".into(),
        })
    );
    update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "Build directory".into(),
            root: "/work/poky-build".into(),
            directory: "/work/poky-build/conf".into(),
            entries: vec![LayerBrowserEntry {
                path: "/work/poky-build/conf/local.conf".into(),
                ..Default::default()
            }],
        },
    );
    assert_eq!(
        update(&mut app, Action::SelectLayerBrowserEntry { delta: 1 }),
        Some(Effect::LoadLayerBrowserPreview(
            "/work/poky-build/conf/local.conf".into()
        ))
    );
    update(&mut app, Action::CloseLayerBrowser);
    assert_eq!(app.focus, FocusTarget::Navigator);
    assert!(app.layer_browser.as_ref().unwrap().is_build_directory());
    assert_eq!(update(&mut app, Action::ActivateNavigator), None);
    assert_eq!(app.layer_browser.as_ref().unwrap().selection, 1);
    update(&mut app, Action::Open(Screen::Layers));
    assert!(
        app.layer_browser.is_none(),
        "F6 Layers must never retain the build tree"
    );
    assert_eq!(
        app.navigator_compatibility_destination(),
        WorkspaceDestination::Layers
    );
}

#[test]
fn build_directory_rejects_unknown_or_relative_build_roots() {
    let mut app = build_app();
    for root in [None, Some(PathBuf::from("relative"))] {
        app.workspace.build_dir = root;
        assert_eq!(update(&mut app, Action::ActivateNavigator), None);
        assert!(
            app.notification
                .as_ref()
                .unwrap()
                .contains("load a build environment")
        );
    }
}
