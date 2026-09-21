use super::*;

#[tokio::test]
async fn maintenance_service_workspace_reports_nonzero_without_export_evidence() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let (mut app, mut coordinator, capability_request) =
        refreshed_service_coordinator(&fixture, "#!/bin/sh\necho denied >&2\nexit 7\n").await;
    let destination = fixture.build.join("failed-export.conf");
    let request = yoctui_model::PrServiceRequest::new(
        PrServiceOperation::Export,
        destination,
        fixture.build.clone(),
        "localhost:8585".into(),
    )
    .unwrap();
    coordinator
        .handle_effect(
            &mut app,
            Effect::Maintenance(MaintenanceEffect::PreviewPrService {
                capability_request,
                request,
            }),
        )
        .await;
    poll_until(&mut coordinator, &mut app, |app, _| {
        app.active_dialog().is_some()
    })
    .await;
    let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog().cloned() else {
        panic!("PR confirmation is absent");
    };
    let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
        panic!("wrong PR confirmation");
    };
    let effect = update(
        &mut app,
        Action::Maintenance(MaintenanceAction::ConfirmOperation(preview.clone())),
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
    let session = app.maintenance.sessions.back().unwrap();
    assert_eq!(session.status, MaintenanceSessionStatus::Failed);
    assert!(session.output.iter().any(|line| line.text == "denied"));
    assert!(app.maintenance.evidence.is_empty());
    coordinator.shutdown().await;
}
