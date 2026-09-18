use super::*;

#[test]
fn test_workflow_cli_correlates_managed_bitbake_success_and_cancellation() {
    fn queued_test_build() -> (App, BuildJobCoordinator, TestSessionId, BuildRequest) {
        let mut app = App::new(100, 100_000);
        app.screen = Screen::Testing;
        app.test_family_selection = yoctui_model::TestFamily::TestImage;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace
            .variables
            .insert("DISTRO".into(), "poky".into());
        app.build.target = Some("core-image-minimal".into());
        let _ = update(&mut app, Action::BeginSelectedTestLaunch);
        let _ = update(&mut app, Action::PreviewTestLaunch);
        let Some(Effect::StartTestBuildSession {
            id,
            family: _,
            request,
        }) = update(&mut app, Action::ConfirmTestLaunch)
        else {
            panic!("expected managed Testing build");
        };
        let mut coordinator = BuildJobCoordinator::default();
        for action in coordinator
            .queue_build(&request, SystemTime::UNIX_EPOCH)
            .unwrap()
        {
            let _ = update(&mut app, action);
        }
        let background_job_id = coordinator.active_job_id().unwrap();
        let _ = update(
            &mut app,
            Action::AttachTestBuildSession {
                id,
                background_job_id,
            },
        );
        (app, coordinator, id, request)
    }

    let (mut succeeded, mut success_jobs, success_id, request) = queued_test_build();
    assert_eq!(request.task.as_deref(), Some("testimage"));
    for event in [
        BackendEvent::BuildStarted,
        BackendEvent::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    ] {
        if let Some(action) = test_build_action_for_event(&succeeded, success_id, &event) {
            let _ = update(&mut succeeded, action);
        }
        for action in success_jobs.job_actions_for_event(&event, SystemTime::UNIX_EPOCH) {
            let _ = update(&mut succeeded, action);
        }
    }
    assert_eq!(
        succeeded.test_session(success_id).unwrap().outcome,
        Some(yoctui_model::TestSessionOutcome::Succeeded)
    );

    let (mut cancelled, mut cancel_jobs, cancel_id, _) = queued_test_build();
    if let Some(action) =
        test_build_action_for_event(&cancelled, cancel_id, &BackendEvent::BuildStarted)
    {
        let _ = update(&mut cancelled, action);
    }
    for action in
        cancel_jobs.job_actions_for_event(&BackendEvent::BuildStarted, SystemTime::UNIX_EPOCH)
    {
        let _ = update(&mut cancelled, action);
    }
    let _ = update(&mut cancelled, Action::BeginActiveTestSessionCancellation);
    assert!(matches!(
        update(&mut cancelled, Action::ConfirmTestSessionCancellation),
        Some(Effect::CancelTestSession(id)) if id == cancel_id
    ));
    let event = BackendEvent::BuildCompleted {
        success: false,
        exit_code: Some(130),
    };
    let action = test_build_action_for_event(&cancelled, cancel_id, &event).unwrap();
    let _ = update(&mut cancelled, action);
    for action in cancel_jobs.job_actions_for_event(&event, SystemTime::UNIX_EPOCH) {
        let _ = update(&mut cancelled, action);
    }
    assert_eq!(
        cancelled.test_session(cancel_id).unwrap().outcome,
        Some(yoctui_model::TestSessionOutcome::Cancelled)
    );
}
