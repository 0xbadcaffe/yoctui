use super::*;

#[test]
fn image_size_delta_compares_consecutive_snapshots_for_the_same_target() {
    let mut app = App::new(16, 4096);
    let composition = |path: &str, installed_size_bytes| RootfsComposition {
        root_directory: None,
        system_inventory: RootfsAuthority::Unavailable {
            reason: "not loaded".into(),
        },
        image: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: PathBuf::from(path),
        },
        installed_packages: RootfsAuthority::Available(RootfsPackageInventory {
            packages: vec![RootfsInstalledPackage {
                identity: PackageIdentity::new("busybox"),
                recipe: Some("busybox".into()),
                category: "base".into(),
                installed_size_bytes,
                file_count: 10,
            }],
        }),
        filesystem_tree: RootfsAuthority::Unavailable {
            reason: "not loaded".into(),
        },
    };
    let previous = composition("/tmp/core-image-minimal-1.rootfs.tar", 1_000);
    let current = composition("/tmp/core-image-minimal-2.rootfs.tar", 1_250);
    app.record_overview_image_size(&previous);
    app.record_overview_image_size(&current);
    app.rootfs_composition = RootfsCompositionState::Available {
        request: RootfsCompositionRequest {
            generation: 2,
            image: current.image.clone(),
        },
        composition: current,
    };
    assert_eq!(
        app.overview_image_size_delta(),
        Some(OverviewImageSizeDelta {
            current_bytes: 1_250,
            previous_bytes: 1_000,
            delta_bytes: 250,
        })
    );
}
