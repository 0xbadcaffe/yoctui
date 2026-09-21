use super::*;

#[test]
fn task_identity_unresolved_job_statistics_preserve_aggregate_without_rows() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    let job = |kind, completed| {
        DaemonEvent::JobChanged(JobSummary {
            id: JobId(99),
            kind,
            label: "build".into(),
            lifecycle: LifecycleState::Running,
            progress_current: Some(completed),
            progress_total: Some(6812),
            exit_code: None,
        })
    };
    journal.publish(job(JobKind::BitBakeBuild, 100)).unwrap();
    assert_eq!(
        journal.snapshot().build_progress,
        None,
        "legacy absence is not a current build"
    );
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Started {
            started_unix_ms: None,
        }))
        .unwrap();
    journal.publish(job(JobKind::BitBakeBuild, 2340)).unwrap();
    assert_eq!(
        journal.snapshot().build_progress,
        Some(DaemonBuildProgress {
            completed: 2340,
            total: Some(6812),
            ..Default::default()
        })
    );
    assert_eq!(journal.snapshot().build_events.len(), 1);
    journal.publish(job(JobKind::Qa, 4000)).unwrap();
    assert_eq!(journal.snapshot().build_progress.unwrap().completed, 2340);
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Disconnected))
        .unwrap();
    journal.publish(job(JobKind::BitBakeBuild, 4500)).unwrap();
    assert_eq!(
        journal.snapshot().build_progress.unwrap().completed,
        2340,
        "lost authority cannot take later progress"
    );
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Completed {
            success: false,
            exit_code: Some(1),
            finished_unix_ms: None,
        }))
        .unwrap();
    journal.publish(job(JobKind::BitBakeBuild, 5000)).unwrap();
    assert_eq!(
        journal.snapshot().build_progress.unwrap().completed,
        2340,
        "late progress cannot change terminal authority"
    );
}
