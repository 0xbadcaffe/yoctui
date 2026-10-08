use super::*;
use crate::{Action, App, update};

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
