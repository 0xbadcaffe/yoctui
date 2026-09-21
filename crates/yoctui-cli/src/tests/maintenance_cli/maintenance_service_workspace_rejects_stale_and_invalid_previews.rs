use super::*;

#[tokio::test]
async fn maintenance_service_workspace_rejects_stale_and_invalid_previews() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let (mut app, mut coordinator, capability_request) =
        refreshed_service_coordinator(&fixture, "#!/bin/sh\nexit 0\n").await;
    let valid = yoctui_model::PrServiceRequest::new(
        PrServiceOperation::Export,
        fixture.build.join("export.conf"),
        fixture.build.clone(),
        "localhost:8585".into(),
    )
    .unwrap();
    coordinator
        .handle_effect(
            &mut app,
            Effect::Maintenance(MaintenanceEffect::PreviewPrService {
                capability_request: capability_request + 1,
                request: valid,
            }),
        )
        .await;
    poll_until(&mut coordinator, &mut app, |app, _| {
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("capability changed"))
    })
    .await;
    assert!(app.active_dialog().is_none());

    app.notification = None;
    let invalid = yoctui_model::PrServiceRequest {
        operation: PrServiceOperation::Export,
        file: PathBuf::from("relative.conf"),
        build_dir: fixture.build.clone(),
        endpoint: "localhost:8585".into(),
    };
    coordinator
        .handle_effect(
            &mut app,
            Effect::Maintenance(MaintenanceEffect::PreviewPrService {
                capability_request,
                request: invalid,
            }),
        )
        .await;
    poll_until(&mut coordinator, &mut app, |app, _| {
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("absolute") || message.contains("unsafe"))
    })
    .await;
    assert!(app.active_dialog().is_none());
    coordinator.shutdown().await;
}
