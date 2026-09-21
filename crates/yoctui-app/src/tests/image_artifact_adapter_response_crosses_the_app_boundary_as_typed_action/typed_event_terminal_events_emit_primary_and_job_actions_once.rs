use super::*;

#[test]
fn typed_event_terminal_events_emit_primary_and_job_actions_once() {
    let mut coordinator = BuildJobCoordinator::default();
    coordinator
        .queue_build(&request(), SystemTime::UNIX_EPOCH)
        .unwrap();
    let completed = coordinator.actions_for_backend_event(
        BackendEvent::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
        SystemTime::UNIX_EPOCH,
    );
    assert_eq!(
        completed
            .iter()
            .filter(|action| matches!(action, Action::BuildCompleted { .. }))
            .count(),
        1
    );
    assert_eq!(
        completed
            .iter()
            .filter(|action| matches!(action, Action::SucceedBackgroundJob { .. }))
            .count(),
        1
    );
    assert_eq!(completed.len(), 2);

    let mut coordinator = BuildJobCoordinator::default();
    coordinator
        .queue_build(&request(), SystemTime::UNIX_EPOCH)
        .unwrap();
    let failed = coordinator.actions_for_backend_event(
        BackendEvent::CommandFailed {
            code: "parse".into(),
            message: "bad metadata".into(),
        },
        SystemTime::UNIX_EPOCH,
    );
    assert_eq!(
        failed
            .iter()
            .filter(|action| matches!(action, Action::Failure(_)))
            .count(),
        1
    );
    assert_eq!(
        failed
            .iter()
            .filter(|action| matches!(action, Action::FailBackgroundJob { .. }))
            .count(),
        1
    );
    assert_eq!(failed.len(), 2);
}
