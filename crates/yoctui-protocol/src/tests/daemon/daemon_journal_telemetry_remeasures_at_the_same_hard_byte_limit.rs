use super::*;

#[test]
fn daemon_journal_telemetry_remeasures_at_the_same_hard_byte_limit() {
    let snapshot = daemon_snapshot_fixture();
    let limit = serde_json::to_vec(&snapshot).unwrap().len() + 4_096;
    let mut journal = DaemonSnapshotJournal::new(
        snapshot,
        DaemonSnapshotLimits {
            snapshot_bytes: limit,
            retained_events: 4,
            ..DaemonSnapshotLimits::default()
        },
    )
    .unwrap();
    for uptime in 1..=100 {
        journal.publish(journal_telemetry(uptime)).unwrap();
        let exact = serde_json::to_vec(journal.snapshot()).unwrap().len();
        assert!(exact <= journal.ipc_metrics().snapshot_bytes_upper_bound);
        assert!(journal.ipc_metrics().snapshot_bytes_upper_bound <= limit);
        assert!(journal.events.len() <= 4);
    }
    let serializations = journal.ipc_metrics().snapshot_serializations;
    assert!(
        serializations > 1,
        "size ledger must eventually be remeasured"
    );
    assert!(serializations < 100, "headroom must amortize snapshot work");
}
