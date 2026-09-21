use super::*;

#[test]
fn background_job_records_failure_and_loss() {
    let mut app = App::new(10, 1_000);
    for id in [1, 2] {
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(id, true)),
        );
        run_background_job(&mut app, id);
    }
    let _ = update(
        &mut app,
        Action::FailBackgroundJob {
            id: BackgroundJobId(1),
            error: BackgroundJobError {
                summary: "BitBake failed".into(),
                detail: Some("exit code 1".into()),
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(4),
        },
    );
    let _ = update(
        &mut app,
        Action::LoseBackgroundJob {
            id: BackgroundJobId(2),
            error: BackgroundJobError {
                summary: "bridge disconnected".into(),
                detail: None,
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
        },
    );

    assert_eq!(
        app.background_jobs.get(BackgroundJobId(1)).unwrap().status,
        BackgroundJobStatus::Failed
    );
    let lost = app.background_jobs.get(BackgroundJobId(2)).unwrap();
    assert_eq!(lost.status, BackgroundJobStatus::Lost);
    assert_eq!(
        lost.error.as_ref().map(|error| error.summary.as_str()),
        Some("bridge disconnected")
    );
}
