use super::*;

#[test]
fn daemon_journal_updates_high_rate_job_progress_without_full_snapshot_serialization() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    for completed in 0..10_000 {
        journal
            .publish(DaemonEvent::JobChanged(JobSummary {
                id: JobId(41),
                kind: JobKind::BitBakeBuild,
                label: "BitBake build core-image-sato".into(),
                lifecycle: LifecycleState::Running,
                progress_current: Some(completed),
                progress_total: Some(10_000),
                exit_code: None,
            }))
            .unwrap();
    }
    let metrics = journal.ipc_metrics();
    let exact = serde_json::to_vec(journal.snapshot()).unwrap().len();
    assert_eq!(metrics.published_events, 10_000);
    assert!(metrics.snapshot_serializations <= 2, "{metrics:?}");
    assert!(exact <= metrics.snapshot_bytes_upper_bound);
    assert_eq!(journal.snapshot().jobs[0].progress_current, Some(9_999));
}
