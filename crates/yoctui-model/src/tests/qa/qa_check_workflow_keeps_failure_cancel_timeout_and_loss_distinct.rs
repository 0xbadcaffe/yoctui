use super::*;

#[test]
fn qa_check_workflow_keeps_failure_cancel_timeout_and_loss_distinct() {
    let mut failed = QaState::default();
    load(&mut failed);
    let id = start(&mut failed);
    let _ = update_qa(
        &mut failed,
        QaAction::FailSession {
            session: id,
            message: "task failed".into(),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(failed.sessions[0].status, QaSessionStatus::Failed);

    let mut cancelled = QaState::default();
    load(&mut cancelled);
    let id = start(&mut cancelled);
    let _ = update_qa(
        &mut cancelled,
        QaAction::CancelSession {
            session: id,
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(cancelled.sessions[0].status, QaSessionStatus::Cancelled);

    let mut timed_out = QaState::default();
    load(&mut timed_out);
    let id = start(&mut timed_out);
    let _ = update_qa(
        &mut timed_out,
        QaAction::TimeoutSession {
            session: id,
            forced: false,
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(timed_out.sessions[0].status, QaSessionStatus::TimedOut);

    let mut lost = QaState::default();
    load(&mut lost);
    let id = start(&mut lost);
    let _ = update_qa(
        &mut lost,
        QaAction::LoseSession {
            session: id,
            message: "coordinator channel closed".into(),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(lost.sessions[0].status, QaSessionStatus::Lost);
}
