use super::*;

#[test]
fn udev_unresolved_file_is_retained_and_limits_are_reported() {
    let root = TestRoot::new();
    let directory = root.path().join("etc/udev/rules.d");
    fs::create_dir_all(&directory).unwrap();
    std::os::unix::fs::symlink("/missing/image/file", directory.join("00-broken.rules")).unwrap();
    let mut limitations = Vec::new();
    let token = RootfsCompositionCancellation::default();
    let rules = scan(
        root.path(),
        &token,
        Instant::now() + ROOTFS_SCAN_TIMEOUT,
        &mut limitations,
    )
    .unwrap();
    assert_eq!(rules[0].status(), "Unresolved");
    assert!(!limitations.is_empty());
    for index in 0..MAX_ROOTFS_UDEV_RULES {
        fs::write(directory.join(format!("{index:05}.rules")), "").unwrap();
    }
    let rules = scan(
        root.path(),
        &token,
        Instant::now() + ROOTFS_SCAN_TIMEOUT,
        &mut limitations,
    )
    .unwrap();
    assert_eq!(rules.len(), MAX_ROOTFS_UDEV_RULES);
    assert!(
        limitations
            .iter()
            .any(|reason| reason.contains("safety bound"))
    );
}
