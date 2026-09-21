use super::*;

#[test]
fn background_job_start_failure_finishes_the_queued_job() {
    let mut coordinator = BuildJobCoordinator::default();
    let mut app = App::new(10, 1_000);
    apply_actions(
        &mut app,
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap(),
    );
    let id = coordinator.active_job_id().unwrap();
    apply_actions(
        &mut app,
        coordinator.start_failed(
            "executable not found".into(),
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        ),
    );
    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Failed);
    assert_eq!(
        job.error.as_ref().and_then(|error| error.detail.as_deref()),
        Some("executable not found")
    );
    assert_eq!(coordinator.active_job_id(), None);
}
