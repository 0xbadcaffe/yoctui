use super::*;

#[tokio::test]
async fn maintenance_sstate_workspace_preview_failure_never_opens_destructive_dialog() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let (mut app, mut coordinator, capability_request, cache, _) =
        refreshed_cleanup_coordinator(&fixture, "#!/bin/sh\necho denied >&2\nexit 9\n").await;
    let request = SstateCleanupRequest::new(
        cache,
        Vec::new(),
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
            && app
                .notification
                .as_deref()
                .is_some_and(|message| message.contains("status Some(9)"))
    })
    .await;
    assert!(app.active_dialog().is_none());
    coordinator.shutdown().await;
}
