use super::*;

#[test]
fn pty_session_enforces_single_writer_epochs_resize_and_detach() {
    let mut session = PtySession::new(spec(), 431).unwrap();
    session.apply(PtySessionAction::MarkRunning).unwrap();
    session.apply(PtySessionAction::Attach(client(1))).unwrap();
    session.apply(PtySessionAction::Attach(client(2))).unwrap();
    session
        .apply(PtySessionAction::TakeControl {
            client: client(1),
            expected_epoch: 0,
        })
        .unwrap();
    assert_eq!(session.writer.unwrap().epoch, 1);
    assert_eq!(
        session.apply(PtySessionAction::TakeControl {
            client: client(2),
            expected_epoch: 1,
        }),
        Err(PtySessionError::WriterBusy)
    );
    assert_eq!(
        session.apply(PtySessionAction::Resize {
            client: client(2),
            writer_epoch: 1,
            dimensions: PtyDimensions {
                columns: 80,
                rows: 24
            },
        }),
        Err(PtySessionError::NotWriter)
    );
    session.apply(PtySessionAction::Detach(client(1))).unwrap();
    assert_eq!(session.writer, None);
    assert_eq!(session.writer_epoch, 2);
    assert_eq!(session.attached_clients, BTreeSet::from([client(2)]));
}
