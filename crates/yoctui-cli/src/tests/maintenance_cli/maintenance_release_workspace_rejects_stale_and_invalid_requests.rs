use super::*;

#[tokio::test]
async fn maintenance_release_workspace_rejects_stale_and_invalid_requests() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let (mut app, mut coordinator, capability_request, _) = refreshed_release_coordinator(
        &fixture,
        "#!/bin/sh\nexit 0\n",
        "#!/bin/sh\nexit 0\n",
        "#!/bin/sh\nexit 0\n",
    )
    .await;
    let locked = fixture.root.join("locked.inc");
    let input = fixture.root.join("input-cache");
    let output = fixture.root.join("output-cache");
    fs::write(&locked, b"SIGGEN_LOCKEDSIGS = \"\"\n").unwrap();
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let valid =
        LockedSignatureCacheRequest::new(locked, input, output, "ubuntu".into(), None).unwrap();
    coordinator
        .handle_effect(
            &mut app,
            Effect::Maintenance(MaintenanceEffect::PreviewLockedSignatureCache {
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
    coordinator
        .handle_effect(
            &mut app,
            Effect::Maintenance(MaintenanceEffect::PreviewLockedSignatureCache {
                capability_request,
                request: LockedSignatureCacheRequest {
                    locked_signatures: PathBuf::from("relative.inc"),
                    input_cache: fixture.root.clone(),
                    output_cache: fixture.build.clone(),
                    native_lsb: "ubuntu".into(),
                    filter: None,
                },
            }),
        )
        .await;
    poll_until(&mut coordinator, &mut app, |app, _| {
        app.notification.as_deref().is_some_and(|message| {
            message.contains("invalid")
                || message.contains("absolute")
                || message.contains("unsafe")
        })
    })
    .await;
    assert!(app.active_dialog().is_none());
    coordinator.shutdown().await;
}
