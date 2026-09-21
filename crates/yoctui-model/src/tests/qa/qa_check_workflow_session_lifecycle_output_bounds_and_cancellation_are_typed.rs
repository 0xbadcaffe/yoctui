use super::*;

#[test]
fn qa_check_workflow_session_lifecycle_output_bounds_and_cancellation_are_typed() {
    let mut state = QaState::default();
    load(&mut state);
    let id = start(&mut state);
    let job = BackgroundJobId(41);
    let _ = update_qa(
        &mut state,
        QaAction::AttachBackgroundJob {
            session: id,
            background_job: job,
        },
    );
    let _ = update_qa(&mut state, QaAction::SessionRunning(id));
    for index in 0..=MAX_QA_SESSION_OUTPUT {
        let _ = update_qa(
            &mut state,
            QaAction::SessionOutput {
                session: id,
                stream: QaOutputStream::Stdout,
                line: format!("line {index}"),
                truncated: false,
            },
        );
    }
    assert_eq!(state.sessions[0].output.len(), MAX_QA_SESSION_OUTPUT);
    assert_eq!(state.sessions[0].dropped_output, 1);

    let transition = update_qa(&mut state, QaAction::BeginCancellation);
    assert!(matches!(
        transition.dialog,
        QaDialogUpdate::Open(dialog)
            if matches!(
                *dialog,
                QaDialog::Cancellation {
                    session,
                    background_job
                } if session == id && background_job == job
            )
    ));
    let transition = update_qa(&mut state, QaAction::ConfirmCancellation(id));
    assert_eq!(
        transition.effect,
        Some(QaEffect::CancelBuild {
            session: id,
            background_job: job
        })
    );
    let _ = update_qa(
        &mut state,
        QaAction::RejectCancellation {
            session: id,
            message: "coordinator busy".into(),
        },
    );
    assert_eq!(state.sessions[0].status, QaSessionStatus::Running);
    let _ = update_qa(
        &mut state,
        QaAction::TimeoutSession {
            session: id,
            forced: true,
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(state.sessions[0].status, QaSessionStatus::TimedOut);
}
