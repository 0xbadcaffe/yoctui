use super::*;

#[tokio::test]
async fn pty_attach_prefix_detach_and_client_exit_leave_process_running() {
    let (root, spec) = fixture();
    let mut session = DaemonPtySession::start(
        spec,
        BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
        100,
        Duration::from_secs(1),
    )
    .await
    .unwrap();
    let first = PtyClientId([1; 16]);
    let initial = session.attach(first).unwrap();
    assert_eq!(initial.listing.lifecycle, PtyAttachLifecycle::Running);
    assert_eq!(initial.listing.viewers, 1);
    let epoch = session.take_control(first, 0).unwrap();
    session
        .resize(
            first,
            epoch,
            PtyDimensions {
                columns: 100,
                rows: 30,
            },
        )
        .unwrap();
    pump_until(&mut session, "ready").await;
    session
        .input(first, epoch, b"before-detach\n")
        .await
        .unwrap();
    pump_until(&mut session, "seen:before-detach").await;

    session.prefix_return(first).unwrap();
    assert!(session.is_process_active());
    assert_eq!(session.listing().unwrap().viewers, 0);
    assert_eq!(session.listing().unwrap().writer, None);

    let second = PtyClientId([2; 16]);
    let restored = session.attach(second).unwrap();
    assert!(restored.terminal.plain_text.contains("seen:before-detach"));
    assert_eq!(
        restored.terminal.dimensions,
        PtyDimensions {
            columns: 100,
            rows: 30
        }
    );
    assert_eq!(restored.listing.viewers, 1);
    let epoch = session
        .take_control(second, restored.listing.writer_epoch)
        .unwrap();
    session.client_disconnected(second).unwrap();
    assert!(session.is_process_active());
    assert_eq!(session.listing().unwrap().writer_epoch, epoch + 1);

    let third = PtyClientId([3; 16]);
    let restored = session.attach(third).unwrap();
    let epoch = session
        .take_control(third, restored.listing.writer_epoch)
        .unwrap();
    session.input(third, epoch, b"exit\n").await.unwrap();
    loop {
        if matches!(
            session.next_event().await.unwrap(),
            PtyAttachEvent::Exited(PtyExitStatus::Code(0))
        ) {
            break;
        }
    }
    assert_eq!(
        session.listing().unwrap().lifecycle,
        PtyAttachLifecycle::Exited
    );
    assert!(!session.is_process_active());
    fs::remove_dir_all(root).unwrap();
}
