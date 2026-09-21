use super::*;

#[test]
fn background_job_invalid_transitions_leave_state_unchanged() {
    let mut app = App::new(10, 1_000);
    let id = BackgroundJobId(1);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(1, true)),
    );
    let _ = update(&mut app, Action::RunBackgroundJob { id });
    let _ = update(
        &mut app,
        Action::UpdateBackgroundJobProgress {
            id,
            progress: BackgroundJobProgress::Percent(101),
        },
    );
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        BackgroundJobStatus::Queued
    );
    assert_eq!(app.background_jobs.ignored_transitions, 2);

    run_background_job(&mut app, 1);
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id,
            result: BackgroundJobResult {
                summary: "done".into(),
                artifacts: vec![],
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        },
    );
    let _ = update(
        &mut app,
        Action::FailBackgroundJob {
            id,
            error: BackgroundJobError {
                summary: "late failure".into(),
                detail: None,
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
        },
    );
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        BackgroundJobStatus::Succeeded
    );
    assert_eq!(app.background_jobs.ignored_transitions, 3);
}
