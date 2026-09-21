use super::*;

fn image() -> ImageArtifactIdentity {
    ImageArtifactIdentity {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        path: "/build/tmp/deploy/images/qemux86-64/core-image-minimal.ext4".into(),
    }
}

fn package(name: &str, category: &str, bytes: u64) -> RootfsInstalledPackage {
    RootfsInstalledPackage {
        identity: PackageIdentity::new(name),
        recipe: Some(name.into()),
        category: category.into(),
        installed_size_bytes: bytes,
        file_count: 1,
    }
}

mod ux_rootfs_normalizes_separate_authorities_bounds_and_correlates_image;

mod udev_normalization_preserves_distinct_paths_and_rejects_unsafe_records;

mod ux_rootfs_groups_other_with_exact_totals_percentages_and_members;

mod ux_rootfs_totals_are_overflow_safe_and_lifecycle_states_are_explicit;
