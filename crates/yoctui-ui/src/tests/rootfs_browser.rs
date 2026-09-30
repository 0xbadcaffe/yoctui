use super::*;

#[test]
fn rootfs_browser_renders_inline_tree_attributes_and_numbered_content_responsively() {
    let mut app = ux_rootfs_ui_app();
    app.images_view = ImagesView::RootfsFilesystem;
    if let RootfsCompositionState::Partial { composition, .. } = &mut app.rootfs_composition {
        composition.root_directory = Some("/build/rootfs".into());
    }
    let mut browser =
        LayerBrowser::new("Rootfs: core-image-minimal".into(), "/build/rootfs".into());
    browser.entries = vec![LayerBrowserEntry {
        path: "/build/rootfs/config".into(),
        size: Some(13),
        rootfs_metadata: Some(yoctui_model::RootfsFileMetadata {
            kind: RootfsEntryKind::RegularFile,
            mode: 0o644,
            uid: 0,
            gid: 0,
            owner: Some("root".into()),
            group: Some("root".into()),
            link_target: None,
        }),
        ..Default::default()
    }];
    browser.preview = "NAME=fixture\nSECOND=line\n".into();
    browser.preview_kind = PreviewKind::Text;
    app.layer_browser = Some(browser);
    for (width, height) in [(200, 50), (140, 40), (100, 30), (80, 24)] {
        let text = rendered_text(&app, width, height);
        for anchor in ["config", "File preview", "NAME=fixture", "0644", "root(0)"] {
            assert!(
                text.contains(anchor),
                "{width}x{height} missing {anchor}: {text}"
            );
        }
        assert!(!text.contains("Configured layers"), "{text}");
        assert!(!text.contains("Logical path"), "{text}");
    }
    let text = rendered_text(&app, 200, 50);
    assert!(text.contains("-rw-r--r--"), "{text}");
    assert!(text.contains("13 B"), "{text}");
    assert!(text.contains("Host IMAGE_ROOTFS attributes"), "{text}");
    app.preferences.symbols = SymbolPreference::Ascii;
    for (width, height) in [(60, 20), (30, 10), (1, 1)] {
        let _ = rendered_text(&app, width, height);
    }
    app.images_view = ImagesView::RootfsPackages;
    let packages = rendered_text(&app, 200, 50);
    assert!(packages.contains("Exact bytes"), "{packages}");
    assert!(
        !packages.contains("Host IMAGE_ROOTFS attributes"),
        "{packages}"
    );
}

#[test]
fn rootfs_browser_symlink_and_binary_previews_never_render_stale_content() {
    let mut app = ux_rootfs_ui_app();
    app.images_view = ImagesView::RootfsFilesystem;
    if let RootfsCompositionState::Partial { composition, .. } = &mut app.rootfs_composition {
        composition.root_directory = Some("/build/rootfs".into());
    }
    let mut browser =
        LayerBrowser::new("Rootfs: core-image-minimal".into(), "/build/rootfs".into());
    browser.entries = vec![LayerBrowserEntry {
        path: "/build/rootfs/link".into(),
        rootfs_metadata: Some(yoctui_model::RootfsFileMetadata {
            kind: RootfsEntryKind::Symlink,
            mode: 0o777,
            uid: 42,
            gid: 43,
            owner: None,
            group: None,
            link_target: Some("/outside".into()),
        }),
        ..Default::default()
    }];
    browser.preview = "SECRET stale text".into();
    browser.preview_kind = PreviewKind::Text;
    app.layer_browser = Some(browser);
    let text = rendered_text(&app, 200, 50);
    assert!(text.contains("not read"), "{text}");
    assert!(text.contains("/outside"), "{text}");
    assert!(!text.contains("SECRET"), "{text}");
    let browser = app.layer_browser.as_mut().unwrap();
    browser.entries[0].rootfs_metadata.as_mut().unwrap().kind = RootfsEntryKind::RegularFile;
    browser.preview_kind = PreviewKind::Binary;
    let text = rendered_text(&app, 200, 50);
    assert!(text.contains("Binary preview unavailable"), "{text}");
    assert!(!text.contains("SECRET"), "{text}");
}
