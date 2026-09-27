use super::*;
use crate::daemon_compatibility::backend_recovery::{
    BackendRecovery, is_current_recovery, needed, recover,
};
use yoctui_model::{CapabilityId, CapabilityState, DaemonCompatibilitySnapshot};

fn unknown(fixture: &RuntimeFixture) -> DaemonCompatibilitySnapshot {
    let resolved = yoctui_bitbake::CapabilityResolver::default()
        .resolve_snapshot(
            1,
            environment(fixture.build.canonicalize().unwrap(), "2.19.0"),
            &yoctui_model::CapabilityCatalog::builtin(),
            &BTreeMap::new(),
        )
        .unwrap();
    DaemonCompatibilitySnapshot {
        snapshot: resolved.snapshot,
        implementations: resolved.implementations,
    }
    .normalize()
    .unwrap()
}

fn probe(fixture: &mut RuntimeFixture, version: &str, capabilities: &[&str]) {
    let python = fixture.bin.join("probe-python");
    fixture
        .environment
        .insert("PYTHON".into(), python.display().to_string());
    let report = serde_json::json!({
        "schema": "yoctui.bridge-capability-probe.v1",
        "build_directory": fixture.build.canonicalize().unwrap(),
        "bitbake_version": version,
        "capabilities": capabilities,
    });
    write_tool(&python, &format!("printf '%s\\n' '{report}'"));
}

#[tokio::test]
async fn backend_recovery_replaces_only_backend_records_after_positive_probe() {
    let mut fixture = RuntimeFixture::new();
    let initial = unknown(&fixture);
    assert!(needed(&initial));
    probe(
        &mut fixture,
        "2.19.0",
        &["recipe_metadata", "workspace", "recipes"],
    );
    let recovered = recover(initial.clone(), &fixture.environment)
        .await
        .unwrap();
    assert!(is_current_recovery(&initial, &recovered));
    assert!(
        recovered
            .snapshot
            .allows(CapabilityId::BitBakeRecipeMetadata)
    );
    assert!(
        recovered
            .snapshot
            .allows(CapabilityId::BitBakeWorkspaceInspection)
    );
    assert_eq!(
        recovered.implementations[&CapabilityId::BitBakeRecipeMetadata].id,
        "tinfoil.recipe_metadata"
    );
    assert_eq!(
        initial.snapshot.capability(CapabilityId::BitBakeGetVar),
        recovered.snapshot.capability(CapabilityId::BitBakeGetVar)
    );
    assert!(!needed(&recovered));
    assert!(!is_current_recovery(&recovered, &recovered));
    let mut other = initial.clone();
    other.snapshot.environment.bitbake_version =
        AuthoritativeValue::detected("2.20.0".into(), IdentityAuthority::BitBakeVersionProbe);
    assert!(!is_current_recovery(&other, &recovered));
}

#[tokio::test]
async fn backend_recovery_rejects_wrong_identity_and_keeps_negative_apis_disabled() {
    let mut fixture = RuntimeFixture::new();
    let initial = unknown(&fixture);
    probe(&mut fixture, "2.20.0", &["recipe_metadata"]);
    assert!(
        recover(initial.clone(), &fixture.environment)
            .await
            .is_err()
    );
    probe(&mut fixture, "2.19.0", &[]);
    let recovered = recover(initial.clone(), &fixture.environment)
        .await
        .unwrap();
    assert!(matches!(
        recovered
            .snapshot
            .capability(CapabilityId::BitBakeRecipeMetadata)
            .unwrap()
            .state,
        CapabilityState::Unavailable { .. }
    ));
    assert!(!needed(&recovered));
    fixture
        .environment
        .insert("BUILDDIR".into(), fixture.root.display().to_string());
    assert!(recover(initial, &fixture.environment).await.is_err());
}

#[tokio::test]
async fn backend_recovery_failure_is_bounded_and_retries_are_throttled() {
    let mut fixture = RuntimeFixture::new();
    probe(&mut fixture, "2.19.0", &[]);
    write_tool(
        &fixture.bin.join("probe-python"),
        "echo 'BitBake server is busy' >&2; exit 1",
    );
    let initial = unknown(&fixture);
    let mut recovery = BackendRecovery::default();
    assert!(
        recovery
            .poll(Some(&initial), &fixture.environment)
            .is_none()
    );
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = recovery.poll(Some(&initial), &fixture.environment) {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("BitBake server is busy")
            );
            break;
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    // A successful next report must not bypass the delay after a failed attempt.
    probe(&mut fixture, "2.19.0", &["workspace"]);
    for _ in 0..10 {
        assert!(
            recovery
                .poll(Some(&initial), &fixture.environment)
                .is_none()
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(needed(&initial));
    recovery.shutdown().await;
    let recovered = recover(initial, &fixture.environment).await.unwrap();
    assert!(
        recovered
            .snapshot
            .allows(CapabilityId::BitBakeWorkspaceInspection)
    );
}

#[tokio::test]
async fn backend_recovery_can_cancel_an_inflight_probe() {
    let mut fixture = RuntimeFixture::new();
    probe(&mut fixture, "2.19.0", &[]);
    write_tool(&fixture.bin.join("probe-python"), "sleep 30");
    let initial = unknown(&fixture);
    let mut recovery = BackendRecovery::default();
    recovery.poll(Some(&initial), &fixture.environment);
    tokio::time::sleep(Duration::from_millis(50)).await;
    tokio::time::timeout(Duration::from_secs(2), recovery.shutdown())
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires an initialized Yocto environment and YOCTUI_LIVE_COMPATIBILITY snapshot"]
async fn backend_recovery_live_recovers_unknown_api_authority() {
    let path = std::env::var("YOCTUI_LIVE_COMPATIBILITY").unwrap();
    let wire: yoctui_protocol::daemon::CompatibilitySnapshotData =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let initial = yoctui_app::compatibility_model_snapshot(&wire).unwrap();
    assert!(
        needed(&initial),
        "capture must contain genuinely unknown API authority"
    );
    let recovered = recover(initial.clone(), &std::env::vars().collect())
        .await
        .unwrap();
    assert!(is_current_recovery(&initial, &recovered));
    for id in [
        CapabilityId::BitBakeWorkspaceInspection,
        CapabilityId::BitBakeRecipeMetadata,
        CapabilityId::BitBakeRecipeInventory,
    ] {
        assert!(
            recovered.snapshot.allows(id),
            "live probe did not verify {id:?}"
        );
    }
}
