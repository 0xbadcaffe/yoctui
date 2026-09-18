use super::*;

#[tokio::test]
async fn security_workflow_cli_cancels_only_the_exact_mapper_session() {
    let fixture = SecurityCliFixture::new("#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do :; done\n");
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
    let id = preview.id;
    let effect = update(
        &mut app,
        Action::Security(SecurityAction::ConfirmOperation(preview)),
    )
    .unwrap();
    assert!(coordinator.handle_effect(&mut app, effect).await);
    let _ = update(
        &mut app,
        Action::Security(SecurityAction::BeginCancellation),
    );
    let effect = update(
        &mut app,
        Action::Security(SecurityAction::ConfirmCancellation(id)),
    )
    .unwrap();
    assert!(coordinator.handle_effect(&mut app, effect).await);
    app.screen = Screen::Configuration;
    poll_security_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.mapper.is_none()
            && app
                .security
                .sessions
                .last()
                .is_some_and(|session| session.status.is_terminal())
    })
    .await;
    assert_eq!(
        app.security.sessions.last().unwrap().status,
        SecuritySessionStatus::Cancelled
    );
    assert_eq!(app.screen, Screen::Configuration);
}
