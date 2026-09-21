use super::*;

#[test]
fn daemon_journal_uses_conservative_headroom_between_snapshot_serializations() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::TaskStarted {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            pid: Some(42),
            worker: None,
            log_path: None,
            stats: None,
            started_unix_ms: None,
        }))
        .unwrap();
    for progress in (0..100).cycle().take(10_000) {
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::TaskProgress {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(progress),
            }))
            .unwrap();
    }
    let metrics = journal.ipc_metrics();
    let exact = serde_json::to_vec(journal.snapshot()).unwrap().len();
    assert_eq!(metrics.published_events, 10_001);
    assert!(metrics.published_event_bytes > 0);
    assert!(metrics.snapshot_serializations <= 2, "{metrics:?}");
    assert!(exact <= metrics.snapshot_bytes_upper_bound);
    assert!(metrics.snapshot_bytes_upper_bound <= MAX_FRAME_BYTES);
    assert!(matches!(
        journal.snapshot().build_events.last(),
        Some(DaemonBuildEvent::TaskProgress {
            progress: Some(99),
            ..
        })
    ));
}
