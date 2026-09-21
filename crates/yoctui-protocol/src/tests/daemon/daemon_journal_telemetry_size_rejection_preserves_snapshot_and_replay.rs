use super::*;

#[test]
fn daemon_journal_telemetry_size_rejection_preserves_snapshot_and_replay() {
    let mut snapshot = daemon_snapshot_fixture();
    snapshot.sequence = 8;
    snapshot.generation = 8;
    let limit = serde_json::to_vec(&snapshot).unwrap().len() + 1;
    let mut journal = DaemonSnapshotJournal::new(
        snapshot,
        DaemonSnapshotLimits {
            snapshot_bytes: limit,
            ..DaemonSnapshotLimits::default()
        },
    )
    .unwrap();
    journal.publish(journal_telemetry(1)).unwrap();
    let before = journal.snapshot().clone();
    let before_events = journal.events.clone();
    let before_metrics = journal.ipc_metrics();
    // Both sequence counters grow from one digit to two; the hard limit
    // leaves only one byte, so even non-retained telemetry must fail.
    assert!(matches!(
        journal.publish(journal_telemetry(2)),
        Err(DaemonSnapshotError::SnapshotTooLarge { .. })
    ));
    assert_eq!(journal.snapshot(), &before);
    assert_eq!(journal.events, before_events);
    assert_eq!(
        journal.ipc_metrics().published_events,
        before_metrics.published_events
    );
    assert_eq!(
        journal.ipc_metrics().published_event_bytes,
        before_metrics.published_event_bytes
    );
    assert_eq!(
        journal.ipc_metrics().snapshot_bytes_upper_bound,
        before_metrics.snapshot_bytes_upper_bound
    );
}
