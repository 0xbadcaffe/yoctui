use super::*;

#[tokio::test]
async fn ux_rootfs_oversized_single_pkgdata_is_partial_not_a_screen_failure() {
    let (build, request, mut sources) = fixture();
    let manifest = sources.manifest.as_ref().unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(manifest)
        .unwrap()
        .write_all(b"oversized qemux86_64 1.0\n")
        .unwrap();
    let oversized = sources
        .pkgdata_directory
        .as_ref()
        .unwrap()
        .join("runtime/oversized");
    fs::File::create(&oversized)
        .unwrap()
        .set_len(MAX_PKGDATA_FILE_BYTES + 1)
        .unwrap();
    sources.image_rootfs = None;

    let response = RootfsCompositionAdapter::new(build.clone(), sources, 4)
        .scan(request)
        .await
        .unwrap();
    assert!(matches!(
        response.composition.installed_packages,
        RootfsAuthority::Partial { .. }
    ));
    assert!(
        response
            .limitations
            .iter()
            .any(|value| value.contains("oversized") && value.contains("limited"))
    );
    fs::remove_dir_all(build).unwrap();
}
