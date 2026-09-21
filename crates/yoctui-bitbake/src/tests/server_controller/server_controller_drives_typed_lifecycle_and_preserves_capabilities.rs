use super::*;

#[tokio::test]
async fn server_controller_drives_typed_lifecycle_and_preserves_capabilities() {
    let adapter = FakeAdapter::default();
    let mut controller =
        BitBakeServerController::new(adapter, context(), Duration::from_secs(1)).unwrap();
    assert_eq!(
        controller.detect().await.unwrap(),
        BitBakeDetection::Available
    );
    assert_eq!(
        controller
            .state()
            .observation
            .as_ref()
            .unwrap()
            .capabilities,
        vec![
            BitBakeServerCapability::Metadata,
            BitBakeServerCapability::BuildControl,
        ]
    );
    controller.connect().await.unwrap();
    assert_eq!(
        controller.state().lifecycle,
        BitBakeServerLifecycle::Connected
    );
    controller.reconnect().await.unwrap();
    assert_eq!(
        controller.state().connection_identity.as_deref(),
        Some("connection-2")
    );
    controller.restart().await.unwrap();
    assert_eq!(
        controller.state().lifecycle,
        BitBakeServerLifecycle::Connected
    );
    controller.stop().await.unwrap();
    assert_eq!(
        controller.state().lifecycle,
        BitBakeServerLifecycle::Unavailable
    );
    assert!(controller.state().generation > 0);
    assert_eq!(
        controller.into_adapter().calls,
        vec![
            "detect",
            "connect",
            "disconnect",
            "connect",
            "disconnect",
            "stop",
            "start",
            "connect",
            "disconnect",
            "stop",
        ]
    );
}
