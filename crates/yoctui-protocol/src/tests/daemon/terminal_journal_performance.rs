use super::*;

#[test]
fn terminal_output_and_lease_updates_do_not_serialize_the_entire_snapshot() {
    let mut snapshot = daemon_snapshot_fixture();
    snapshot
        .recovery_warnings
        .push("retained build context ".repeat(8192));
    let mut journal =
        DaemonSnapshotJournal::new(snapshot, DaemonSnapshotLimits::default()).unwrap();
    let serializations = journal.ipc_metrics().snapshot_serializations;
    for sequence in 1..=1000 {
        let event = journal
            .publish(DaemonEvent::PtyOutput {
                session_id: PtySessionId(1),
                bytes: b"terminal output\r\n".to_vec(),
            })
            .unwrap();
        assert_eq!(event.sequence, sequence);
    }
    journal
        .publish(DaemonEvent::PtyChanged(PtySessionSummary {
            id: PtySessionId(1),
            name: "shell".into(),
            kind: PtyKind::BuildShell,
            cwd: "/tmp".into(),
            lifecycle: LifecycleState::Running,
            dimensions: TerminalDimensions {
                columns: 80,
                rows: 24,
            },
            writer: Some(client_id(2)),
            writer_epoch: 1,
            viewers: 1,
            exit_code: None,
            restartable: true,
        }))
        .unwrap();
    assert_eq!(journal.snapshot().pty_sessions[0].writer_epoch, 1);
    journal
        .publish(DaemonEvent::PtyRemoved {
            session_id: PtySessionId(1),
        })
        .unwrap();
    assert!(journal.snapshot().pty_sessions.is_empty());
    assert_eq!(
        journal.ipc_metrics().snapshot_serializations,
        serializations
    );
    let exact = serde_json::to_vec(journal.snapshot()).unwrap().len();
    assert!(exact <= journal.ipc_metrics().snapshot_bytes_upper_bound);
    assert!(journal.ipc_metrics().snapshot_bytes_upper_bound <= MAX_FRAME_BYTES);
}

#[test]
fn terminal_output_size_rejection_keeps_snapshot_and_replay_transactional() {
    let mut snapshot = daemon_snapshot_fixture();
    snapshot.sequence = 9;
    snapshot.generation = 9;
    let limit = serde_json::to_vec(&snapshot).unwrap().len();
    let mut journal = DaemonSnapshotJournal::new(
        snapshot.clone(),
        DaemonSnapshotLimits {
            snapshot_bytes: limit,
            ..DaemonSnapshotLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        journal.publish(DaemonEvent::PtyOutput {
            session_id: PtySessionId(1),
            bytes: vec![b'x'],
        }),
        Err(DaemonSnapshotError::SnapshotTooLarge { .. })
    ));
    assert_eq!(journal.snapshot(), &snapshot);
    assert_eq!(journal.ipc_metrics().published_events, 0);
    let DaemonSnapshotSync::Replay { events, .. } = journal.synchronize(Some(ResumeCursor {
        daemon_instance_id: snapshot.daemon_instance_id,
        last_sequence: 9,
    })) else {
        panic!("unchanged cursor should remain replayable");
    };
    assert!(events.is_empty());
}
