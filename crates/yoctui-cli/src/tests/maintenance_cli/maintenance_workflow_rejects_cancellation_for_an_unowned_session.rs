use super::*;

#[tokio::test]
async fn maintenance_workflow_rejects_cancellation_for_an_unowned_session() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let (mut app, mut coordinator, _) = refreshed_coordinator(&fixture).await;
    let handled = coordinator
        .handle_effect(
            &mut app,
            Effect::Maintenance(MaintenanceEffect::CancelOperation(MaintenanceSessionId(7))),
        )
        .await;
    assert!(handled);
    assert!(!coordinator.operation_active());
    coordinator.shutdown().await;
}
