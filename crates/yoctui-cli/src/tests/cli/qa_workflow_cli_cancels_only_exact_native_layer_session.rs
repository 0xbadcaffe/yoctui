use super::*;

#[tokio::test]
async fn qa_workflow_cli_cancels_only_exact_native_layer_session() {
    let fixture = QaCliFixture::new("#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n");
    let mut app = fixture.app();
    let mut coordinator = QaCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
    inspect_qa_capability(&fixture, &mut app, &mut coordinator).await;
    let effect = update(&mut app, Action::Qa(QaAction::CycleView)).unwrap();
    assert!(coordinator.handle_effect(&mut app, effect).await);
    poll_qa_until(&mut coordinator, &mut app, |app, _| {
        app.qa.layer_capability.snapshot().is_some()
    })
    .await;
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
    assert!(coordinator.handle_effect(&mut app, effect).await);
    let actual_id = app.qa.layer_sessions.back().unwrap().id;
    let _ = update(&mut app, Action::Qa(QaAction::BeginLayerCancellation));
    let effect = update(
        &mut app,
        Action::Qa(QaAction::ConfirmLayerCancellation(actual_id)),
    )
    .unwrap();
    assert!(coordinator.handle_effect(&mut app, effect).await);
    assert!(!coordinator.owns_layer(QaLayerSessionId(actual_id.0 + 1)));
    poll_qa_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.layer.is_none()
            && app
                .qa
                .layer_sessions
                .back()
                .is_some_and(|session| session.status.is_terminal())
    })
    .await;
    assert_eq!(
        app.qa.layer_sessions.back().unwrap().status,
        QaSessionStatus::Cancelled
    );
}
