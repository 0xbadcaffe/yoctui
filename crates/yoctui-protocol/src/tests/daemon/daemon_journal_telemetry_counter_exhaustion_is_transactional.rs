use super::*;

#[test]
fn daemon_journal_telemetry_counter_exhaustion_is_transactional() {
    for sequence_exhausted in [true, false] {
        let mut snapshot = daemon_snapshot_fixture();
        if sequence_exhausted {
            snapshot.sequence = u64::MAX;
        } else {
            snapshot.generation = u64::MAX;
        }
        let mut journal =
            DaemonSnapshotJournal::new(snapshot.clone(), DaemonSnapshotLimits::default()).unwrap();
        let result = journal.publish(journal_telemetry(1));
        if sequence_exhausted {
            assert!(matches!(
                result,
                Err(DaemonSnapshotError::SequenceExhausted)
            ));
        } else {
            assert!(matches!(
                result,
                Err(DaemonSnapshotError::GenerationExhausted)
            ));
        }
        assert_eq!(journal.snapshot(), &snapshot);
        assert!(journal.events.is_empty());
        assert_eq!(journal.ipc_metrics().published_events, 0);
        assert_eq!(journal.ipc_metrics().snapshot_serializations, 1);
    }
}
