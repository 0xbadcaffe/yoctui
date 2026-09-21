use super::*;

#[tokio::test]
async fn maintenance_service_workspace_builds_distinct_exact_previews() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let (mut app, mut coordinator, capability_request) =
        refreshed_service_coordinator(&fixture, "#!/bin/sh\nexit 0\n").await;
    for operation in [PrServiceOperation::Export, PrServiceOperation::Import] {
        app.dialogs.clear();
        let file = fixture.build.join(match operation {
            PrServiceOperation::Export => "export.conf",
            PrServiceOperation::Import => "import.inc",
        });
        if operation == PrServiceOperation::Import {
            fs::write(&file, b"PRSERV_DUMP = \"1\"\n").unwrap();
        }
        let request = yoctui_model::PrServiceRequest::new(
            operation,
            file.clone(),
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
            matches!(
                app.active_dialog(),
                Some(yoctui_model::Dialog::Maintenance(dialog))
                    if matches!(dialog.as_ref(), yoctui_model::MaintenanceDialog::Confirm(_))
            )
        })
        .await;
        let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog() else {
            panic!("PR preview is absent");
        };
        let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
            panic!("wrong PR preview dialog");
        };
        let expected = match operation {
            PrServiceOperation::Export => "export",
            PrServiceOperation::Import => "import",
        };
        assert!(
            preview
                .arguments
                .iter()
                .any(|argument| argument.ends_with(&format!(": {expected}")))
        );
        assert!(
            preview
                .arguments
                .iter()
                .any(|argument| argument.ends_with(&format!(": {}", file.display())))
        );
    }
    coordinator.shutdown().await;
}
