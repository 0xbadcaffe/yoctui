use super::*;

#[tokio::test]
async fn daemon_compatibility_backend_probe_enables_future_api_only_with_current_evidence() {
    let mut fixture = RuntimeFixture::new();
    write_tool(
        &fixture.bin.join("bitbake"),
        "echo 'BitBake Build Tool version 2.19.0'",
    );
    let python = fixture.bin.join("probe-python");
    fixture
        .environment
        .insert("PYTHON".into(), python.display().to_string());
    let report = serde_json::json!({
        "schema": "yoctui.bridge-capability-probe.v1",
        "build_directory": fixture.build.canonicalize().unwrap(),
        "bitbake_version": "2.19.0",
        "capabilities": ["workspace", "layers", "layer_relationships", "recipes",
            "recipe_dependencies", "recipe_sources", "recipe_metadata", "tasks",
            "build", "cancel", "native_events", "variable_history", "server_socket"],
    });
    write_tool(&python, &format!("printf '%s\\n' '{report}'"));
    let authority = DaemonCompatibilityCoordinator::default()
        .startup_from_environment(&fixture.environment)
        .await
        .unwrap()
        .unwrap();
    assert!(authority.snapshot.allows(CapabilityId::BitBakeBuild));
    assert_eq!(
        authority.implementations[&CapabilityId::BitBakeBuild].id,
        "tinfoil.build"
    );
    assert!(authority.snapshot.allows(CapabilityId::BitBakeCancellation));

    write_tool(&python, "echo malformed");
    let incomplete = DaemonCompatibilityCoordinator::default()
        .startup_from_environment(&fixture.environment)
        .await
        .unwrap()
        .unwrap();
    assert!(!incomplete.snapshot.allows(CapabilityId::BitBakeBuild));
    assert!(matches!(
        incomplete
            .snapshot
            .capability(CapabilityId::BitBakeBuild)
            .unwrap()
            .state,
        yoctui_model::CapabilityState::Unknown { .. }
    ));
}
