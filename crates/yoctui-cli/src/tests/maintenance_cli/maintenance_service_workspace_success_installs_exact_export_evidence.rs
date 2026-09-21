use super::*;

#[tokio::test]
async fn maintenance_service_workspace_success_installs_exact_export_evidence() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let script = "#!/bin/sh\nprintf 'PRSERV_DUMP = \\\"1\\\"\\n' > \"$2\"\n";
    let (mut app, mut coordinator, capability_request) =
        refreshed_service_coordinator(&fixture, script).await;
    let destination = fixture.build.join("export.inc");
    let request = yoctui_model::PrServiceRequest::new(
        PrServiceOperation::Export,
        destination.clone(),
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
    assert_eq!(
        app.maintenance.sessions.back().unwrap().status,
        MaintenanceSessionStatus::Succeeded
    );
    assert_eq!(app.maintenance.evidence.len(), 1);
    assert_eq!(app.maintenance.evidence[0].identity.path, destination);
    coordinator.shutdown().await;
}
