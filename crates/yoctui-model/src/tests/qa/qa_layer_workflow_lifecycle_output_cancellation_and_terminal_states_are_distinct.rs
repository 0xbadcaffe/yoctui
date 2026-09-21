use super::*;

#[test]
fn qa_layer_workflow_lifecycle_output_cancellation_and_terminal_states_are_distinct() {
    let mut state = QaState::default();
    load_layer(&mut state);
    let id = start_layer(&mut state);
    let _ = update_qa(&mut state, QaAction::LayerSessionRunning(id));
    for index in 0..=MAX_QA_SESSION_OUTPUT {
        let _ = update_qa(
            &mut state,
            QaAction::LayerSessionOutput {
                session: id,
                stream: QaOutputStream::Stderr,
                line: format!("line {index}"),
                truncated: false,
            },
        );
    }
    assert_eq!(state.layer_sessions[0].output.len(), MAX_QA_SESSION_OUTPUT);
    assert_eq!(state.layer_sessions[0].dropped_output, 1);
    assert!(matches!(
        update_qa(&mut state, QaAction::BeginLayerCancellation).dialog,
        QaDialogUpdate::Open(dialog)
            if matches!(*dialog, QaDialog::LayerCancellation(session) if session == id)
    ));
    assert_eq!(
        update_qa(&mut state, QaAction::ConfirmLayerCancellation(id)).effect,
        Some(QaEffect::CancelLayerCheck(id))
    );
    let _ = update_qa(
        &mut state,
        QaAction::RejectLayerCancellation {
            session: id,
            message: "runner busy".into(),
        },
    );
    assert_eq!(state.layer_sessions[0].status, QaSessionStatus::Running);
    let _ = update_qa(
        &mut state,
        QaAction::CompleteLayerSession {
            session: id,
            exit_code: 2,
            result_paths: vec![],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(state.layer_sessions[0].status, QaSessionStatus::Failed);
    assert_eq!(state.layer_sessions[0].exit_code, Some(2));

    let mut cancelled = QaState::default();
    load_layer(&mut cancelled);
    let id = start_layer(&mut cancelled);
    let _ = update_qa(
        &mut cancelled,
        QaAction::CancelLayerSession {
            session: id,
            forced: true,
            exit_code: None,
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(
        cancelled.layer_sessions[0].status,
        QaSessionStatus::Cancelled
    );

    let mut timed_out = QaState::default();
    load_layer(&mut timed_out);
    let id = start_layer(&mut timed_out);
    let _ = update_qa(
        &mut timed_out,
        QaAction::TimeoutLayerSession {
            session: id,
            forced: false,
            exit_code: None,
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(
        timed_out.layer_sessions[0].status,
        QaSessionStatus::TimedOut
    );

    let mut lost = QaState::default();
    load_layer(&mut lost);
    let id = start_layer(&mut lost);
    let _ = update_qa(
        &mut lost,
        QaAction::LoseLayerSession {
            session: id,
            message: "runner channel closed".into(),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(lost.layer_sessions[0].status, QaSessionStatus::Lost);
}
