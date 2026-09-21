use super::*;

#[tokio::test]
async fn maintenance_release_workspace_defers_push_until_local_head_success() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let script = "#!/bin/sh\nmkdir -p \"$2\"\nprintf 'ref: refs/heads/main\\n' > \"$2/HEAD\"\n";
    let (mut app, mut coordinator, capability_request, _) = refreshed_release_coordinator(
        &fixture,
        "#!/bin/sh\nexit 0\n",
        "#!/bin/sh\nprintf 'comparison\\n'\n",
        script,
    )
    .await;
    let request = GitArchiveRequest::new(GitArchiveRequest {
        data_dir: fixture.root.clone(),
        git_dir: fixture.root.join("push-release.git"),
        create: true,
        bare: true,
        create_tag: false,
        branch_name: "release".into(),
        tag_name: None,
        commit_subject: "Release".into(),
        commit_body: String::new(),
        tag_subject: "Tag".into(),
        tag_body: String::new(),
        exclusions: Vec::new(),
        notes: Vec::new(),
        push_remote: Some("origin".into()),
    })
    .unwrap();
    coordinator
        .handle_effect(
            &mut app,
            Effect::Maintenance(MaintenanceEffect::PreviewGitArchive {
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
        panic!("local archive confirmation is absent");
    };
    let yoctui_model::MaintenanceDialog::Confirm(local) = dialog.as_ref() else {
        panic!("wrong local archive confirmation");
    };
    assert!(!local.operation.network_side_effect());
    let effect = update(
        &mut app,
        Action::Maintenance(MaintenanceAction::ConfirmOperation(local.clone())),
    )
    .unwrap();
    coordinator.handle_effect(&mut app, effect).await;
    poll_until(&mut coordinator, &mut app, |app, coordinator| {
        !coordinator.operation_active()
            && matches!(
                app.active_dialog(),
                Some(yoctui_model::Dialog::Maintenance(dialog))
                    if matches!(dialog.as_ref(), yoctui_model::MaintenanceDialog::Confirm(preview) if preview.operation.network_side_effect())
            )
    })
    .await;
    assert!(
        app.maintenance
            .evidence
            .iter()
            .any(|evidence| evidence.label == "Git archive HEAD")
    );

    let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog().cloned() else {
        panic!("push confirmation is absent");
    };
    let yoctui_model::MaintenanceDialog::Confirm(push) = dialog.as_ref() else {
        panic!("wrong push confirmation");
    };
    let _ = update(
        &mut app,
        Action::Maintenance(MaintenanceAction::ConfirmOperation(push.clone())),
    );
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::Maintenance(dialog))
            if matches!(dialog.as_ref(), yoctui_model::MaintenanceDialog::ConfirmNetworkPush(_))
    ));
    fs::write(
        fixture.root.join("push-release.git/HEAD"),
        b"changed local head\n",
    )
    .unwrap();
    let effect = update(
        &mut app,
        Action::Maintenance(MaintenanceAction::ConfirmNetworkPush(push.clone())),
    )
    .unwrap();
    coordinator.handle_effect(&mut app, effect).await;
    poll_until(&mut coordinator, &mut app, |app, coordinator| {
        !coordinator.operation_active()
            && app.maintenance.sessions.len() == 2
            && app
                .maintenance
                .sessions
                .back()
                .is_some_and(|session| session.status.is_terminal())
    })
    .await;
    let push_session = app.maintenance.sessions.back().unwrap();
    assert_eq!(push_session.status, MaintenanceSessionStatus::Failed);
    assert!(
        push_session
            .message
            .as_deref()
            .is_some_and(|message| message.contains("changed"))
    );
    coordinator.shutdown().await;
}
