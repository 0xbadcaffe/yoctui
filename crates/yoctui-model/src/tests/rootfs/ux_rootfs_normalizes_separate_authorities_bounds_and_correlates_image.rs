use super::*;

#[test]
fn ux_rootfs_normalizes_separate_authorities_bounds_and_correlates_image() {
    let request = RootfsCompositionRequest {
        generation: 7,
        image: image(),
    };
    let mut deep = PathBuf::from("/");
    for _ in 0..=MAX_ROOTFS_DEPTH {
        deep.push("d");
    }
    let composition = RootfsComposition {
        image: image(),
        installed_packages: RootfsAuthority::Partial {
            value: RootfsPackageInventory {
                packages: vec![
                    package("busybox", "base", 10),
                    package("busybox", "base", 20),
                    package("bad name", "base", 1),
                ],
            },
            limitations: vec!["manifest omitted versions".into()],
        },
        filesystem_tree: RootfsAuthority::Available(RootfsFilesystemTree {
            entries: vec![
                RootfsEntry {
                    identity: RootfsPathIdentity("/".into()),
                    kind: RootfsEntryKind::Directory,
                    size_bytes: 0,
                    package: None,
                },
                RootfsEntry {
                    identity: RootfsPathIdentity("/usr/bin/busybox".into()),
                    kind: RootfsEntryKind::RegularFile,
                    size_bytes: 10,
                    package: Some(PackageIdentity::new("busybox")),
                },
                RootfsEntry {
                    identity: RootfsPathIdentity(deep),
                    kind: RootfsEntryKind::Directory,
                    size_bytes: 0,
                    package: None,
                },
            ],
        }),
        system_inventory: RootfsAuthority::Available(RootfsSystemInventory::default()),
        root_directory: None,
    };
    let (normalized, report) = normalize_rootfs_composition(&request, composition);
    let normalized = normalized.unwrap();
    assert_eq!(normalized.package_inventory().unwrap().packages.len(), 1);
    assert_eq!(normalized.filesystem_tree().unwrap().entries.len(), 2);
    assert_eq!(report.duplicate_packages, 1);
    assert_eq!(report.invalid_packages, 1);
    assert_eq!(report.truncated_depth, 1);
    assert_eq!(report.orphan_entries, 1);
    assert!(report.is_partial());
    let tree = normalized.filesystem_tree().unwrap();
    assert!(tree.children(&RootfsPathIdentity("/".into())).is_empty());
    let (descendants, truncated) = tree.descendants(&RootfsPathIdentity("/".into()), 1);
    assert_eq!(descendants.len(), 1);
    assert_eq!(truncated, 0);

    let wrong_request = RootfsCompositionRequest {
        generation: 8,
        image: ImageArtifactIdentity {
            image: "other".into(),
            ..image()
        },
    };
    assert!(
        normalize_rootfs_composition(&wrong_request, normalized)
            .0
            .is_none()
    );
}
