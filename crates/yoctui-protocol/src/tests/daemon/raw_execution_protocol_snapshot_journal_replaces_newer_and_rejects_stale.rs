use super::*;

#[test]
fn raw_execution_protocol_snapshot_journal_replaces_newer_and_rejects_stale() {
    let mut base = daemon_snapshot_fixture();
    base.raw_executions = vec![raw_execution_snapshot_fixture(1)];
    let mut journal = DaemonSnapshotJournal::new(base, DaemonSnapshotLimits::default()).unwrap();
    journal
        .publish(DaemonEvent::RawExecutionChanged(Box::new(
            raw_execution_snapshot_fixture(2),
        )))
        .unwrap();
    assert_eq!(journal.snapshot().raw_executions[0].sequence, 2);
    let before = journal.snapshot().clone();
    assert!(matches!(
        journal.publish(DaemonEvent::RawExecutionChanged(Box::new(
            raw_execution_snapshot_fixture(2)
        ))),
        Err(DaemonSnapshotError::StaleRawExecution { .. })
    ));
    assert_eq!(journal.snapshot(), &before);
}
