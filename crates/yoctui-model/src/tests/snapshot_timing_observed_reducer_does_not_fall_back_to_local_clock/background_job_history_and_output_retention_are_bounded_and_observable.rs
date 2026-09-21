use super::*;

#[test]
fn background_job_history_and_output_retention_are_bounded_and_observable() {
    let mut app = App::new(10, 1_000);
    app.background_jobs = BackgroundJobs::new(2, 2, 4);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(1, true)),
    );
    run_background_job(&mut app, 1);
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id: BackgroundJobId(1),
            result: BackgroundJobResult {
                summary: "done".into(),
                artifacts: vec![],
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        },
    );
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(2, true)),
    );
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(3, true)),
    );
    assert_eq!(app.background_jobs.jobs.len(), 2);
    assert_eq!(app.background_jobs.dropped_jobs, 1);
    assert!(app.background_jobs.get(BackgroundJobId(1)).is_none());

    let _ = update(
        &mut app,
        Action::AppendBackgroundJobOutput {
            id: BackgroundJobId(2),
            entry: BackgroundJobOutputEntry {
                severity: Severity::Warning,
                message: "abc".into(),
                source: BackgroundJobOutputSource::Backend,
                truncated: false,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        },
    );
    let _ = update(
        &mut app,
        Action::AppendBackgroundJobOutput {
            id: BackgroundJobId(2),
            entry: BackgroundJobOutputEntry {
                severity: Severity::Error,
                message: "de".into(),
                source: BackgroundJobOutputSource::Backend,
                truncated: false,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        },
    );
    let retained = app.background_jobs.get(BackgroundJobId(2)).unwrap();
    assert_eq!(retained.output.len(), 1);
    assert_eq!(retained.retained_output_bytes, 2);
    assert_eq!(retained.dropped_output_entries, 1);
    assert_eq!(retained.warnings, 1);
    assert_eq!(retained.errors, 1);

    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(4, true)),
    );
    assert_eq!(app.background_jobs.jobs.len(), 2);
    assert_eq!(app.background_jobs.rejected_jobs, 1);
}
