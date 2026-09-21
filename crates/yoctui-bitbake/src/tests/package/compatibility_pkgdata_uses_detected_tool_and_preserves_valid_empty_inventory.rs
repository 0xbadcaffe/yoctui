use super::*;

#[tokio::test]
async fn compatibility_pkgdata_uses_detected_tool_and_preserves_valid_empty_inventory() {
    let directory = TestDirectory::new("discover");
    let build_dir = directory.path().join("build");
    fs::create_dir_all(build_dir.join("tmp/pkgdata")).unwrap();
    let scripts = directory.path().join("layers/openembedded-core/scripts");
    fs::create_dir_all(&scripts).unwrap();
    let tool = scripts.join("oe-pkgdata-util");
    fs::write(
        &tool,
        "#!/bin/sh\nprintf 'No packages found\\n' >&2\nexit 1\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&tool).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&tool, permissions).unwrap();
    }
    let pkgdata_dir = build_dir.join("tmp/pkgdata");
    let authority = compatibility(&build_dir, &tool);
    let response = PackageDataAdapter::with_paths(build_dir, tool, pkgdata_dir)
        .with_compatibility(authority, 1)
        .unwrap()
        .inventory(inventory_request())
        .await
        .unwrap();
    assert!(response.packages.is_empty());
    assert!(response.limitations.is_empty());
}
