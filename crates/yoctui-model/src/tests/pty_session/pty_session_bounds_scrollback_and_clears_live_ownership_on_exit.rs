use super::*;

#[test]
fn pty_session_bounds_scrollback_and_clears_live_ownership_on_exit() {
    let mut session = PtySession::new(spec(), 431).unwrap();
    session.apply(PtySessionAction::MarkRunning).unwrap();
    session.apply(PtySessionAction::Attach(client(1))).unwrap();
    session
        .apply(PtySessionAction::TakeControl {
            client: client(1),
            expected_epoch: 0,
        })
        .unwrap();
    session
        .apply(PtySessionAction::AdvanceScrollback(PtyScrollbackMetadata {
            first_sequence: 3,
            next_sequence: 10,
            retained_lines: 4,
            retained_cells: 20,
            retained_bytes: 20,
            dropped_lines: 3,
        }))
        .unwrap();
    session
        .apply(PtySessionAction::Exit(PtyExitStatus::Code(0)))
        .unwrap();
    assert_eq!(session.lifecycle, PtySessionLifecycle::Exited);
    assert_eq!(session.process_group, None);
    assert!(session.attached_clients.is_empty());
    assert_eq!(session.writer, None);
    assert_eq!(session.exit_status, Some(PtyExitStatus::Code(0)));
    assert_eq!(
        session.apply(PtySessionAction::MarkLost),
        Err(PtySessionError::InvalidTransition(
            PtySessionLifecycle::Exited
        ))
    );
}
