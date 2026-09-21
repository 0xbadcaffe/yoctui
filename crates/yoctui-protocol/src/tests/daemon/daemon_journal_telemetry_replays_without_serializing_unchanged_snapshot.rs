use super::*;

#[test]
fn daemon_journal_telemetry_replays_without_serializing_unchanged_snapshot() {
    let mut snapshot = daemon_snapshot_fixture();
    snapshot.recovery_warnings = vec!["retained metadata with \\".repeat(8_192)];
    let mut replica = snapshot.clone();
    let cursor = ResumeCursor {
        daemon_instance_id: snapshot.daemon_instance_id,
        last_sequence: snapshot.sequence,
    };
    let mut journal =
        DaemonSnapshotJournal::new(snapshot.clone(), DaemonSnapshotLimits::default()).unwrap();
    let initial_serializations = journal.ipc_metrics().snapshot_serializations;
    for uptime in 1..=100 {
        let published = journal.publish(journal_telemetry(uptime)).unwrap();
        assert_eq!(published.sequence, uptime);
        assert_eq!(published.generation, uptime);
        assert_eq!(published.event, journal_telemetry(uptime));
    }
    assert_eq!(
        journal.ipc_metrics().snapshot_serializations,
        initial_serializations,
        "telemetry must not serialize unchanged retained metadata while headroom exists"
    );
    let DaemonSnapshotSync::Replay {
        events,
        replayed_through,
    } = journal.synchronize(Some(cursor))
    else {
        panic!("retained telemetry must replay incrementally");
    };
    assert_eq!(events.len(), 100);
    assert_eq!(replayed_through, 100);
    for event in events {
        let frame = encode_frame(&ServerMessage::Event(event.clone())).unwrap();
        assert_eq!(
            decode_frame::<ServerMessage>(&frame).unwrap(),
            ServerMessage::Event(event.clone())
        );
        apply_sequenced_event(&mut replica, &event).unwrap();
    }
    snapshot.sequence = 100;
    snapshot.generation = 100;
    assert_eq!(journal.snapshot(), &snapshot);
    assert_eq!(replica, snapshot);
    assert_eq!(journal.ipc_metrics().published_events, 100);
    let exact = serde_json::to_vec(journal.snapshot()).unwrap().len();
    assert!(exact <= journal.ipc_metrics().snapshot_bytes_upper_bound);
    assert!(journal.ipc_metrics().snapshot_bytes_upper_bound <= MAX_FRAME_BYTES);
}
