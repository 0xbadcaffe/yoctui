use super::*;

#[test]
fn background_job_summary_counts_exact_states_and_current_daemon_ownership() {
    let mut app = App::new(10, 1_000);
    for id in 1..=4 {
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(id, true)),
        );
    }
    run_background_job(&mut app, 2);
    let _ = update(
        &mut app,
        Action::StartBackgroundJob {
            id: BackgroundJobId(3),
            started_at: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
        },
    );
    let _ = update(
        &mut app,
        Action::RunBackgroundJob {
            id: BackgroundJobId(3),
        },
    );
    let _ = update(
        &mut app,
        Action::FailBackgroundJob {
            id: BackgroundJobId(3),
            error: BackgroundJobError {
                summary: "failed".into(),
                detail: None,
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(4),
        },
    );
    run_background_job(&mut app, 4);
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id: BackgroundJobId(4),
            result: BackgroundJobResult {
                summary: "done".into(),
                artifacts: Vec::new(),
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
        },
    );
    app.build_history.push_back(BuildRecord {
        target: Some("already represented".into()),
        success: false,
        exit_code: Some(1),
        elapsed: Some(Duration::from_secs(5)),
        completed_tasks: 1,
        warnings: 0,
        errors: 1,
    });
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.jobs.push(ClientDaemonJobSummary {
        id: 90,
        kind: ClientDaemonJobKind::Utility,
        label: "daemon job".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        progress_current: None,
        progress_total: None,
        exit_code: None,
    });

    assert_eq!(
        app.job_summary(),
        JobSummary {
            active: 2,
            queued: 1,
            failed: 1,
            recent_completed: 2,
            daemon_owned: Some(1),
        }
    );
    app.daemon.status = ClientReplicaStatus::Stale;
    assert_eq!(app.job_summary().daemon_owned, None);
}
