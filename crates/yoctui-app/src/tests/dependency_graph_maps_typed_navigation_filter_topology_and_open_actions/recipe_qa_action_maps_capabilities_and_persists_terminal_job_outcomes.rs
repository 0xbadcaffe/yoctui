use super::*;

#[test]
fn recipe_qa_action_maps_capabilities_and_persists_terminal_job_outcomes() {
    assert_eq!(
        recipes_workspace_action(false, Input::Char('V')),
        Some(Action::BeginSelectedRecipeCveCheck)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('X')),
        Some(Action::BeginSelectedRecipeSpdx)
    );

    let cve = BuildRequest {
        targets: vec!["busybox".into()],
        task: Some("cve_check".into()),
        force: false,
    };
    let mut coordinator = BuildJobCoordinator::default();
    let mut app = App::new(20, 4_000);
    let queued = coordinator
        .queue_build(&cve, SystemTime::UNIX_EPOCH)
        .unwrap();
    assert!(matches!(
        &queued[0],
        Action::QueueBackgroundJob(spec)
            if spec.kind == BackgroundJobKind::CveCheck
                && spec.context.workspace == Some(Screen::Recipes)
                && spec.context.recipe.as_deref() == Some("busybox")
                && spec.context.task.as_deref() == Some("cve_check")
    ));
    for action in queued {
        let _ = yoctui_model::update(&mut app, action);
    }
    for action in
        coordinator.actions_for_backend_event(BackendEvent::BuildStarted, SystemTime::UNIX_EPOCH)
    {
        let _ = yoctui_model::update(&mut app, action);
    }
    for action in coordinator.actions_for_backend_event(
        BackendEvent::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
        SystemTime::UNIX_EPOCH,
    ) {
        let _ = yoctui_model::update(&mut app, action);
    }
    let cve_job = app.background_jobs.jobs.back().unwrap();
    assert_eq!(cve_job.status, BackgroundJobStatus::Succeeded);
    assert!(
        cve_job
            .result
            .as_ref()
            .unwrap()
            .summary
            .contains("no result path")
    );
    assert!(cve_job.result.as_ref().unwrap().artifacts.is_empty());

    let spdx = BuildRequest {
        targets: vec!["busybox".into()],
        task: Some("create_spdx".into()),
        force: false,
    };
    for action in coordinator
        .queue_build(&spdx, SystemTime::UNIX_EPOCH)
        .unwrap()
    {
        let _ = yoctui_model::update(&mut app, action);
    }
    assert!(
        coordinator
            .queue_build(&spdx, SystemTime::UNIX_EPOCH)
            .is_none()
    );
    let cancellation = coordinator.request_cancellation().unwrap();
    let _ = yoctui_model::update(&mut app, cancellation);
    for action in coordinator.actions_for_backend_event(
        BackendEvent::BuildCompleted {
            success: false,
            exit_code: Some(130),
        },
        SystemTime::UNIX_EPOCH,
    ) {
        let _ = yoctui_model::update(&mut app, action);
    }
    assert_eq!(
        app.background_jobs.jobs.back().unwrap().status,
        BackgroundJobStatus::Cancelled
    );

    for action in coordinator
        .queue_build(&cve, SystemTime::UNIX_EPOCH)
        .unwrap()
    {
        let _ = yoctui_model::update(&mut app, action);
    }
    for action in
        coordinator.actions_for_backend_event(BackendEvent::Disconnected, SystemTime::UNIX_EPOCH)
    {
        let _ = yoctui_model::update(&mut app, action);
    }
    assert_eq!(
        app.background_jobs.jobs.back().unwrap().status,
        BackgroundJobStatus::Lost
    );
}
