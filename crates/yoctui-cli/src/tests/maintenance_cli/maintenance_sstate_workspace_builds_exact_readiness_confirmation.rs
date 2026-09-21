use super::*;

#[tokio::test]
async fn maintenance_sstate_workspace_builds_exact_readiness_confirmation() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let (mut app, mut coordinator, capability_request) = refreshed_coordinator(&fixture).await;
    let request = SstateReadinessRequest::new(
        vec!["core-image-minimal".into()],
        SstateReadinessMode::SameTmpdir,
        Some(fixture.build.join("readiness.txt")),
        None,
        30,
    )
    .unwrap();
    assert!(
        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::PreviewReadiness {
                    capability_request,
                    request,
                }),
            )
            .await
    );
    poll_until(&mut coordinator, &mut app, |app, _| {
        matches!(
            app.active_dialog(),
            Some(yoctui_model::Dialog::Maintenance(dialog))
                if matches!(dialog.as_ref(), yoctui_model::MaintenanceDialog::Confirm(_))
        )
    })
    .await;
    let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog() else {
        panic!("readiness confirmation is absent");
    };
    let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
        panic!("wrong readiness confirmation");
    };
    assert!(
        preview
            .arguments
            .iter()
            .any(|argument| argument.ends_with(": --same-tmpdir"))
    );
    assert!(
        preview
            .arguments
            .iter()
            .any(|argument| argument.ends_with(": core-image-minimal"))
    );
    coordinator.shutdown().await;
}
