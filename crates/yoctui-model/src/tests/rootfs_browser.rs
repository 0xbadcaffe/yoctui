use super::*;

#[test]
fn rootfs_browser_opens_on_files_tab_and_correlated_composition_completion() {
    let mut app = App::new(20, 2000);
    app.screen = Screen::Images;
    app.images_view = ImagesView::RootfsFilesystem;
    let image = ImageArtifactIdentity {
        image: "image".into(),
        machine: "qemu".into(),
        path: "/build/image.ext4".into(),
    };
    let request = RootfsCompositionRequest {
        image: image.clone(),
        generation: 1,
    };
    app.image_artifact_selection = Some(image.clone());
    app.workspace.recipes.push(Recipe {
        name: "image".into(),
        ..Default::default()
    });
    app.image_artifacts = ImageArtifactInventoryState::Available {
        request: ImageArtifactRequest {
            generation: 1,
            machine: "qemu".into(),
        },
        inventory: ImageArtifactInventory {
            machine: "qemu".into(),
            deploy_directory: ImageArtifactField::Available("/build".into()),
            artifacts: Vec::new(),
        },
    };
    let composition = RootfsComposition {
        image,
        root_directory: Some("/build/rootfs".into()),
        installed_packages: RootfsAuthority::Available(Default::default()),
        filesystem_tree: RootfsAuthority::Available(RootfsFilesystemTree {
            entries: vec![RootfsEntry {
                identity: RootfsPathIdentity("/".into()),
                kind: RootfsEntryKind::Directory,
                size_bytes: 0,
                package: None,
            }],
        }),
        system_inventory: RootfsAuthority::Available(Default::default()),
    };
    app.rootfs_composition = RootfsCompositionState::Loading {
        request: request.clone(),
    };
    let effect = update(
        &mut app,
        Action::RootfsCompositionLoaded {
            request: request.clone(),
            composition,
        },
    );
    assert!(
        matches!(effect, Some(Effect::LoadLayerBrowserDirectory { root, directory, .. }) if root == std::path::Path::new("/build/rootfs") && directory == root)
    );
    app.layer_browser = Some(LayerBrowser::new(
        "Rootfs: image".into(),
        "/build/rootfs".into(),
    ));
    app.layer_browser
        .as_mut()
        .unwrap()
        .entries
        .push(LayerBrowserEntry {
            path: "/build/rootfs/config".into(),
            rootfs_metadata: Some(attributes(RootfsEntryKind::RegularFile, 0o644)),
            ..Default::default()
        });
    update(&mut app, Action::BeginMetadataSearch);
    assert!(matches!(
        update(&mut app, Action::AppendMetadataQuery('c')),
        Some(Effect::LoadLayerBrowserPreview(_))
    ));
    app.layer_browser.as_mut().unwrap().preview = "old file content".into();
    assert!(update(&mut app, Action::AppendMetadataQuery('z')).is_none());
    assert!(app.layer_browser.as_ref().unwrap().preview.is_empty());
    assert!(matches!(
        update(&mut app, Action::ClearMetadataQuery),
        Some(Effect::LoadLayerBrowserPreview(_))
    ));
    app.metadata_query = "old search".into();
    update(&mut app, Action::ShiftImagesView { delta: -1 });
    assert_eq!(app.images_view, ImagesView::RootfsPackages);
    assert!(app.layer_browser.is_none());
    assert!(app.metadata_query.is_empty());
    assert!(matches!(
        update(&mut app, Action::ShiftImagesView { delta: 1 }),
        Some(Effect::LoadLayerBrowserDirectory { .. })
    ));
    let stale = RootfsCompositionRequest {
        generation: 99,
        ..request
    };
    let composition = app.rootfs_composition.composition().unwrap().clone();
    assert!(
        update(
            &mut app,
            Action::RootfsCompositionLoaded {
                request: stale,
                composition
            }
        )
        .is_none()
    );
}

fn attributes(kind: RootfsEntryKind, mode: u32) -> RootfsFileMetadata {
    RootfsFileMetadata {
        kind,
        mode: Some(mode),
        uid: Some(123),
        gid: Some(456),
        owner: None,
        group: None,
        link_target: None,
    }
}

#[test]
fn rootfs_browser_modes_cover_special_bits_and_unknown_accounts() {
    for (mode, expected) in [
        (0o4755, "-rwsr-xr-x"),
        (0o2644, "-rw-r-Sr--"),
        (0o1700, "-rwx-----T"),
        (0o1777, "-rwxrwxrwt"),
    ] {
        assert_eq!(
            attributes(RootfsEntryKind::RegularFile, mode).permissions(),
            expected
        );
    }
    assert!(
        attributes(RootfsEntryKind::Directory, 0o755)
            .listing(Some(4096))
            .contains("0755 123 456 4096 B")
    );
    let mut unknown = attributes(RootfsEntryKind::RegularFile, 0o644);
    unknown.mode = None;
    unknown.uid = None;
    unknown.gid = None;
    assert_eq!(unknown.permissions(), "??????????");
    assert_eq!(
        unknown.listing(Some(13)),
        "?????????? mode unavailable unavailable unavailable 13 B"
    );
    unknown.owner = Some("build-user".into());
    assert!(!unknown.listing(None).contains("build-user"));
}

#[test]
fn rootfs_systemd_selection_is_bounded_through_scroll_pages_and_inventory_shrink() {
    let mut app = App::new(20, 2000);
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
            system_inventory: RootfsAuthority::Available(RootfsSystemInventory {
                systemd_services: (0..80)
                    .map(|index| RootfsSystemdService {
                        name: format!("service-{index}.service"),
                        logical_path: RootfsPathIdentity(
                            format!("/usr/lib/systemd/system/service-{index}.service").into(),
                        ),
                        host_path: "/build/rootfs/service".into(),
                        description: None,
                        bus_name: None,
                        enabled_by: vec![],
                        preview: String::new(),
                        preview_truncated: false,
                    })
                    .collect(),
                ..Default::default()
            }),
        },
    };
    for (delta, selected) in [
        (10, 10),
        (10, 20),
        (-1, 19),
        (isize::MAX, 79),
        (1, 79),
        (-10, 69),
        (isize::MIN, 0),
        (-1, 0),
    ] {
        assert!(update(&mut app, Action::SelectRootfsSystemdService { delta }).is_none());
        assert_eq!(app.rootfs_systemd_selection, selected);
    }
    if let RootfsCompositionState::Available { composition, .. } = &mut app.rootfs_composition {
        composition.system_inventory = RootfsAuthority::Available(Default::default());
    }
    update(
        &mut app,
        Action::SelectRootfsSystemdService { delta: isize::MAX },
    );
    assert_eq!(app.rootfs_systemd_selection, 0);
}

#[test]
fn rootfs_browser_reuses_lazy_navigation_and_preview_without_changing_layers_enter() {
    let mut app = App::new(20, 2000);
    app.screen = Screen::Images;
    let root = PathBuf::from("/build/rootfs");
    let directory = root.join("etc");
    update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "Rootfs: image".into(),
            root: root.clone(),
            directory: root.clone(),
            entries: vec![LayerBrowserEntry {
                path: directory.clone(),
                is_dir: true,
                ..Default::default()
            }],
        },
    );
    assert!(
        matches!(update(&mut app, Action::LayerBrowserExpand), Some(Effect::LoadLayerBrowserDirectory { directory: path, .. }) if path == directory)
    );
    let file = directory.join("config");
    update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "Rootfs: image".into(),
            root,
            directory: directory.clone(),
            entries: vec![LayerBrowserEntry {
                path: file.clone(),
                rootfs_metadata: Some(attributes(RootfsEntryKind::RegularFile, 0o644)),
                ..Default::default()
            }],
        },
    );
    assert!(
        matches!(update(&mut app, Action::SelectLayerBrowserEntry { delta: 1 }), Some(Effect::LoadLayerBrowserPreview(path)) if path == file)
    );
    assert!(update(&mut app, Action::LayerBrowserEnter).is_none());
    assert!(app.layer_browser.as_ref().unwrap().preview_focused);
    update(&mut app, Action::FocusLayerBrowserTree);
    assert!(matches!(
        update(&mut app, Action::EditSelectedLayerBrowserFile),
        Some(Effect::OpenLayerBrowserEditor { .. })
    ));
    update(&mut app, Action::LayerBrowserUp);
    assert_eq!(app.layer_browser.as_ref().unwrap().selection, 0);
    update(&mut app, Action::LayerBrowserEnter);
    assert_eq!(app.layer_browser.as_ref().unwrap().entries.len(), 1);
    let browser = app.layer_browser.as_mut().unwrap();
    browser.entries[0].is_dir = false;
    browser.entries[0].rootfs_metadata = Some(attributes(RootfsEntryKind::Symlink, 0o777));
    assert!(update(&mut app, Action::SelectLayerBrowserEntry { delta: 0 }).is_none());
    assert!(update(&mut app, Action::EditSelectedLayerBrowserFile).is_none());
    let browser = app.layer_browser.as_mut().unwrap();
    browser.layer = "meta-test".into();
    browser.entries[0].rootfs_metadata = None;
    assert!(matches!(
        update(&mut app, Action::LayerBrowserEnter),
        Some(Effect::OpenLayerBrowserEditor { .. })
    ));
    assert_eq!(app.screen, Screen::Images);
}
