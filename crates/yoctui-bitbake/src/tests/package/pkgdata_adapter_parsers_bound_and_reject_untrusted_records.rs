use super::*;

#[test]
fn pkgdata_adapter_parsers_bound_and_reject_untrusted_records() {
    let mut limitations = Vec::new();
    let mut input = String::new();
    for index in 0..=MAX_PACKAGE_RECORDS {
        input.push_str(&format!("package-{index}\n"));
    }
    input.push_str("bad package\n");
    let identities = parse_package_list(input.as_bytes(), &mut limitations).unwrap();
    assert_eq!(identities.len(), MAX_PACKAGE_RECORDS);
    assert!(
        limitations
            .iter()
            .any(|limitation| limitation.contains("limited"))
    );

    let mut summaries = BTreeMap::from([unavailable_summary(PackageIdentity::new("busybox"))]);
    parse_package_info(
        b"other 1.0 other 1.0 5 \"MIT\"\nmalformed\nbusybox 1.0 busybox 1.0 nope \"MIT\"\n",
        &mut summaries,
        &mut limitations,
    )
    .unwrap();
    assert_eq!(
        summaries[&PackageIdentity::new("busybox")].installed_size_bytes,
        PackageField::Unavailable
    );
    assert!(
        limitations
            .iter()
            .any(|limitation| limitation.contains("unexpected package-info identity"))
    );

    assert!(matches!(
        parse_package_files(
            &PackageIdentity::new("busybox"),
            b"wrong:\n\t/bin/value\n",
            &mut limitations
        ),
        Err(PackageDataAdapterError::Malformed(_))
    ));
}
