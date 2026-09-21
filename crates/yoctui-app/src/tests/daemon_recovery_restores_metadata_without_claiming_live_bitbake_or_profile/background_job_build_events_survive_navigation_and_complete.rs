use super::*;

#[test]
fn background_job_build_events_survive_navigation_and_complete() {
    let mut coordinator = BuildJobCoordinator::default();
    let mut app = App::new(10, 1_000);
    apply_actions(
        &mut app,
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap(),
    );
    let id = coordinator.active_job_id().unwrap();
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        BackgroundJobStatus::Starting
    );
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(
            BackendEvent::BuildStarted,
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        ),
    );
    let _ = update(&mut app, Action::Open(Screen::Layers));
    let log = yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Warning,
        message: "cache miss".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        build: None,
        protected: false,
        diagnostic: None,
    };
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(
            BackendEvent::Log(log),
            SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        ),
    );
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(
            BackendEvent::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
            SystemTime::UNIX_EPOCH + Duration::from_secs(3),
        ),
    );

    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(app.screen, Screen::Layers);
    assert_eq!(app.build.status, BuildStatus::Completed);
    assert_eq!(job.status, BackgroundJobStatus::Succeeded);
    assert_eq!(job.output.len(), 1);
    assert_eq!(job.warnings, 1);
    assert_eq!(coordinator.active_job_id(), None);
}
