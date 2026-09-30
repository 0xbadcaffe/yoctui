use super::*;

#[test]
fn rootfs_browser_routes_tree_preview_search_and_images_tabs() {
    use yoctui_model::*;
    let mut app = App::new(20, 2000);
    app.screen = Screen::Images;
    app.images_view = ImagesView::RootfsFilesystem;
    app.focus = FocusTarget::Workspace;
    let image = ImageArtifactIdentity {
        image: "image".into(),
        machine: "qemu".into(),
        path: "/build/image.ext4".into(),
    };
    app.rootfs_composition = RootfsCompositionState::Available {
        request: RootfsCompositionRequest {
            image: image.clone(),
            generation: 1,
        },
        composition: RootfsComposition {
            image,
            root_directory: Some("/build/rootfs".into()),
            installed_packages: RootfsAuthority::Available(Default::default()),
            filesystem_tree: RootfsAuthority::Available(Default::default()),
            system_inventory: RootfsAuthority::Available(Default::default()),
        },
    };
    app.layer_browser = Some(LayerBrowser::new(
        "Rootfs: image".into(),
        "/build/rootfs".into(),
    ));
    assert_eq!(
        rootfs_browser_action(&app, Input::Down),
        Some(Action::SelectLayerBrowserEntry { delta: 1 })
    );
    assert_eq!(
        workspace_collection_action(&app, Input::PageDown),
        Some(Action::SelectLayerBrowserEntry { delta: 10 })
    );
    assert_eq!(
        rootfs_browser_action(&app, Input::Tab),
        Some(Action::ShiftImagesView { delta: 1 })
    );
    assert_eq!(
        rootfs_browser_action(&app, Input::BackTab),
        Some(Action::ShiftImagesView { delta: -1 })
    );
    assert_eq!(
        rootfs_browser_action(&app, Input::Char('1')),
        Some(Action::ShiftImagesView { delta: -2 })
    );
    app.layer_browser.as_mut().unwrap().preview_focused = true;
    assert_eq!(
        rootfs_browser_action(&app, Input::Down),
        Some(Action::ScrollLayerBrowserPreview { delta: 1 })
    );
    assert_eq!(
        rootfs_browser_action(&app, Input::Left),
        Some(Action::FocusLayerBrowserTree)
    );
    app.metadata_searching = true;
    assert_eq!(
        rootfs_browser_action(&app, Input::Char('1')),
        Some(Action::AppendMetadataQuery('1'))
    );
    assert!(workspace_text_input_active(&app));
}
