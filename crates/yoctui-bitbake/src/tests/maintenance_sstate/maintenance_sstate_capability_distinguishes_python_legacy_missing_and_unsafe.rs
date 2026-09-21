use super::*;

#[test]
fn maintenance_sstate_capability_distinguishes_python_legacy_missing_and_unsafe() {
    let (root, snapshot) = fixture(
        "python-capability",
        "sstate-cache-management.py",
        "#!/bin/sh\nexit 0\n",
    );
    assert!(matches!(
        snapshot.capability(MaintenanceTool::SstateCacheManagement),
        Some(MaintenanceToolCapability::Available {
            interface: MaintenanceToolInterface::SstatePython,
            ..
        })
    ));
    fs::remove_file(root.0.join("bin/sstate-cache-management.py")).unwrap();
    write_executable(
        &root.0.join("bin/sstate-cache-management.sh"),
        "#!/bin/sh\nexit 0\n",
    );
    let legacy = MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
        build_dir: root.0.join("build"),
        sstate_dir: Some(root.0.join("cache")),
        tmp_dir: Some(root.0.join("tmp")),
        stamps_dirs: vec![root.0.join("stamps")],
        executable_search_path: vec![root.0.join("bin")],
    })
    .unwrap();
    assert!(matches!(
        legacy.capability(MaintenanceTool::SstateCacheManagement),
        Some(MaintenanceToolCapability::Available {
            interface: MaintenanceToolInterface::SstateLegacyShell,
            ..
        })
    ));
    fs::remove_file(root.0.join("bin/sstate-cache-management.sh")).unwrap();
    let missing = MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
        build_dir: root.0.join("build"),
        sstate_dir: Some(root.0.join("cache")),
        tmp_dir: None,
        stamps_dirs: vec![],
        executable_search_path: vec![root.0.join("bin")],
    })
    .unwrap();
    assert!(matches!(
        missing.capability(MaintenanceTool::SstateCacheManagement),
        Some(MaintenanceToolCapability::Unavailable { .. })
    ));

    let linked = root.0.join("linked-bin");
    symlink(root.0.join("bin"), &linked).unwrap();
    let unsafe_snapshot =
        MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
            build_dir: root.0.join("build"),
            sstate_dir: Some(root.0.join("cache")),
            tmp_dir: None,
            stamps_dirs: vec![],
            executable_search_path: vec![linked],
        })
        .unwrap();
    assert!(!unsafe_snapshot.limitations.is_empty());
}
