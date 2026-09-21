use super::*;

#[tokio::test]
async fn maintenance_workflow_cancels_only_the_exact_active_session() {
    let fixture = Fixture::new("#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n");
    let (mut app, mut coordinator, request) = refreshed_coordinator(&fixture).await;
    let id = MaintenanceSessionId(94);
    begin_readiness(&mut app, &mut coordinator, request, id, 30).await;
    poll_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.operation_active()
            && app
                .maintenance
                .sessions
                .back()
                .is_some_and(|session| session.status == MaintenanceSessionStatus::Running)
    })
    .await;

    coordinator
        .handle_effect(
            &mut app,
            Effect::Maintenance(MaintenanceEffect::CancelOperation(MaintenanceSessionId(
                999,
            ))),
        )
        .await;
    assert!(coordinator.operation_active());
    let _ = update(
        &mut app,
        Action::Maintenance(MaintenanceAction::BeginCancellation),
    );
    let effect = update(
        &mut app,
        Action::Maintenance(MaintenanceAction::ConfirmCancellation(id)),
    )
    .unwrap();
    coordinator.handle_effect(&mut app, effect).await;
    poll_until(&mut coordinator, &mut app, |app, coordinator| {
        !coordinator.operation_active()
            && app
                .maintenance
                .sessions
                .back()
                .is_some_and(|session| session.status.is_terminal())
    })
    .await;
    assert_eq!(
        app.maintenance.sessions.back().unwrap().status,
        MaintenanceSessionStatus::Cancelled
    );
    coordinator.shutdown().await;
}
