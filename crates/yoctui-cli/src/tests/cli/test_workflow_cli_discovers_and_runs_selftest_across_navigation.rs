use super::*;

#[cfg(unix)]
#[tokio::test]
async fn test_workflow_cli_discovers_and_runs_selftest_across_navigation() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-testing-cli-runner-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    let bin = directory.join("bin");
    let build = directory.join("build");
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&build).unwrap();
    test_workflow_cli_executable(
        &bin.join("oe-selftest"),
        "#!/bin/sh\nprintf 'selftest output\\n'\nexit 0\n",
    );
    test_workflow_cli_executable(&bin.join("bitbake-selftest"), "#!/bin/sh\nexit 0\n");
    test_workflow_cli_executable(&bin.join("resulttool"), "#!/bin/sh\nexit 0\n");

    let mut app = App::new(100, 100_000);
    app.screen = Screen::Testing;
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    app.build.target = Some("core-image-minimal".into());
    let mut coordinator =
        TestCliCoordinator::new(build, vec![bin], yoctui_model::PtestCapability::Configured);
    assert!(
        coordinator
            .handle_effect(&mut app, Effect::InspectTestCapability)
            .await
    );
    assert!(matches!(
        app.test_capability.oe_selftest,
        yoctui_model::TestExecutableCapability::Available(_)
    ));
    assert!(matches!(
        app.result_tool_capability,
        yoctui_model::ResultToolCapability::Available(_)
    ));

    let _ = update(&mut app, Action::BeginSelectedTestLaunch);
    let _ = update(&mut app, Action::PreviewTestLaunch);
    let effect = update(&mut app, Action::ConfirmTestLaunch).unwrap();
    let Effect::StartTestSession { id, .. } = effect.clone() else {
        panic!("expected exact selftest effect");
    };
    assert!(coordinator.handle_effect(&mut app, effect).await);
    let _ = update(&mut app, Action::Open(Screen::Dashboard));
    for _ in 0..100 {
        coordinator.poll(&mut app).await;
        if app
            .test_session(id)
            .is_some_and(|session| session.outcome.is_some())
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    let session = app.test_session(id).unwrap();
    assert_eq!(
        session.outcome,
        Some(yoctui_model::TestSessionOutcome::Succeeded)
    );
    let job = app
        .background_jobs
        .get(session.background_job_id.unwrap())
        .unwrap();
    assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
    assert!(
        job.output
            .iter()
            .any(|entry| entry.message == "selftest output")
    );
    fs::remove_dir_all(directory).unwrap();
}
