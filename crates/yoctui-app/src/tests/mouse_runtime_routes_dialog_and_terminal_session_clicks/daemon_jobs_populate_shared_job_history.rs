use super::*;

#[test]
fn daemon_jobs_populate_shared_job_history() {
    use yoctui_protocol::daemon::{JobId, JobKind, JobSummary, LifecycleState};

    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([3; 16]),
        123,
        "job-history".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    snapshot.jobs = vec![
        JobSummary {
            id: JobId(8),
            kind: JobKind::BitBakeBuild,
            label: "BitBake build core-image-minimal".into(),
            lifecycle: LifecycleState::Running,
            progress_current: Some(59),
            progress_total: Some(100),
            exit_code: None,
        },
        JobSummary {
            id: JobId(7),
            kind: JobKind::Raw,
            label: "Raw bitbake -s".into(),
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code: Some(0),
        },
    ];

    let mut app = yoctui_model::App::new(16, 4096);
    app.build_history.push_back(yoctui_model::BuildRecord {
        target: Some("core-image-minimal".into()),
        success: true,
        exit_code: Some(0),
        elapsed: None,
        completed_tasks: 10,
        warnings: 0,
        errors: 0,
    });
    let mut client = DaemonClientSnapshot::default();
    client.replace_app(&mut app, snapshot);

    let rows = app.job_history_rows();
    assert_eq!(
        rows.len(),
        3,
        "a running daemon build must not hide a prior retained completion"
    );
    assert!(matches!(
        rows[0],
        yoctui_model::JobHistoryRowRef::Daemon(job)
            if job.id == 8
                && job.kind == yoctui_model::ClientDaemonJobKind::BitBakeBuild
                && job.progress_current == Some(59)
    ));
    assert!(matches!(
        rows[1],
        yoctui_model::JobHistoryRowRef::Daemon(job)
            if job.id == 7 && job.exit_code == Some(0)
    ));
    assert!(matches!(
        rows[2],
        yoctui_model::JobHistoryRowRef::Build(record)
            if record.target.as_deref() == Some("core-image-minimal")
    ));
    assert_eq!(app.job_summary().active, 1);
    assert_eq!(app.job_summary().recent_completed, 1);
}
