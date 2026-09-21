use super::*;

#[tokio::test]
async fn daemon_compatibility_runtime_refuses_host_path_without_initialized_build() {
    let fixture = RuntimeFixture::new();
    let environment = BTreeMap::from([("PATH".into(), fixture.bin.display().to_string())]);
    assert!(
        DaemonCompatibilityCoordinator::default()
            .startup_from_environment(&environment)
            .await
            .unwrap()
            .is_none()
    );
}
