use super::*;

#[test]
fn daemon_job_identity_recovery_preserves_terminal_rows_and_raw_namespace() {
    use yoctui_protocol::daemon::{DaemonEvent, DaemonSnapshotJournal, DaemonSnapshotLimits};
    use yoctui_protocol::daemon_persist::{
        DaemonPersistedState, PersistedPreferences, recover_persisted_snapshot,
    };
    let mut before = snapshot(&[1, 2, (5 << 60) | 42]);
    before.jobs[2].kind = JobKind::Raw;
    let persisted = DaemonPersistedState::capture(
        &before,
        10,
        "fixture-boot".into(),
        Vec::new(),
        PersistedPreferences::default(),
    );
    let (recovered, _) = recover_persisted_snapshot(snapshot(&[]), &persisted, "fixture-boot");
    let ids = DaemonJobIds::from_snapshot(&recovered);
    let mut new_job = before.jobs[0].clone();
    new_job.id = ids.allocate().unwrap();
    new_job.label = "new build".into();
    new_job.lifecycle = LifecycleState::Connecting;
    new_job.exit_code = None;
    assert_eq!(new_job.id, JobId(3));
    let mut journal =
        DaemonSnapshotJournal::new(recovered, DaemonSnapshotLimits::default()).unwrap();
    journal.publish(DaemonEvent::JobChanged(new_job)).unwrap();
    assert_eq!(journal.snapshot().jobs.len(), 4);
    assert_eq!(&journal.snapshot().jobs[..3], &before.jobs);
    assert_eq!(
        DaemonJobIds::from_snapshot(journal.snapshot()).allocate(),
        Ok(JobId(4))
    );
}
