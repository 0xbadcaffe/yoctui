use super::*;

#[test]
fn raw_history_journal_updates_duplicate_request_idempotently_newest_first() {
    let base = daemon_snapshot_fixture();
    let mut journal = DaemonSnapshotJournal::new(base, DaemonSnapshotLimits::default()).unwrap();
    journal
        .publish(DaemonEvent::RawExecutionChanged(Box::new(
            terminal_raw_snapshot(1, RawExecutionOutcomeData::Failed),
        )))
        .unwrap();
    journal
        .publish(DaemonEvent::RawExecutionChanged(Box::new(
            terminal_raw_snapshot(2, RawExecutionOutcomeData::Succeeded),
        )))
        .unwrap();
    assert_eq!(journal.snapshot().raw_history.len(), 1);
    assert_eq!(
        journal.snapshot().raw_history[0].outcome,
        RawExecutionOutcomeData::Succeeded
    );

    let mut second = terminal_raw_snapshot(1, RawExecutionOutcomeData::Lost);
    second.request.request_id = "raw-request:second".into();
    second.result.as_mut().unwrap().durable_reference = None;
    journal
        .publish(DaemonEvent::RawExecutionChanged(Box::new(second)))
        .unwrap();
    assert_eq!(journal.snapshot().raw_history.len(), 2);
    assert!(
        journal
            .snapshot()
            .raw_history
            .windows(2)
            .all(|pair| { pair[0].ended_unix_ms >= pair[1].ended_unix_ms })
    );
}
