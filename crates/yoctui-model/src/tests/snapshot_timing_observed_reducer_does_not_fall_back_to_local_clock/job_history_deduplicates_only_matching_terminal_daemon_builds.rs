use super::*;

#[test]
fn job_history_deduplicates_only_matching_terminal_daemon_builds() {
    let mut app = App::new(10, 1_000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.jobs.push(ClientDaemonJobSummary {
        id: 1,
        kind: ClientDaemonJobKind::BitBakeBuild,
        label: "BitBake build core-image-minimal".into(),
        lifecycle: ClientDaemonLifecycle::Exited,
        progress_current: Some(100),
        progress_total: Some(100),
        exit_code: Some(0),
    });
    for target in ["older-image", "core-image-minimal"] {
        app.build_history.push_back(BuildRecord {
            target: Some(target.into()),
            success: true,
            exit_code: Some(0),
            elapsed: None,
            completed_tasks: 1,
            warnings: 0,
            errors: 0,
        });
    }

    let rows = app.job_history_rows();
    assert_eq!(rows.len(), 2);
    assert!(matches!(rows[0], JobHistoryRowRef::Daemon(job) if job.id == 1));
    assert!(matches!(
        rows[1],
        JobHistoryRowRef::Build(record)
            if record.target.as_deref() == Some("older-image")
    ));
}
