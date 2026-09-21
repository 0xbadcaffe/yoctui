use super::*;

#[tokio::test]
async fn maintenance_sstate_workspace_discovers_exact_cleanup_candidates_before_phrase() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let candidate = fixture.root.join("sstate-cache/candidate.tgz");
    let script = format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", candidate.display());
    let (mut app, mut coordinator, capability_request, cache, stamps) =
        refreshed_cleanup_coordinator(&fixture, &script).await;
    fs::write(&candidate, b"candidate").unwrap();
    let request = SstateCleanupRequest::new(
        cache.clone(),
        vec![stamps],
        vec![yoctui_model::SstateCleanupMode::Duplicates],
        1,
    )
    .unwrap();
    coordinator
        .handle_effect(
            &mut app,
            Effect::Maintenance(MaintenanceEffect::PreviewCleanup {
                capability_request,
                request,
            }),
        )
        .await;
    poll_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.cleanup_preview.is_none()
            && matches!(
                app.active_dialog(),
                Some(yoctui_model::Dialog::Maintenance(dialog))
                    if matches!(dialog.as_ref(), yoctui_model::MaintenanceDialog::CleanupPhrase { .. })
            )
    })
    .await;
    let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog() else {
        panic!("cleanup phrase dialog is absent");
    };
    let yoctui_model::MaintenanceDialog::CleanupPhrase { preview, .. } = dialog.as_ref() else {
        panic!("wrong cleanup dialog");
    };
    let MaintenanceOperation::SstateCleanup(preview) = &preview.operation else {
        panic!("wrong cleanup operation");
    };
    assert_eq!(preview.candidates.len(), 1);
    assert_eq!(preview.candidates[0].path, candidate);
    assert_eq!(
        preview.required_phrase(),
        format!("DELETE 1 FROM {}", cache.display())
    );
    coordinator.shutdown().await;
}
