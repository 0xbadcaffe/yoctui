use super::*;

#[test]
fn daemon_snapshot_evicts_oldest_logs_to_preserve_byte_bound() {
    let mut journal = DaemonSnapshotJournal::new(
        daemon_snapshot_fixture(),
        DaemonSnapshotLimits {
            retained_events: 16,
            recent_logs: 16,
            snapshot_bytes: 2_048,
        },
    )
    .unwrap();
    for index in 1..=4 {
        journal
            .publish(DaemonEvent::Log(LogRecord {
                source: "bitbake".into(),
                severity: LogSeverity::Info,
                message: "x".repeat(900),
                unix_ms: index,
                recipe: None,
                task: None,
                path: None,
                build: None,
            }))
            .unwrap();
    }
    let encoded = serde_json::to_vec(journal.snapshot()).unwrap();
    assert!(encoded.len() <= 2_048);
    assert!(journal.snapshot().recent_logs.len() < 4);
    assert_eq!(journal.snapshot().sequence, 4);
}
