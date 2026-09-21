use super::*;

#[test]
fn pty_multi_requires_runner_termination_before_close_and_bounds_history() {
    let mut registry = PtySessionRegistry::new(2, 1, 2).unwrap();
    let first = registry.reserve_id().unwrap();
    registry.insert(session(first, "one", 10)).unwrap();
    assert_eq!(
        registry.request_close(first).unwrap(),
        PtyCloseDisposition::TerminationRequired
    );
    registry
        .get_mut(first)
        .unwrap()
        .apply(PtySessionAction::Exit(PtyExitStatus::Code(0)))
        .unwrap();
    registry.complete_close(first).unwrap();
    assert_eq!(registry.history()[0].id, first);

    let second = registry.reserve_id().unwrap();
    let mut second_session = session(second, "two", 11);
    second_session.apply(PtySessionAction::MarkLost).unwrap();
    registry.insert(second_session).unwrap();
    assert_eq!(
        registry.request_close(second).unwrap(),
        PtyCloseDisposition::Closed
    );
    assert_eq!(registry.history().len(), 1);
    assert_eq!(registry.history()[0].id, second);
    assert_eq!(registry.dropped_history(), 1);
}
