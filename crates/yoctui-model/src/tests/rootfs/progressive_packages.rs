use super::*;
use crate::{Action, App, Effect, ImagesView, LayerBrowser, Screen, update};

#[test]
fn rootfs_files_preview_opens_tree_and_completion_preserves_navigation() {
    let mut app = App::new(10, 4096);
    app.screen = Screen::Images;
    app.images_view = ImagesView::RootfsFilesystem;
    let request = RootfsCompositionRequest {
        generation: 1,
        image: image(),
    };
    let composition = RootfsComposition {
        image: image(),
        root_directory: Some("/build/rootfs".into()),
        installed_packages: RootfsAuthority::Available(Default::default()),
        filesystem_tree: RootfsAuthority::Unavailable {
            reason: "Still loading".into(),
        },
        system_inventory: RootfsAuthority::Unavailable {
            reason: "Still loading".into(),
        },
    };
    app.rootfs_composition = RootfsCompositionState::Loading {
        request: request.clone(),
    };
    let effect = update(
        &mut app,
        Action::RootfsCompositionPreview {
            request: request.clone(),
            composition: composition.clone(),
            limitations: vec!["Still loading".into()],
        },
    );
    assert!(matches!(
        effect,
        Some(Effect::LoadLayerBrowserDirectory { .. })
    ));
    let mut browser =
        LayerBrowser::new("Rootfs: core-image-minimal".into(), "/build/rootfs".into());
    browser.directory = "/build/rootfs/etc".into();
    app.layer_browser = Some(browser.clone());
    let effect = update(
        &mut app,
        Action::RootfsCompositionPartial {
            request: request.clone(),
            composition,
            limitations: vec!["Partial".into()],
        },
    );
    assert!(effect.is_none());
    assert_eq!(app.layer_browser, Some(browser));
    app.rootfs_composition = RootfsCompositionState::LoadingDetails {
        request: request.clone(),
        composition: app.rootfs_composition.composition().unwrap().clone(),
        limitations: Vec::new(),
    };
    update(
        &mut app,
        Action::RootfsCompositionFailed {
            request,
            message: "Authority changed".into(),
        },
    );
    assert!(app.layer_browser.is_none());
}

#[test]
fn rootfs_preview_preserves_packages_until_matching_details_finish() {
    let mut app = App::new(10, 4096);
    let request = RootfsCompositionRequest {
        generation: 1,
        image: image(),
    };
    let composition = RootfsComposition {
        image: image(),
        installed_packages: RootfsAuthority::Available(RootfsPackageInventory {
            packages: vec![package("busybox", "base", 1024)],
        }),
        filesystem_tree: RootfsAuthority::Unavailable {
            reason: "Details still loading".into(),
        },
        system_inventory: RootfsAuthority::Unavailable {
            reason: "Details still loading".into(),
        },
        root_directory: None,
    };
    app.rootfs_composition = RootfsCompositionState::Loading {
        request: request.clone(),
    };
    let preview = |request| Action::RootfsCompositionPreview {
        request,
        composition: composition.clone(),
        limitations: vec!["Details still loading".into()],
    };
    let mut stale = request.clone();
    stale.generation = 0;
    update(&mut app, preview(stale));
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Loading { .. }
    ));
    update(&mut app, preview(request.clone()));
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::LoadingDetails { .. }
    ));
    assert_eq!(
        app.rootfs_composition
            .composition()
            .unwrap()
            .totals()
            .0
            .installed_package_bytes,
        1024
    );
    assert!(app.rootfs_package_selection.is_some());
    update(
        &mut app,
        Action::RootfsCompositionPartial {
            request: request.clone(),
            composition: composition.clone(),
            limitations: vec!["Rootfs cleaned".into()],
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Partial { .. }
    ));
    update(&mut app, preview(request.clone()));
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Partial { .. }
    ));
    app.rootfs_composition = RootfsCompositionState::Loading {
        request: request.clone(),
    };
    update(&mut app, preview(request.clone()));
    update(
        &mut app,
        Action::RootfsCompositionFailed {
            request,
            message: "Authority changed".into(),
        },
    );
    assert!(matches!(
        app.rootfs_composition,
        RootfsCompositionState::Failed { .. }
    ));
    assert!(app.rootfs_composition.composition().is_none());
}
