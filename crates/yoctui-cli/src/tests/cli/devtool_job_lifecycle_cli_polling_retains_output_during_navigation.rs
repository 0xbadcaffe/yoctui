use super::*;

#[cfg(unix)]
#[tokio::test]
async fn devtool_job_lifecycle_cli_polling_retains_output_during_navigation() {
    use std::os::unix::fs::PermissionsExt;

    let script =
        std::env::temp_dir().join(format!("yoctui-devtool-lifecycle-{}", std::process::id()));
    fs::write(&script, "#!/bin/sh\nprintf 'background output\\n'\n").unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();

    let operation = DevtoolOperation::Reset {
        recipe: "busybox".into(),
    };
    let command = devtool_test_command(script.clone(), &operation);
    let mut coordinator = DevtoolJobCoordinator::default();
    let mut app = App::new(10, 1_000);
    for action in coordinator
        .queue(operation, SystemTime::UNIX_EPOCH)
        .unwrap()
    {
        let _ = update(&mut app, action);
    }
    let id = coordinator.active_job_id().unwrap();
    let mut started = DevtoolJobRunner::new(std::env::temp_dir());
    started.start(command).await.unwrap();
    let mut runner = Some(started);
    app.screen = Screen::Dashboard;
    let mut completed = None;
    tokio::time::timeout(Duration::from_secs(2), async {
        while runner.is_some() {
            let result = poll_devtool_job(&mut app, &mut coordinator, &mut runner).await;
            if result.is_some() {
                completed = result;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
    assert_eq!(job.output[0].message, "background output");
    assert_eq!(
        job.output[0].source,
        yoctui_model::BackgroundJobOutputSource::Stdout
    );
    assert_eq!(
        completed,
        Some(DevtoolOperation::Reset {
            recipe: "busybox".into()
        })
    );
    assert_eq!(app.screen, Screen::Dashboard);
    fs::remove_file(script).unwrap();
}
