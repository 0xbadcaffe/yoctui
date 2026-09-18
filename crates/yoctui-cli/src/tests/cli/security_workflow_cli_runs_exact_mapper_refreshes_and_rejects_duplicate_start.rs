use super::*;

#[tokio::test]
async fn security_workflow_cli_runs_exact_mapper_refreshes_and_rejects_duplicate_start() {
    let fixture =
        SecurityCliFixture::new("#!/bin/sh\nprintf 'mapped busybox -> busybox\\n'\nexit 0\n");
    fixture.write_cve();
    let mut app = fixture.app();
    let mut coordinator =
        SecurityCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
    let effect = update(
        &mut app,
        Action::Security(SecurityAction::InspectCapability),
    )
    .unwrap();
    assert!(coordinator.handle_effect(&mut app, effect).await);
    poll_security_until(&mut coordinator, &mut app, |app, _| {
        matches!(
            app.security.capability,
            yoctui_model::SecurityCapability::Available(_)
        )
    })
    .await;

    let _ = update(&mut app, Action::Security(SecurityAction::BeginPackageMap));
    let preview = match app.active_dialog().cloned() {
        Some(Dialog::Security(yoctui_model::SecurityDialog::Operation(preview))) => preview,
        other => panic!("unexpected mapper dialog: {other:?}"),
    };
    let effect = update(
        &mut app,
        Action::Security(SecurityAction::ConfirmOperation(preview)),
    )
    .unwrap();
    assert!(coordinator.handle_effect(&mut app, effect.clone()).await);
    assert!(coordinator.handle_effect(&mut app, effect).await);
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("already owned"))
    );
    app.screen = Screen::Layers;
    poll_security_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.mapper.is_none()
            && coordinator.report.is_none()
            && app
                .security
                .sessions
                .last()
                .is_some_and(|session| session.status.is_terminal())
    })
    .await;
    let session = app.security.sessions.last().unwrap();
    assert_eq!(session.status, SecuritySessionStatus::Succeeded);
    assert!(
        session
            .output
            .iter()
            .any(|line| line.line.contains("mapped busybox"))
    );
    assert!(app.security.inventory.reports().is_some());
    assert_eq!(app.screen, Screen::Layers);
}
