use super::*;

#[test]
fn background_job_cancellation_failure_recovers_then_acknowledges() {
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
        coordinator.actions_for_backend_event(BackendEvent::BuildStarted, SystemTime::UNIX_EPOCH),
    );
    assert_eq!(update(&mut app, Action::Cancel), None);
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::BuildCancellationConfirmation)
    ));
    assert!(matches!(
        update(&mut app, Action::ConfirmBuildCancellation),
        Some(yoctui_model::Effect::Cancel)
    ));
    apply_actions(&mut app, vec![coordinator.request_cancellation().unwrap()]);
    apply_actions(
        &mut app,
        coordinator.cancellation_failed(
            "backend refused".into(),
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        ),
    );
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        BackgroundJobStatus::Running
    );
    assert_eq!(app.build.status, BuildStatus::Running);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("may still be running")
    );

    assert_eq!(update(&mut app, Action::Cancel), None);
    assert!(matches!(
        update(&mut app, Action::ConfirmBuildCancellation),
        Some(yoctui_model::Effect::Cancel)
    ));
    apply_actions(&mut app, vec![coordinator.request_cancellation().unwrap()]);
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(
            BackendEvent::BuildCompleted {
                success: false,
                exit_code: None,
            },
            SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        ),
    );
    assert_eq!(app.build.status, BuildStatus::Cancelled);
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        BackgroundJobStatus::Cancelled
    );
}
