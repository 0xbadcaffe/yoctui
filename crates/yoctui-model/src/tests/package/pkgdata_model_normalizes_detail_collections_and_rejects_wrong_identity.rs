use super::*;

#[test]
fn pkgdata_model_normalizes_detail_collections_and_rejects_wrong_identity() {
    let expected = PackageIdentity::new("busybox");
    let detail = PackageDetail {
        identity: expected.clone(),
        files: PackageField::Available(vec![
            "/usr/bin/busybox".into(),
            "relative".into(),
            "/usr/bin/busybox".into(),
        ]),
        runtime_dependencies: PackageField::Available(vec![
            PackageIdentity::new("libc"),
            PackageIdentity::new("bad dep"),
            PackageIdentity::new("libc"),
        ]),
        reverse_dependencies: PackageField::Available(Vec::new()),
    };
    let (detail, report) = normalize_package_detail(&expected, detail);
    let detail = detail.unwrap();
    assert_eq!(
        detail.files,
        PackageField::Available(vec![PathBuf::from("/usr/bin/busybox")])
    );
    assert_eq!(
        detail.runtime_dependencies,
        PackageField::Available(vec![PackageIdentity::new("libc")])
    );
    assert_eq!(report.invalid_fields, 2);

    let wrong = PackageDetail {
        identity: PackageIdentity::new("other"),
        files: PackageField::Unavailable,
        runtime_dependencies: PackageField::Unavailable,
        reverse_dependencies: PackageField::Unavailable,
    };
    let (detail, report) = normalize_package_detail(&expected, wrong);
    assert!(detail.is_none());
    assert_eq!(report.invalid_records, 1);
}
