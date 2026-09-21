use super::*;

#[test]
fn pkgdata_model_normalizes_inventory_duplicates_bounds_and_unavailable_fields() {
    let mut preferred = summary("busybox");
    preferred.recipe = PackageField::Available("aaa".into());
    preferred.image_membership = PackageField::Available(vec![
        "image-z".into(),
        "image-a".into(),
        "image-a".into(),
        "bad image".into(),
    ]);
    let mut duplicate = preferred.clone();
    duplicate.recipe = PackageField::Available("zzz".into());
    let mut invalid_field = summary("libc");
    invalid_field.provider = PackageField::Available("relative.bb".into());
    let invalid = summary("bad package");
    let overflow = summary("zlib");

    let (packages, report) = normalize_package_summaries(
        vec![duplicate, invalid, overflow, invalid_field, preferred],
        2,
    );
    assert_eq!(packages.len(), 2);
    assert_eq!(packages[0].identity.name, "busybox");
    assert_eq!(packages[0].recipe, PackageField::Available("aaa".into()));
    assert_eq!(
        packages[0].image_membership,
        PackageField::Available(vec!["image-a".into(), "image-z".into()])
    );
    assert_eq!(packages[1].provider, PackageField::Unavailable);
    assert_eq!(report.duplicate_records, 1);
    assert_eq!(report.invalid_records, 1);
    assert_eq!(report.invalid_fields, 3);
    assert_eq!(report.truncated_records, 1);
    assert!(report.is_partial());
}
