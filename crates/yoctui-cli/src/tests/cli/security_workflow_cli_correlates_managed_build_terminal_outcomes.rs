use super::*;

#[test]
fn security_workflow_cli_correlates_managed_build_terminal_outcomes() {
    let fixture = SecurityCliFixture::new("#!/bin/sh\nexit 0\n");
    let mut app = fixture.app();
    let input =
        security_capability_input(&app, fixture.build.clone(), vec![fixture.bin.clone()]).unwrap();
    let capability = SecurityCapabilityInspector::new(input).inspect().unwrap();
    let _ = update(
        &mut app,
        Action::Security(SecurityAction::CapabilityLoaded(capability)),
    );
    let _ = update(&mut app, Action::Security(SecurityAction::BeginCveCheck));
    let preview = match app.active_dialog().cloned() {
        Some(Dialog::Security(yoctui_model::SecurityDialog::Operation(preview))) => preview,
        other => panic!("unexpected build dialog: {other:?}"),
    };
    let _ = update(
        &mut app,
        Action::Security(SecurityAction::ConfirmOperation(preview)),
    );
    let id = app.security.sessions.last().unwrap().preview.id;

    let started = security_build_action_for_event(&app, id, &BackendEvent::BuildStarted).unwrap();
    let _ = update(&mut app, started);
    assert_eq!(
        app.security.sessions.last().unwrap().status,
        SecuritySessionStatus::Running
    );

    let mut succeeded = app.clone();
    succeeded.screen = Screen::Logs;
    let action = security_build_action_for_event(
        &succeeded,
        id,
        &BackendEvent::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    )
    .unwrap();
    assert!(matches!(
        update(&mut succeeded, action),
        Some(Effect::Security(SecurityEffect::ImportReports(_)))
    ));
    assert_eq!(
        succeeded.security.sessions.last().unwrap().status,
        SecuritySessionStatus::Succeeded
    );
    assert_eq!(succeeded.screen, Screen::Logs);

    let mut failed = app.clone();
    let action = security_build_action_for_event(
        &failed,
        id,
        &BackendEvent::BuildCompleted {
            success: false,
            exit_code: Some(1),
        },
    )
    .unwrap();
    let _ = update(&mut failed, action);
    assert_eq!(
        failed.security.sessions.last().unwrap().status,
        SecuritySessionStatus::Failed
    );

    let mut cancelled = app.clone();
    cancelled.security.sessions.last_mut().unwrap().status = SecuritySessionStatus::Cancelling;
    let action = security_build_action_for_event(
        &cancelled,
        id,
        &BackendEvent::BuildCompleted {
            success: false,
            exit_code: Some(130),
        },
    )
    .unwrap();
    let _ = update(&mut cancelled, action);
    assert_eq!(
        cancelled.security.sessions.last().unwrap().status,
        SecuritySessionStatus::Cancelled
    );

    let mut lost = app;
    let action = security_build_action_for_event(&lost, id, &BackendEvent::Disconnected).unwrap();
    let _ = update(&mut lost, action);
    assert_eq!(
        lost.security.sessions.last().unwrap().status,
        SecuritySessionStatus::Lost
    );
}
