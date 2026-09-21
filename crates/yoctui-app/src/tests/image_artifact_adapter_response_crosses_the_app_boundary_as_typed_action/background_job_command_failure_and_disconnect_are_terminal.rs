use super::*;

#[test]
fn background_job_command_failure_and_disconnect_are_terminal() {
    let mut coordinator = BuildJobCoordinator::default();
    let mut app = App::new(10, 1_000);
    apply_actions(
        &mut app,
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap(),
    );
    let failed_id = coordinator.active_job_id().unwrap();
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(
            BackendEvent::CommandFailed {
                code: "start_failed".into(),
                message: "server rejected build".into(),
            },
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        ),
    );
    assert_eq!(
        app.background_jobs.get(failed_id).unwrap().status,
        BackgroundJobStatus::Failed
    );

    apply_actions(
        &mut app,
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap(),
    );
    let lost_id = coordinator.active_job_id().unwrap();
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(
            BackendEvent::Disconnected,
            SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        ),
    );
    assert_eq!(
        app.background_jobs.get(lost_id).unwrap().status,
        BackgroundJobStatus::Lost
    );
}
