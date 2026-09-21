use super::*;

#[test]
fn qa_layer_workflow_history_is_bounded_and_dialog_focus_is_trapped() {
    let mut state = QaState::default();
    load_layer(&mut state);
    for _ in 0..=MAX_QA_SESSIONS {
        let id = start_layer(&mut state);
        let _ = update_qa(
            &mut state,
            QaAction::CompleteLayerSession {
                session: id,
                exit_code: 0,
                result_paths: vec![],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
    }
    assert_eq!(state.layer_sessions.len(), MAX_QA_SESSIONS);
    assert_eq!(
        state.layer_sessions.front().unwrap().id,
        QaLayerSessionId(2)
    );

    let mut app = App::new(10, 1_000);
    app.screen = crate::Screen::Qa;
    app.focus = FocusTarget::Workspace;
    app.qa.view = QaView::LayerQa;
    let _ = update(
        &mut app,
        Action::Qa(QaAction::LayerCapabilityLoaded(layer_capability())),
    );
    let _ = update(&mut app, Action::Qa(QaAction::BeginSelectedLayerCheck));
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::Qa(QaDialog::LayerOperation(_)))
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = update(&mut app, Action::Qa(QaAction::CancelDialog));
    assert!(app.active_dialog().is_none());
    assert_eq!(app.focus, FocusTarget::Workspace);
}
