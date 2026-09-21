use super::*;

#[test]
fn multi_client_fanout_replays_global_events_from_independent_cursors() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    journal
        .publish(DaemonEvent::RecoveryWarning {
            message: "shared".into(),
        })
        .unwrap();
    let cursor = ResumeCursor {
        daemon_instance_id: journal.snapshot().daemon_instance_id,
        last_sequence: 0,
    };
    let first = journal.synchronize(Some(cursor));
    let second = journal.synchronize(Some(cursor));
    let events = |sync| match sync {
        DaemonSnapshotSync::Replay { events, .. } => events,
        _ => panic!("expected replay"),
    };
    assert_eq!(events(first), events(second));
}
