use super::*;

#[test]
fn rootfs_sources_rejects_invalid_identity_paths_and_bounds() {
    let sources = RootfsSourcesData {
        query: RootfsSourcesRequestData {
            request: data().request,
            daemon_instance_id: crate::daemon::DaemonInstanceId([7; 16]),
            compatibility_generation: 3,
        },
        image_manifest: None,
        pkgdata_dir: None,
        image_rootfs: None,
    };
    for path in [
        "relative".to_owned(),
        "/".to_owned(),
        "/build/../escape".to_owned(),
        "/build/bad\0path".to_owned(),
        format!("/{}", "x".repeat(MAX_ROOTFS_WIRE_PATH_BYTES)),
    ] {
        let mut invalid = sources.clone();
        invalid.image_rootfs = Some(path);
        assert!(invalid.validate().is_err());
    }
    let mut invalid = sources.clone();
    invalid.query.compatibility_generation = 0;
    assert!(invalid.validate().is_err());
    let mut invalid = sources.clone();
    invalid.query.daemon_instance_id.0 = [0; 16];
    assert!(invalid.validate().is_err());
    let mut invalid = sources;
    invalid.query.request.image.image = "image; command".into();
    assert!(invalid.validate().is_err());
}
