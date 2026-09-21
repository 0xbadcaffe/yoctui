use super::*;

#[test]
fn daemon_snapshot_rejects_invalid_limits_and_oversized_snapshots() {
    assert!(matches!(
        DaemonSnapshotJournal::new(
            daemon_snapshot_fixture(),
            DaemonSnapshotLimits {
                retained_events: 0,
                ..DaemonSnapshotLimits::default()
            },
        ),
        Err(DaemonSnapshotError::InvalidLimit("retained events"))
    ));
    assert!(matches!(
        DaemonSnapshotJournal::new(
            daemon_snapshot_fixture(),
            DaemonSnapshotLimits {
                snapshot_bytes: 1,
                ..DaemonSnapshotLimits::default()
            },
        ),
        Err(DaemonSnapshotError::SnapshotTooLarge { .. })
    ));

    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    assert!(matches!(
        journal.publish(DaemonEvent::Log(LogRecord {
            source: "test".into(),
            severity: LogSeverity::Error,
            message: "x".repeat(MAX_FRAME_BYTES),
            unix_ms: 0,
            recipe: None,
            task: None,
            path: None,
            build: None,
        })),
        Err(DaemonSnapshotError::EventTooLarge { .. })
    ));
    assert_eq!(journal.snapshot().sequence, 0);
}
