use super::*;

#[test]
fn background_job_coordinator_prevents_duplicate_active_builds() {
    let mut coordinator = BuildJobCoordinator::default();
    assert!(
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .is_some()
    );
    assert!(
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .is_none()
    );
    assert_eq!(coordinator.active_job_id(), Some(BackgroundJobId(1)));
}
