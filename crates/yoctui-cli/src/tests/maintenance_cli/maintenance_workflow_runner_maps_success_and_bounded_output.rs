use super::*;

#[tokio::test]
async fn maintenance_workflow_runner_maps_success_and_bounded_output() {
    let fixture = Fixture::new("#!/bin/sh\nprintf 'cache ready\\n'\nexit 0\n");
    let (mut app, mut coordinator, request) = refreshed_coordinator(&fixture).await;
    let snapshot = app.maintenance.capability.snapshot().unwrap().clone();
    let id = MaintenanceSessionId(91);
    let readiness = SstateReadinessRequest::new(
        vec!["core-image-minimal".into()],
        SstateReadinessMode::IsolatedTmpdir,
        None,
        None,
        30,
    )
    .unwrap();
    let (preview, _) =
        MaintenanceSstateCommandSpec::readiness(id, request, &snapshot, id.0, readiness).unwrap();
    let _ = update(
        &mut app,
        Action::Maintenance(MaintenanceAction::BeginOperation(preview.clone())),
    );
    let effect = update(
        &mut app,
        Action::Maintenance(MaintenanceAction::ConfirmOperation(preview)),
    )
    .unwrap();
    assert!(coordinator.handle_effect(&mut app, effect).await);
    poll_until(&mut coordinator, &mut app, |app, coordinator| {
        !coordinator.operation_active()
            && app
                .maintenance
                .sessions
                .back()
                .is_some_and(|session| session.status.is_terminal())
    })
    .await;
    let session = app.maintenance.sessions.back().unwrap();
    assert_eq!(session.status, MaintenanceSessionStatus::Succeeded);
    assert!(session.output.iter().any(|line| line.text == "cache ready"));
    coordinator.shutdown().await;
}
