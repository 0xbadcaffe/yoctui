use super::*;

fn summary(name: &str) -> PackageSummary {
    PackageSummary {
        identity: PackageIdentity::new(name),
        recipe: PackageField::Available("busybox".into()),
        provider: PackageField::Available("/layers/meta/busybox.bb".into()),
        version: PackageField::Available("1.0".into()),
        installed_size_bytes: PackageField::Available(1024),
        license: PackageField::Available("GPL-2.0-only".into()),
        image_membership: PackageField::Available(vec!["core-image-minimal".into()]),
    }
}

mod pkgdata_model_normalizes_inventory_duplicates_bounds_and_unavailable_fields;

mod pkgdata_model_normalizes_detail_collections_and_rejects_wrong_identity;
