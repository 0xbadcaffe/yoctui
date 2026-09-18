use super::*;

#[tokio::test]
async fn qa_workflow_cli_runs_exact_layer_check_refreshes_and_rejects_duplicate() {
    let fixture = QaCliFixture::new("#!/bin/sh\nprintf 'checking configured layer\\n'\nexit 0\n");
    fixture.write_report();
    let mut app = fixture.app();
    let mut coordinator = QaCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
    inspect_qa_capability(&fixture, &mut app, &mut coordinator).await;
    let effect = update(&mut app, Action::Qa(QaAction::CycleView)).unwrap();
    assert!(coordinator.handle_effect(&mut app, effect).await);
    poll_qa_until(&mut coordinator, &mut app, |app, _| {
        app.qa.layer_capability.snapshot().is_some()
    })
    .await;
    let selected = app.qa.selected_layer().unwrap();
    assert_eq!(selected.identity.root, fixture.layer);
    assert!(matches!(
        selected.run,
        yoctui_model::QaLayerRunCapability::Available { .. }
    ));

    let _ = update(&mut app, Action::Qa(QaAction::BeginSelectedLayerCheck));
    let preview = match app.active_dialog().cloned() {
        Some(Dialog::Qa(yoctui_model::QaDialog::LayerOperation(preview))) => preview,
        other => panic!("unexpected layer-QA dialog: {other:?}"),
    };
    let effect = update(
        &mut app,
        Action::Qa(QaAction::ConfirmLayerOperation(preview)),
    )
    .unwrap();
    assert!(coordinator.handle_effect(&mut app, effect.clone()).await);
    assert!(coordinator.handle_effect(&mut app, effect).await);
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("already owned"))
    );
    app.screen = Screen::Dashboard;
    poll_qa_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.layer.is_none()
            && coordinator.report.is_none()
            && app
                .qa
                .layer_sessions
                .back()
                .is_some_and(|session| session.status.is_terminal())
    })
    .await;
    let session = app.qa.layer_sessions.back().unwrap();
    assert_eq!(session.status, QaSessionStatus::Succeeded);
    assert!(
        session
            .output
            .iter()
            .any(|line| line.line.contains("checking configured layer"))
    );
    assert!(app.qa.inventory.reports().is_some());
    assert_eq!(app.screen, Screen::Dashboard);
}
