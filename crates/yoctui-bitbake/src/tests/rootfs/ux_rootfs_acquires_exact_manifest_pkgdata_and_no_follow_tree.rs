use super::*;

#[tokio::test]
async fn ux_rootfs_acquires_exact_manifest_pkgdata_and_no_follow_tree() {
    let (build, request, sources) = fixture();
    let expected_root = sources.image_rootfs.clone().unwrap();
    let response = RootfsCompositionAdapter::new(build.clone(), sources, 4)
        .scan(request.clone())
        .await
        .unwrap();
    assert_eq!(response.request, request);
    let packages = response.composition.package_inventory().unwrap();
    assert_eq!(packages.packages.len(), 2);
    assert_eq!(packages.packages[0].identity.name, "base-files");
    assert_eq!(packages.packages[0].file_count, 2);
    assert_eq!(packages.packages[1].installed_size_bytes, 12);
    let entries = &response.composition.filesystem_tree().unwrap().entries;
    assert!(entries.iter().any(|entry| {
        entry.identity.0 == Path::new("/usr/bin/busybox")
            && entry.kind == RootfsEntryKind::RegularFile
            && entry.size_bytes == 7
    }));
    #[cfg(unix)]
    assert!(entries.iter().any(|entry| {
        entry.identity.0 == Path::new("/usr/bin/sh") && entry.kind == RootfsEntryKind::Symlink
    }));
    assert!(
        response
            .limitations
            .iter()
            .any(|value| value.contains("ownership"))
    );
    let system = response.composition.system_inventory().unwrap();
    let unit = system
        .systemd_services
        .iter()
        .find(|service| service.name == "example.service")
        .unwrap();
    assert_eq!(unit.description.as_deref(), Some("Example daemon"));
    assert_eq!(unit.bus_name.as_deref(), Some("org.example.Daemon"));
    #[cfg(unix)]
    assert_eq!(unit.enabled_by, ["multi-user.target.wants"]);
    let activation = system
        .dbus_services
        .iter()
        .find(|service| service.name == "org.example.Helper")
        .unwrap();
    assert_eq!(
        activation.systemd_service.as_deref(),
        Some("example.service")
    );
    assert_eq!(activation.policy_files.len(), 1);
    assert!(
        system
            .dbus_services
            .iter()
            .any(|service| service.name == "org.example.Daemon")
    );
    assert_eq!(
        response.composition.root_directory.as_deref(),
        Some(expected_root.as_path())
    );
    fs::remove_dir_all(build).unwrap();
}
