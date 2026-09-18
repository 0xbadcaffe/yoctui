use super::*;

#[cfg(unix)]
#[tokio::test]
async fn wic_workspace_cli_discovers_runs_scans_and_persists_across_navigation() {
    let (directory, build_dir, mut app) = wic_workspace_fixture(
        "success",
        "printf '%s\\n' \"$@\"; printf 'wic' > \"$6/generated.wic\"; exit 0",
    )
    .await;
    let WicCapability::Available { kickstarts, .. } = &app.wic_capability else {
        panic!("available capability");
    };
    assert_eq!(
        kickstarts[0].identity.path.as_deref(),
        Some(directory.join("directdisk.wks").as_path())
    );
    assert_eq!(
        wic_create_dialog_action(false, Input::Char('Q')),
        None,
        "modal Q input must not leak to the Images workspace"
    );
    let (id, operation_request) = wic_workspace_start_effect(&mut app);
    let mut operation = None;
    begin_wic_job(
        &mut app,
        &mut operation,
        &WicDeviceInspector::default(),
        &build_dir,
        Duration::from_millis(100),
        id,
        operation_request,
    )
    .await;
    let duplicate = update(&mut app, Action::BeginSelectedWicCreate);
    assert!(duplicate.is_none());
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("already active"))
    );
    let _ = update(&mut app, Action::DismissNotification);
    app.screen = Screen::Logs;
    tokio::time::timeout(Duration::from_secs(2), async {
        while operation.is_some() {
            poll_wic_job(&mut app, &mut operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let session = app.wic_session(id).unwrap();
    let job = app.background_jobs.get(session.background_job_id).unwrap();
    assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
    assert_eq!(app.screen, Screen::Logs);
    assert!(
        job.output
            .iter()
            .any(|entry| entry.message == "core-image-minimal")
    );
    let outputs = app.wic_output_rows();
    assert_eq!(outputs.len(), 1);
    assert_eq!(
        outputs[0].identity.path,
        directory.join("deploy/generated.wic")
    );
    assert_eq!(app.wic_output_selection, Some(outputs[0].identity.clone()));
    fs::remove_dir_all(directory).unwrap();
}
