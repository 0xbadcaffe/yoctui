use super::*;

#[tokio::test]
async fn raw_capability_probe_daemon_publishes_and_reuses_option_authority() {
    let fixture = RuntimeFixture::new();
    let mut coordinator = DaemonCompatibilityCoordinator::default();
    let first = coordinator
        .startup_from_environment(&fixture.environment)
        .await
        .unwrap()
        .unwrap();
    assert!(first.snapshot.allows(CapabilityId::BitBakeRawCli));
    assert!(first.snapshot.allows(CapabilityId::BitBakeRawDryRun));
    assert!(!first.snapshot.allows(CapabilityId::BitBakeRawRunAll));
    assert_eq!(
        first
            .implementations
            .get(&CapabilityId::BitBakeRawDryRun)
            .unwrap()
            .id,
        "bitbake.raw.dry_run.argv"
    );
    assert!(matches!(
        first
            .snapshot
            .capability(CapabilityId::BitBakeRawRunAll)
            .unwrap()
            .state,
        yoctui_model::CapabilityState::Unavailable { .. }
    ));

    let second = coordinator
        .startup_from_environment(&fixture.environment)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first, second);
}
