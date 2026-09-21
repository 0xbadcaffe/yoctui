use super::*;

#[test]
fn background_job_cancellation_requires_capability_and_acknowledgement() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(1, true)),
    );
    let _ = update(
        &mut app,
        Action::RequestBackgroundJobCancellation {
            id: BackgroundJobId(1),
        },
    );
    assert_eq!(
        app.background_jobs.get(BackgroundJobId(1)).unwrap().status,
        BackgroundJobStatus::Cancelling
    );
    let _ = update(
        &mut app,
        Action::CancelBackgroundJob {
            id: BackgroundJobId(1),
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        },
    );
    assert_eq!(
        app.background_jobs.get(BackgroundJobId(1)).unwrap().status,
        BackgroundJobStatus::Cancelled
    );

    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(2, false)),
    );
    run_background_job(&mut app, 2);
    let ignored_before = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::RequestBackgroundJobCancellation {
            id: BackgroundJobId(2),
        },
    );
    assert_eq!(
        app.background_jobs.get(BackgroundJobId(2)).unwrap().status,
        BackgroundJobStatus::Running
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored_before + 1);
}
