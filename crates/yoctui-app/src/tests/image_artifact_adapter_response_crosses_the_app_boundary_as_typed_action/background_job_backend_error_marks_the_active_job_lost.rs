use super::*;

#[test]
fn background_job_backend_error_marks_the_active_job_lost() {
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
        coordinator.backend_lost(
            "protocol framing failed".into(),
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        ),
    );
    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Lost);
    assert_eq!(
        job.error.as_ref().and_then(|error| error.detail.as_deref()),
        Some("protocol framing failed")
    );
    assert_eq!(coordinator.active_job_id(), None);
}
