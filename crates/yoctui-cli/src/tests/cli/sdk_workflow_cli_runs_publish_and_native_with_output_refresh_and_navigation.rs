use super::*;

#[cfg(unix)]
#[tokio::test]
async fn sdk_workflow_cli_runs_publish_and_native_with_output_refresh_and_navigation() {
    let (directory, build, artifact_adapter, tool_adapter, mut app) = sdk_workflow_fixture(
        "success",
        "printf 'publish:%s\\n' \"$1\"; touch \"$2/published\"; exit 0",
        "printf 'sysroot:%s\\n' \"$1\"; exit 0",
        "printf 'child:%s args:%s\\n' \"$YOCTUI_SDK_CHILD_ONLY\" \"$*\"; exit 0",
    );
    let _ = update(
        &mut app,
        Action::SdkToolCapabilityLoaded(tool_adapter.capability()),
    );
    let effect = update(&mut app, Action::BeginSdkArtifactInventory).unwrap();
    let mut scan = None;
    begin_sdk_artifact_operation(&mut app, Some(&artifact_adapter), &mut scan, effect);
    sdk_workflow_poll_scan(&mut app, &mut scan).await;
    let open =
        sdk_workspace_action(false, Input::Char('o')).and_then(|action| update(&mut app, action));
    assert!(
        matches!(open, Some(Effect::OpenInEditor(path)) if path.to_string_lossy().ends_with(".sh"))
    );

    let destination = directory.join("published");
    fs::create_dir(&destination).unwrap();
    let _ = update(&mut app, Action::BeginSelectedSdkPublish);
    set_sdk_publish_destination(&mut app, &destination);
    let _ = update(&mut app, Action::PreviewSdkPublish);
    let Some(Effect::StartSdkSession { id, operation }) =
        update(&mut app, Action::ConfirmSdkPublish)
    else {
        panic!("expected SDK publication");
    };
    let mut owned = None;
    begin_sdk_job(
        &mut app,
        &mut owned,
        Some(&tool_adapter),
        Duration::from_millis(100),
        Duration::from_secs(2),
        id,
        operation,
    );
    app.screen = Screen::Logs;
    let completed = sdk_workflow_poll_job(&mut app, &mut owned).await;
    assert!(matches!(completed, Some(SdkOperation::Publish(_))));
    let job = app
        .background_jobs
        .get(app.sdk_session(id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
    assert!(
        job.output
            .iter()
            .any(|entry| entry.message.starts_with("publish:"))
    );
    assert_eq!(app.screen, Screen::Logs);
    let generation = app.sdk_artifact_generation;
    let refresh = update(&mut app, Action::RefreshSdkArtifactInventory).unwrap();
    begin_sdk_artifact_operation(&mut app, Some(&artifact_adapter), &mut scan, refresh);
    sdk_workflow_poll_scan(&mut app, &mut scan).await;
    assert!(app.sdk_artifact_generation > generation);

    let extracted = directory.join("extracted");
    fs::create_dir(&extracted).unwrap();
    fs::write(
        extracted.join("environment-setup-x86_64-pokysdk-linux"),
        "export YOCTUI_SDK_CHILD_ONLY=visible\n",
    )
    .unwrap();
    let executable = match &app.sdk_tool_capability {
        SdkToolCapability::Available {
            run_native: Some(path),
            ..
        } => path.clone(),
        capability => panic!("unexpected SDK capability: {capability:?}"),
    };
    let _ = update(&mut app, Action::BeginSdkNative);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::SdkNativeTomlEditor(_))
    ));
    set_sdk_native_draft(
        &mut app,
        yoctui_model::SdkNativeDraft {
            mode: yoctui_model::SdkNativeMode::RunNative,
            extracted_root: extracted.display().to_string(),
            recipe: "busybox".into(),
            tool: "sh".into(),
            arguments: vec!["--version".into()],
        },
    );
    let _ = update(&mut app, Action::PreviewSdkNative);
    let Some(Effect::StartSdkSession {
        id: native_id,
        operation: native_operation,
    }) = update(&mut app, Action::ConfirmSdkNative)
    else {
        panic!("expected SDK native tool operation");
    };
    assert!(matches!(
        &native_operation,
        SdkOperation::Native(request)
            if request.executable == executable && request.extracted_root.as_ref() == Some(&extracted)
    ));
    begin_sdk_job(
        &mut app,
        &mut owned,
        Some(&tool_adapter),
        Duration::from_millis(100),
        Duration::from_secs(2),
        native_id,
        native_operation,
    );
    let _ = sdk_workflow_poll_job(&mut app, &mut owned).await;
    let native_job = app
        .background_jobs
        .get(app.sdk_session(native_id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(
        native_job.status,
        yoctui_model::BackgroundJobStatus::Succeeded
    );
    assert!(
        native_job
            .output
            .iter()
            .any(|entry| entry.message == "child:visible args:busybox sh --version")
    );
    assert!(std::env::var_os("YOCTUI_SDK_CHILD_ONLY").is_none());

    let populate = BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: Some("populate_sdk".into()),
        force: false,
    };
    let mut pending = Some(populate);
    let effect = sdk_refresh_after_build_event(
        &mut app,
        &mut pending,
        &BackendEvent::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(matches!(effect, Some(Effect::GetSdkArtifacts(_))));
    assert!(pending.is_none());
    let mut test_pending = Some(BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: Some("testsdk".into()),
        force: false,
    });
    assert!(
        sdk_refresh_after_build_event(
            &mut app,
            &mut test_pending,
            &BackendEvent::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
        )
        .is_none()
    );
    assert!(test_pending.is_none());
    assert!(build.is_dir());
    fs::remove_dir_all(directory).unwrap();
}
