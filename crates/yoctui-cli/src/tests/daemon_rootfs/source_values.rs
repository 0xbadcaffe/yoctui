use super::environment_value;

#[test]
fn rootfs_environment_values_use_exact_last_assignment_and_exported_names() {
    let output = "# IMAGE_ROOTFS=ignored\nIMAGE_ROOTFS_SUFFIX=wrong\nIMAGE_ROOTFS=\"/old\"\nexport IMAGE_ROOTFS=\"/build/root fs\"\nPKGDATA_DIR=\"\"\nOTHER=wrong\n";
    assert_eq!(
        environment_value(output, "IMAGE_ROOTFS"),
        Some("/build/root fs".into())
    );
    assert_eq!(environment_value(output, "PKGDATA_DIR"), None);
    assert_eq!(environment_value(output, "IMAGE_MANIFEST"), None);
    assert_eq!(
        environment_value("IMAGE_ROOTFS=/plain\n", "IMAGE_ROOTFS"),
        Some("/plain".into())
    );
    assert_eq!(
        environment_value("IMAGE_ROOTFS=\"/old\"\nIMAGE_ROOTFS=\"\"\n", "IMAGE_ROOTFS"),
        None
    );
}
