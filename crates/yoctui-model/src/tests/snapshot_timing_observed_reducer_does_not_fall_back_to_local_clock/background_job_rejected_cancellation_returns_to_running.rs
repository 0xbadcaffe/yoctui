use super::*;

#[test]
fn background_job_rejected_cancellation_returns_to_running() {
    let mut app = App::new(10, 1_000);
    let id = BackgroundJobId(1);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(1, true)),
    );
    run_background_job(&mut app, 1);
    let _ = update(&mut app, Action::RequestBackgroundJobCancellation { id });
    let _ = update(&mut app, Action::RejectBackgroundJobCancellation { id });
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        BackgroundJobStatus::Running
    );
}
