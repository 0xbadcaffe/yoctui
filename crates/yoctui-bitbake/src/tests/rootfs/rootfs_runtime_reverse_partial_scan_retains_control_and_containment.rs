use super::*;

#[tokio::test]
async fn rootfs_runtime_reverse_partial_scan_retains_control_and_containment() {
    let (build, request, sources) = fixture();
    let pkgdata = sources.pkgdata_directory.as_ref().unwrap();
    fs::create_dir(pkgdata.join("runtime-reverse")).unwrap();
    std::os::unix::fs::symlink(
        "../runtime/missing",
        pkgdata.join("runtime-reverse/busybox"),
    )
    .unwrap();
    let response = RootfsCompositionAdapter::new(build.clone(), sources.clone(), 4)
        .scan(request.clone())
        .await
        .unwrap();
    assert!(matches!(
        response.composition.installed_packages,
        RootfsAuthority::Partial { .. }
    ));
    let packages = &response.composition.package_inventory().unwrap().packages;
    assert_eq!(packages.len(), 2);
    assert_eq!(
        packages
            .iter()
            .find(|p| p.identity.name == "base-files")
            .unwrap()
            .installed_size_bytes,
        5
    );
    assert_eq!(
        packages
            .iter()
            .find(|p| p.identity.name == "busybox")
            .unwrap()
            .installed_size_bytes,
        0
    );
    let cancellation = RootfsCompositionCancellation::default();
    cancellation.cancel();
    assert_eq!(
        RootfsCompositionAdapter::new(build.clone(), sources.clone(), 4)
            .scan_with_cancellation(request.clone(), cancellation)
            .await,
        Err(RootfsCompositionAdapterError::Cancelled)
    );
    assert!(matches!(
        scan_sources(
            request,
            build.clone(),
            sources.clone(),
            RootfsCompositionCancellation::default(),
            Instant::now()
        ),
        Err(RootfsCompositionAdapterError::Timeout(_))
    ));
    fs::rename(
        pkgdata.join("runtime-reverse"),
        pkgdata.join("saved-reverse"),
    )
    .unwrap();
    std::os::unix::fs::symlink("saved-reverse", pkgdata.join("runtime-reverse")).unwrap();
    assert!(read_installed_pkgdata(pkgdata, "busybox", &mut 0).is_err());
    fs::remove_dir_all(build).unwrap();
}
