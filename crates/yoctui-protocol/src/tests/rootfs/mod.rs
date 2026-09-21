use super::*;

fn data() -> RootfsCompositionData {
    RootfsCompositionData {
        schema_version: ROOTFS_COMPOSITION_SCHEMA_VERSION,
        request: RootfsCompositionRequestData {
            generation: 9,
            image: RootfsImageIdentityData {
                machine: "qemux86-64".into(),
                image: "core-image-minimal".into(),
                path: "/build/tmp/deploy/images/qemux86-64/image.ext4".into(),
            },
        },
        installed_packages: RootfsAuthorityData::Partial {
            records: vec![RootfsInstalledPackageData {
                name: "busybox".into(),
                recipe: Some("busybox".into()),
                category: "base".into(),
                installed_size_bytes: 1_024,
                file_count: 12,
            }],
            limitations: vec!["versions unavailable".into()],
        },
        filesystem_entries: RootfsAuthorityData::Available {
            records: vec![RootfsEntryData {
                path: "/usr/bin/busybox".into(),
                kind: RootfsEntryKindData::RegularFile,
                size_bytes: 1_024,
                package: Some("busybox".into()),
            }],
        },
        limitations: Vec::new(),
    }
}

mod ux_rootfs_protocol_round_trips_separate_authorities_and_exact_correlation;

mod rootfs_sources_roundtrip_preserves_missing_and_cleaned_paths;

mod rootfs_sources_rejects_invalid_identity_paths_and_bounds;

mod ux_rootfs_protocol_rejects_unknown_variants_paths_bounds_and_schema;
