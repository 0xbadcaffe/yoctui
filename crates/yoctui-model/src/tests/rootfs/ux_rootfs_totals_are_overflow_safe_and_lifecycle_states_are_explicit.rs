use super::*;

#[test]
fn ux_rootfs_totals_are_overflow_safe_and_lifecycle_states_are_explicit() {
    let composition = RootfsComposition {
        image: image(),
        installed_packages: RootfsAuthority::Available(RootfsPackageInventory {
            packages: vec![package("a", "base", u64::MAX), package("b", "base", 1)],
        }),
        filesystem_tree: RootfsAuthority::Unavailable {
            reason: "IMAGE_ROOTFS not reported".into(),
        },
        system_inventory: RootfsAuthority::Unavailable {
            reason: "IMAGE_ROOTFS not reported".into(),
        },
        root_directory: None,
    };
    let (totals, overflowed) = composition.totals();
    assert_eq!(totals.installed_package_bytes, u64::MAX);
    assert_eq!(totals.packages, 2);
    assert_eq!(totals.package_reported_files, 2);
    assert!(overflowed);
    assert!(composition.is_partial());
    assert!(!composition.is_unavailable());

    let request = RootfsCompositionRequest {
        generation: 1,
        image: image(),
    };
    for state in [
        RootfsCompositionState::Loading {
            request: request.clone(),
        },
        RootfsCompositionState::Unavailable {
            request: request.clone(),
            reason: "manifest unavailable".into(),
        },
        RootfsCompositionState::Failed {
            request: request.clone(),
            message: "adapter failed".into(),
        },
    ] {
        assert_eq!(state.request(), Some(&request));
        assert!(state.composition().is_none());
    }
}
