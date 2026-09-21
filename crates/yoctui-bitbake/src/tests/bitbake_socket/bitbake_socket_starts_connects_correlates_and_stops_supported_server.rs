use super::*;

#[tokio::test]
async fn bitbake_socket_starts_connects_correlates_and_stops_supported_server() {
    let (root, script, context) = fixture();
    let adapter = test_adapter(script, &context);
    let mut controller =
        BitBakeServerController::new(adapter, context.clone(), Duration::from_secs(2)).unwrap();
    assert_eq!(
        controller.detect().await.unwrap(),
        crate::BitBakeDetection::Unavailable
    );
    controller.start().await.unwrap();
    let observation = controller.state().observation.as_ref().unwrap();
    assert_eq!(observation.version.as_deref(), Some("2.8.1"));
    assert!(
        observation
            .capabilities
            .contains(&BitBakeServerCapability::EventStream)
    );
    controller.connect().await.unwrap();
    assert_eq!(
        controller.state().lifecycle,
        BitBakeServerLifecycle::Connected
    );
    assert!(
        controller
            .state()
            .connection_identity
            .as_deref()
            .unwrap()
            .starts_with("tinfoil-")
    );
    controller.stop().await.unwrap();
    assert_eq!(
        controller.state().lifecycle,
        BitBakeServerLifecycle::Unavailable
    );
    assert!(!context.build_dir.join("bitbake.sock").exists());
    fs::remove_dir_all(root).unwrap();
}
