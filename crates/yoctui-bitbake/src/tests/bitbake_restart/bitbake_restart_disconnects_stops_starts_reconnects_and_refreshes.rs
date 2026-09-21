use super::*;

#[tokio::test]
async fn bitbake_restart_disconnects_stops_starts_reconnects_and_refreshes() {
    let mut coordinator = coordinator().await;
    let preview = coordinator.preview(affected()).unwrap();
    let metadata = coordinator
        .restart(&preview, affected(), Some(&preview.confirmation()))
        .await
        .unwrap();
    assert_eq!(metadata.machine.as_deref(), Some("qemux86-64"));
    assert_eq!(
        coordinator.controller().state().lifecycle,
        BitBakeServerLifecycle::Connected
    );
    let (controller, refresher) = coordinator.into_parts();
    assert_eq!(refresher.calls, 1);
    assert_eq!(
        controller.into_adapter().calls,
        vec!["start", "connect", "disconnect", "stop", "start", "connect"]
    );
}
