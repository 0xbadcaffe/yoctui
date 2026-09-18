use super::*;

#[tokio::test]
async fn qa_workflow_cli_reuses_managed_build_and_correlates_terminal_outcomes() {
    let fixture = QaCliFixture::new("#!/bin/sh\nexit 0\n");
    fixture.write_report();
    let mut app = fixture.app();
    let mut coordinator = QaCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
    inspect_qa_capability(&fixture, &mut app, &mut coordinator).await;
    app.qa.check_selection = Some(QaCheckId::new("recipe-package".into()).unwrap());
    let _ = update(&mut app, Action::Qa(QaAction::BeginSelectedCheck));
    let preview = match app.active_dialog().cloned() {
        Some(Dialog::Qa(yoctui_model::QaDialog::Operation(preview))) => preview,
        other => panic!("unexpected QA build dialog: {other:?}"),
    };
    let effect = update(&mut app, Action::Qa(QaAction::ConfirmOperation(preview))).unwrap();
    let Effect::Qa(QaEffect::StartBuild { session, request }) = effect else {
        panic!("QA build did not produce its typed effect");
    };
    let started = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut backend: Box<dyn BitBakeBackend> = Box::new(SecurityBuildBackend {
        started: started.clone(),
        fail_start: false,
    });
    let mut jobs = BuildJobCoordinator::default();
    assert!(begin_qa_build(&mut backend, &mut app, &mut jobs, session, request.clone()).await);
    assert_eq!(*started.lock().unwrap(), [request]);
    assert_eq!(
        app.qa.sessions.back().unwrap().background_job_id,
        jobs.active_job_id()
    );
    let started_action =
        qa_build_action_for_event(&app, session, &BackendEvent::BuildStarted).unwrap();
    let _ = update(&mut app, started_action);
    assert_eq!(
        app.qa.sessions.back().unwrap().status,
        QaSessionStatus::Running
    );
    app.screen = Screen::Logs;
    let completed_action = qa_build_action_for_event(
        &app,
        session,
        &BackendEvent::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    )
    .unwrap();
    let followup = update(&mut app, completed_action);
    assert!(matches!(
        followup,
        Some(Effect::Qa(QaEffect::ImportReports(_)))
    ));
    assert_eq!(app.screen, Screen::Logs);

    let mut failed = fixture.app();
    inspect_qa_capability(&fixture, &mut failed, &mut coordinator).await;
    failed.qa.check_selection = Some(QaCheckId::new("recipe-package".into()).unwrap());
    let _ = update(&mut failed, Action::Qa(QaAction::BeginSelectedCheck));
    let preview = match failed.active_dialog().cloned() {
        Some(Dialog::Qa(yoctui_model::QaDialog::Operation(preview))) => preview,
        other => panic!("unexpected QA build dialog: {other:?}"),
    };
    let _ = update(&mut failed, Action::Qa(QaAction::ConfirmOperation(preview)));
    let failed_id = failed.qa.sessions.back().unwrap().id;
    let action = qa_build_action_for_event(
        &failed,
        failed_id,
        &BackendEvent::BuildCompleted {
            success: false,
            exit_code: Some(1),
        },
    )
    .unwrap();
    let _ = update(&mut failed, action);
    assert_eq!(
        failed.qa.sessions.back().unwrap().status,
        QaSessionStatus::Failed
    );
}
