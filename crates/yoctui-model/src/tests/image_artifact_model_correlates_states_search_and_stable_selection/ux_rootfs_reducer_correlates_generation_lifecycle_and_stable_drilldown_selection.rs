use super::*;

#[test]
fn ux_rootfs_reducer_correlates_generation_lifecycle_and_stable_drilldown_selection() {
    let mut app = App::new(20, 20_000);
    let artifact = ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: "/build/tmp/deploy/images/qemux86-64/core-image-minimal.ext4".into(),
        },
        kind: ImageArtifactKind::RootFilesystem,
        size_bytes: ImageArtifactField::Available(42),
        modified_unix_seconds: ImageArtifactField::Available(10),
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Unavailable,
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Unavailable,
    };
    app.image_artifacts = ImageArtifactInventoryState::Available {
        request: ImageArtifactRequest {
            generation: 1,
            machine: artifact.identity.machine.clone(),
        },
        inventory: ImageArtifactInventory {
            machine: artifact.identity.machine.clone(),
            deploy_directory: ImageArtifactField::Available(
                "/build/tmp/deploy/images/qemux86-64".into(),
            ),
            artifacts: vec![artifact.clone()],
        },
    };
    app.image_artifact_selection = Some(artifact.identity.clone());
    let request = RootfsCompositionRequest {
        generation: 1,
        image: artifact.identity.clone(),
    };
    assert_eq!(
        update(&mut app, Action::BeginSelectedRootfsComposition),
        Some(Effect::GetRootfsComposition(request.clone()))
    );
    assert_eq!(app.images_view, ImagesView::RootfsPackages);
    let package = |name: &str, category: &str| RootfsInstalledPackage {
        identity: PackageIdentity::new(name),
        recipe: Some(name.into()),
        category: category.into(),
        installed_size_bytes: 100,
        file_count: 2,
    };
    let composition = RootfsComposition {
        image: artifact.identity.clone(),
        installed_packages: RootfsAuthority::Available(RootfsPackageInventory {
            packages: vec![package("busybox", "base"), package("glibc", "runtime")],
        }),
        filesystem_tree: RootfsAuthority::Available(RootfsFilesystemTree {
            entries: vec![
                RootfsEntry {
                    identity: RootfsPathIdentity("/".into()),
                    kind: RootfsEntryKind::Directory,
                    size_bytes: 0,
                    package: None,
                },
                RootfsEntry {
                    identity: RootfsPathIdentity("/usr".into()),
                    kind: RootfsEntryKind::Directory,
                    size_bytes: 0,
                    package: None,
                },
            ],
        }),
        system_inventory: RootfsAuthority::Available(RootfsSystemInventory::default()),
        root_directory: Some("/build/tmp/rootfs".into()),
    };
    let stale = RootfsCompositionRequest {
        generation: 99,
        image: artifact.identity.clone(),
    };
    let _ = update(
        &mut app,
        Action::RootfsCompositionLoaded {
            request: stale,
            composition: composition.clone(),
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Loading { .. }
    ));
    let _ = update(
        &mut app,
        Action::RootfsCompositionLoaded {
            request: request.clone(),
            composition: composition.clone(),
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Available { .. }
    ));
    assert_eq!(
        app.rootfs_group_selection,
        Some(RootfsGroupIdentity::Category("base".into()))
    );
    assert_eq!(
        app.rootfs_package_selection,
        Some(PackageIdentity::new("busybox"))
    );
    let _ = update(&mut app, Action::SelectRootfsEntry { delta: 1 });
    assert_eq!(
        app.rootfs_entry_selection,
        Some(RootfsPathIdentity("/usr".into()))
    );
    let _ = update(&mut app, Action::SelectRootfsGroup { delta: 1 });
    assert_eq!(
        app.rootfs_group_selection,
        Some(RootfsGroupIdentity::Category("runtime".into()))
    );
    assert_eq!(
        app.rootfs_package_selection,
        Some(PackageIdentity::new("glibc"))
    );

    let Effect::GetRootfsComposition(refresh) =
        update(&mut app, Action::RefreshRootfsComposition).unwrap()
    else {
        panic!("expected rootfs refresh effect")
    };
    assert_eq!(refresh.generation, 2);
    assert_eq!(refresh.image, artifact.identity);
    let _ = update(
        &mut app,
        Action::RootfsCompositionPartial {
            request: refresh,
            composition,
            limitations: vec!["pkgdata sizes partial".into()],
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Partial { .. }
    ));
    assert_eq!(
        app.rootfs_group_selection,
        Some(RootfsGroupIdentity::Category("runtime".into()))
    );
    assert_eq!(
        app.rootfs_package_selection,
        Some(PackageIdentity::new("glibc"))
    );
    assert_eq!(
        app.rootfs_entry_selection,
        Some(RootfsPathIdentity("/usr".into()))
    );

    let Effect::GetRootfsComposition(empty_request) =
        update(&mut app, Action::RefreshRootfsComposition).unwrap()
    else {
        panic!("expected rootfs refresh effect")
    };
    let _ = update(
        &mut app,
        Action::RootfsCompositionLoaded {
            request: empty_request,
            composition: RootfsComposition {
                image: artifact.identity.clone(),
                installed_packages: RootfsAuthority::Available(RootfsPackageInventory::default()),
                filesystem_tree: RootfsAuthority::Available(RootfsFilesystemTree::default()),
                system_inventory: RootfsAuthority::Available(RootfsSystemInventory::default()),
                root_directory: None,
            },
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::AvailableEmpty { .. }
    ));
    assert_eq!(app.rootfs_group_selection, None);
    assert_eq!(app.rootfs_entry_selection, None);

    let Effect::GetRootfsComposition(unavailable) =
        update(&mut app, Action::RefreshRootfsComposition).unwrap()
    else {
        panic!("expected rootfs refresh effect")
    };
    let _ = update(
        &mut app,
        Action::RootfsCompositionUnavailable {
            request: unavailable,
            reason: "manifest absent".into(),
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Unavailable { .. }
    ));

    let Effect::GetRootfsComposition(failed) =
        update(&mut app, Action::RefreshRootfsComposition).unwrap()
    else {
        panic!("expected rootfs refresh effect")
    };
    let _ = update(
        &mut app,
        Action::RootfsCompositionFailed {
            request: failed,
            message: "adapter failed".into(),
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Failed { .. }
    ));
}
