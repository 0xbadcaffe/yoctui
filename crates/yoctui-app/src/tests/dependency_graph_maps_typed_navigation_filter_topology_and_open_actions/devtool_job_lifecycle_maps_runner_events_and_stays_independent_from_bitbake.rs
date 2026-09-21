use super::*;

#[test]
fn devtool_job_lifecycle_maps_runner_events_and_stays_independent_from_bitbake() {
    let now = SystemTime::UNIX_EPOCH;
    let mut devtool = DevtoolJobCoordinator::default();
    let operation = DevtoolOperation::Reset {
        recipe: "busybox".into(),
    };
    let actions = devtool.queue(operation.clone(), now).unwrap();
    let id = devtool.active_job_id().unwrap();
    assert_eq!(id, BackgroundJobId(1_u64 << 63));
    assert_eq!(devtool.active_operation(), Some(&operation));
    assert!(devtool.queue(operation, now).is_none());

    let mut build = BuildJobCoordinator::default();
    let build_actions = build
        .queue_build(
            &BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
                force: false,
            },
            now,
        )
        .unwrap();
    assert_eq!(build.active_job_id(), Some(BackgroundJobId(1)));
    assert_ne!(build.active_job_id(), devtool.active_job_id());

    let mut app = yoctui_model::App::new(10, 1_000);
    for action in actions.into_iter().chain(build_actions) {
        let _ = yoctui_model::update(&mut app, action);
    }
    for action in devtool.actions_for_event(DevtoolRunnerEvent::Started, now) {
        let _ = yoctui_model::update(&mut app, action);
    }
    for action in devtool.actions_for_event(
        DevtoolRunnerEvent::Output {
            stream: DevtoolOutputStream::Stderr,
            line: "progress".into(),
            truncated: true,
        },
        now,
    ) {
        let _ = yoctui_model::update(&mut app, action);
    }
    app.screen = Screen::Dashboard;
    for action in
        devtool.actions_for_event(DevtoolRunnerEvent::Completed { exit_code: Some(0) }, now)
    {
        let _ = yoctui_model::update(&mut app, action);
    }
    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
    assert_eq!(job.output[0].source, BackgroundJobOutputSource::Stderr);
    assert!(job.output[0].truncated);
    assert_eq!(app.screen, Screen::Dashboard);
    assert_eq!(build.active_job_id(), Some(BackgroundJobId(1)));
}
