use super::*;

#[tokio::test]
async fn maintenance_workflow_refresh_is_correlated_and_preserves_unavailable_tools() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let (app, mut coordinator, request) = refreshed_coordinator(&fixture).await;
    let snapshot = app.maintenance.capability.snapshot().unwrap();
    assert_eq!(request, 1);
    assert!(snapshot.supports(MaintenanceTool::OeCheckSstate));
    assert!(matches!(
        snapshot.capability(MaintenanceTool::BuildCompare),
        Some(MaintenanceToolCapability::Unavailable { .. })
    ));
    assert_eq!(app.maintenance.services.request(), Some(request));
    assert_eq!(app.maintenance.integrations.request(), Some(request));
    coordinator.shutdown().await;
}
