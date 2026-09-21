use super::*;

#[tokio::test]
async fn pkgdata_adapter_rejects_symlinked_tool_and_pkgdata_paths() {
    let directory = TestDirectory::new("symlinks");
    let build_dir = directory.path().join("build");
    let real_pkgdata = directory.path().join("real-pkgdata");
    fs::create_dir_all(&build_dir).unwrap();
    fs::create_dir_all(&real_pkgdata).unwrap();
    let linked_pkgdata = build_dir.join("pkgdata");
    symlink(&real_pkgdata, &linked_pkgdata).unwrap();
    let tool = directory.path().join("tool");
    fs::write(&tool, "#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = fs::metadata(&tool).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&tool, permissions).unwrap();
    let error = PackageDataAdapter::with_paths(build_dir.clone(), tool.clone(), linked_pkgdata)
        .with_compatibility(compatibility(&build_dir, &tool), 1)
        .unwrap()
        .inventory(inventory_request())
        .await
        .unwrap_err();
    assert!(matches!(error, PackageDataAdapterError::MissingPkgdata(_)));

    let linked_tool = directory.path().join("linked-tool");
    symlink(&tool, &linked_tool).unwrap();
    let authority = compatibility(&build_dir, &linked_tool);
    let error = PackageDataAdapter::with_paths(build_dir, linked_tool, real_pkgdata)
        .with_compatibility(authority, 1)
        .unwrap()
        .inventory(inventory_request())
        .await
        .unwrap_err();
    assert!(matches!(error, PackageDataAdapterError::InvalidPath(_)));
}
