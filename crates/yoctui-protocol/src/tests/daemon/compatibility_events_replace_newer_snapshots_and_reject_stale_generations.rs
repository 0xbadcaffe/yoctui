use super::*;

#[test]
fn compatibility_events_replace_newer_snapshots_and_reject_stale_generations() {
    let mut initial = daemon_snapshot_fixture();
    initial.compatibility = Some(compatibility_snapshot_fixture(1));
    let mut journal = DaemonSnapshotJournal::new(initial, DaemonSnapshotLimits::default())
        .expect("valid compatibility snapshot");

    journal
        .publish(DaemonEvent::CompatibilityChanged(Box::new(
            compatibility_snapshot_fixture(2),
        )))
        .unwrap();
    assert_eq!(
        journal
            .snapshot()
            .compatibility
            .as_ref()
            .unwrap()
            .generation,
        2
    );

    assert!(matches!(
        journal.publish(DaemonEvent::CompatibilityChanged(Box::new(
            compatibility_snapshot_fixture(2)
        ))),
        Err(DaemonSnapshotError::StaleCompatibilityGeneration {
            current: 2,
            received: 2
        })
    ));
    assert_eq!(journal.snapshot().sequence, 1);
}
