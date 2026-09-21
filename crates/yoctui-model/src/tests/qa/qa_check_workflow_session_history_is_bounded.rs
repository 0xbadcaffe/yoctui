use super::*;

#[test]
fn qa_check_workflow_session_history_is_bounded() {
    let mut state = QaState::default();
    load(&mut state);
    for _ in 0..=MAX_QA_SESSIONS {
        let id = start(&mut state);
        let _ = update_qa(
            &mut state,
            QaAction::CompleteSession {
                session: id,
                result_paths: vec![],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
    }
    assert_eq!(state.sessions.len(), MAX_QA_SESSIONS);
    assert_eq!(state.sessions.front().unwrap().id, QaSessionId(2));
}
