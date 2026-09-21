use super::*;

#[tokio::test]
async fn compatibility_pkgdata_rejects_missing_command_stale_snapshot_and_zero_spawn() {
    let directory = TestDirectory::new("capability-reject");
    let build_dir = directory.path().join("build");
    let pkgdata_dir = build_dir.join("tmp/pkgdata");
    fs::create_dir_all(&pkgdata_dir).unwrap();
    let tool = directory.path().join("oe-pkgdata-util");
    let marker = directory.path().join("spawned");
    fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    #[cfg(unix)]
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    let mut authority = compatibility(&build_dir, &tool);
    let record = authority
        .snapshot
        .capabilities
        .iter_mut()
        .find(|record| record.id == CapabilityId::PkgDataListPackages)
        .unwrap();
    record.state = CapabilityState::Unavailable {
        reason: yoctui_model::CapabilityReason::new(
            "pkgdata.command_missing",
            "Current oe-pkgdata-util does not expose list-pkgs.",
            Some("Required command: list-pkgs".into()),
        )
        .unwrap(),
    };
    record.evidence[0].outcome = CapabilityEvidenceOutcome::Negative;
    authority
        .implementations
        .remove(&CapabilityId::PkgDataListPackages);
    let authority = authority.normalize().unwrap();
    assert!(matches!(
        PackageDataAdapter::with_paths(build_dir.clone(), tool.clone(), pkgdata_dir.clone())
            .with_compatibility(authority.clone(), 2),
        Err(PackageDataAdapterError::StaleCapability { .. })
    ));
    let error = PackageDataAdapter::with_paths(build_dir, tool, pkgdata_dir)
        .with_compatibility(authority, 1)
        .unwrap()
        .inventory(inventory_request())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        PackageDataAdapterError::CapabilityUnavailable {
            capability: CapabilityId::PkgDataListPackages,
            reason,
        } if reason.contains("does not expose list-pkgs")
    ));
    assert!(!marker.exists());
}
