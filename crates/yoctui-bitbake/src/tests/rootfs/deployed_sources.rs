use super::*;

fn sidecar(
    build: &Path,
    request: &RootfsCompositionRequest,
    sources: &RootfsCompositionSources,
) -> serde_json::Value {
    serde_json::json!({
        "PN": request.image.image, "MACHINE": request.image.machine,
        "IMAGE_NAME": "core-image-minimal",
        "IMAGE_MANIFEST": sources.manifest.as_ref().unwrap().strip_prefix(build).unwrap(),
        "IMAGE_ROOTFS": sources.image_rootfs.as_ref().unwrap().strip_prefix(build).unwrap(),
        "PKGDATA_DIR": sources.pkgdata_directory.as_ref().unwrap().strip_prefix(build).unwrap(),
    })
}

#[tokio::test]
async fn rootfs_deployed_metadata_opens_exact_root_before_full_scan() {
    let (build, request, sources) = fixture();
    let manifest = sources.manifest.as_ref().unwrap();
    let data = sidecar(&build, &request, &sources);
    fs::write(
        manifest.with_extension("testdata.json"),
        serde_json::to_vec(&data).unwrap(),
    )
    .unwrap();
    let resolved = rootfs_sources_from_deployed_metadata(&build, &request, manifest).unwrap();
    assert_eq!(resolved, sources);
    let preview = RootfsCompositionAdapter::new(build.clone(), resolved, request.generation)
        .scan_preview_with_cancellation(request.clone(), RootfsCompositionCancellation::default())
        .await
        .unwrap();
    assert_eq!(preview.composition.root_directory, sources.image_rootfs);
    assert!(preview.composition.filesystem_tree().is_none());
    assert!(preview.composition.system_inventory().is_none());
    assert!(preview.composition.package_inventory().is_some());
    let cancelled = RootfsCompositionCancellation::default();
    cancelled.cancel();
    let result = RootfsCompositionAdapter::new(build.clone(), sources, request.generation)
        .scan_preview_with_cancellation(request, cancelled)
        .await;
    assert_eq!(
        result.unwrap_err(),
        RootfsCompositionAdapterError::Cancelled
    );
    fs::remove_dir_all(build).unwrap();
}

#[test]
fn rootfs_deployed_metadata_rejects_wrong_image_machine_unsafe_and_cleaned_paths() {
    let (build, request, sources) = fixture();
    let manifest = sources.manifest.as_ref().unwrap();
    let path = manifest.with_extension("testdata.json");
    let original = sidecar(&build, &request, &sources);
    assert!(rootfs_sources_from_deployed_metadata(&build, &request, manifest).is_none());
    for (key, value) in [
        ("PN", "another-image"),
        ("MACHINE", "another-machine"),
        ("IMAGE_NAME", "another-build"),
        ("IMAGE_MANIFEST", "tmp/other.manifest"),
        ("IMAGE_ROOTFS", "../outside"),
        ("IMAGE_ROOTFS", "/etc"),
        ("IMAGE_ROOTFS", "tmp/cleaned/rootfs"),
        ("PKGDATA_DIR", "../pkgdata"),
    ] {
        let mut data = original.clone();
        data[key] = value.into();
        fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
        assert!(
            rootfs_sources_from_deployed_metadata(&build, &request, manifest).is_none(),
            "{key}={value}"
        );
    }
    fs::write(&path, b"not json").unwrap();
    assert!(rootfs_sources_from_deployed_metadata(&build, &request, manifest).is_none());
    #[cfg(unix)]
    {
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(manifest, &path).unwrap();
        assert!(rootfs_sources_from_deployed_metadata(&build, &request, manifest).is_none());
    }
    fs::remove_dir_all(build).unwrap();
}
