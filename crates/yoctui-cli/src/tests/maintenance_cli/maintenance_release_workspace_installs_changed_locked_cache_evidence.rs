use super::*;

#[tokio::test]
async fn maintenance_release_workspace_installs_changed_locked_cache_evidence() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let locked_script = "#!/bin/sh\nmkdir -p \"$3/aa\"\nprintf 'sig\\n' > \"$3/aa/new.siginfo\"\nprintf 'locked cache ready\\n'\n";
    let (mut app, mut coordinator, capability_request, _) = refreshed_release_coordinator(
        &fixture,
        locked_script,
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
    coordinator
        .handle_effect(
            &mut app,
            Effect::Maintenance(MaintenanceEffect::PreviewLockedSignatureCache {
                capability_request,
                request: LockedSignatureCacheRequest::new(
                    locked,
                    input,
                    output.clone(),
                    "ubuntu".into(),
                    None,
                )
                .unwrap(),
            }),
        )
        .await;
    poll_until(&mut coordinator, &mut app, |app, _| {
        app.active_dialog().is_some()
    })
    .await;
    let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog().cloned() else {
        panic!("locked-cache confirmation is absent");
    };
    let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
        panic!("wrong locked-cache confirmation");
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
    assert_eq!(session.status, MaintenanceSessionStatus::Succeeded);
    assert!(
        session
            .output
            .iter()
            .any(|line| line.text == "locked cache ready")
    );
    assert!(app.maintenance.evidence.iter().any(|evidence| {
        evidence.identity.path == output.join("aa/new.siginfo")
            && evidence.label == "created locked-signature cache evidence"
    }));
    coordinator.shutdown().await;
}
