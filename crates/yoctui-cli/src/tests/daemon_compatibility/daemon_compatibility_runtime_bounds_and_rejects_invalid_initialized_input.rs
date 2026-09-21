use super::*;

#[tokio::test]
async fn daemon_compatibility_runtime_bounds_and_rejects_invalid_initialized_input() {
    let fixture = RuntimeFixture::new();
    fs::remove_file(fixture.build.join("conf/bblayers.conf")).unwrap();
    assert!(matches!(
        DaemonCompatibilityCoordinator::default()
            .startup_from_environment(&fixture.environment)
            .await,
        Err(DaemonCompatibilityError::InvalidStartupEnvironment(_))
    ));
}
