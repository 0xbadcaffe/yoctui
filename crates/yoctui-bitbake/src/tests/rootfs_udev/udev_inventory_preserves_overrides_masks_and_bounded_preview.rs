use super::*;

#[test]
fn udev_inventory_preserves_overrides_masks_and_bounded_preview() {
    let temp = TestRoot::new();
    let root = temp.path();
    for dir in DIRECTORIES {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    fs::write(
        root.join("usr/lib/udev/rules.d/10-device.rules"),
        "SUBSYSTEM==\"tty\"\n",
    )
    .unwrap();
    fs::write(
        root.join("etc/udev/rules.d/10-device.rules"),
        "# override\n",
    )
    .unwrap();
    fs::write(
        root.join("usr/lib/udev/rules.d/20-large.rules"),
        "x".repeat(9000),
    )
    .unwrap();
    fs::write(root.join("etc/udev/rules.d/ignored.txt"), "ignored").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("/dev/null", root.join("etc/udev/rules.d/30-mask.rules")).unwrap();
    let mut limitations = Vec::new();
    let rules = scan(
        root,
        &RootfsCompositionCancellation::default(),
        Instant::now() + ROOTFS_SCAN_TIMEOUT,
        &mut limitations,
    )
    .unwrap();
    assert!(limitations.is_empty(), "{limitations:?}");
    assert_eq!(
        rules
            .iter()
            .filter(|rule| rule.name == "10-device.rules")
            .count(),
        2
    );
    assert!(rules.iter().any(|rule| rule.overridden_by.is_some()));
    assert!(
        rules
            .iter()
            .any(|rule| rule.preview_truncated && rule.preview.len() == 8192)
    );
    #[cfg(unix)]
    assert!(rules.iter().any(|rule| rule.status() == "Masked"));
}
