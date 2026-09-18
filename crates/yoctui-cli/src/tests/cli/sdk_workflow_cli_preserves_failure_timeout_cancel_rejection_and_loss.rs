use super::*;

#[cfg(unix)]
#[tokio::test]
async fn sdk_workflow_cli_preserves_failure_timeout_cancel_rejection_and_loss() {
    let (directory, _, artifact_adapter, tool_adapter, mut app) = sdk_workflow_fixture(
        "terminal",
        "printf 'publish failed\\n' >&2; exit 17",
        "exit 0",
        "trap '' TERM; printf 'ready\\n'; while :; do :; done",
    );
    let _ = update(
        &mut app,
        Action::SdkToolCapabilityLoaded(tool_adapter.capability()),
    );
    let effect = update(&mut app, Action::BeginSdkArtifactInventory).unwrap();
    let mut scan = None;
    begin_sdk_artifact_operation(&mut app, Some(&artifact_adapter), &mut scan, effect);
    sdk_workflow_poll_scan(&mut app, &mut scan).await;

    let destination = directory.join("failure-destination");
    fs::create_dir(&destination).unwrap();
    let _ = update(&mut app, Action::BeginSelectedSdkPublish);
    set_sdk_publish_destination(&mut app, &destination);
    let _ = update(&mut app, Action::PreviewSdkPublish);
    let Some(Effect::StartSdkSession { id, operation }) =
        update(&mut app, Action::ConfirmSdkPublish)
    else {
        panic!("expected failing SDK publication");
    };
    let mut owned = None;
    begin_sdk_job(
        &mut app,
        &mut owned,
        Some(&tool_adapter),
        Duration::from_millis(50),
        Duration::from_secs(2),
        id,
        operation,
    );
    let _ = sdk_workflow_poll_job(&mut app, &mut owned).await;
    let failed = app
        .background_jobs
        .get(app.sdk_session(id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(failed.status, yoctui_model::BackgroundJobStatus::Failed);
    assert_eq!(app.sdk_session(id).unwrap().exit_code, Some(17));
    assert!(
        failed
            .output
            .iter()
            .any(|entry| entry.message == "publish failed")
    );

    let _ = update(&mut app, Action::BeginSdkNative);
    set_sdk_native_draft(
        &mut app,
        yoctui_model::SdkNativeDraft {
            mode: yoctui_model::SdkNativeMode::RunNative,
            extracted_root: String::new(),
            recipe: "busybox".into(),
            tool: "sh".into(),
            arguments: Vec::new(),
        },
    );
    let _ = update(&mut app, Action::PreviewSdkNative);
    let Some(Effect::StartSdkSession {
        id: timeout_id,
        operation: timeout_operation,
    }) = update(&mut app, Action::ConfirmSdkNative)
    else {
        panic!("expected timed SDK native operation");
    };
    begin_sdk_job(
        &mut app,
        &mut owned,
        Some(&tool_adapter),
        Duration::from_millis(30),
        Duration::from_millis(30),
        timeout_id,
        timeout_operation,
    );
    let _ = sdk_workflow_poll_job(&mut app, &mut owned).await;
    let timed_out = app
        .background_jobs
        .get(app.sdk_session(timeout_id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(timed_out.status, yoctui_model::BackgroundJobStatus::Failed);
    assert!(
        timed_out
            .error
            .as_ref()
            .and_then(|error| error.detail.as_deref())
            .is_some_and(|detail| detail.contains("timed out"))
    );

    let _ = update(&mut app, Action::BeginSdkNative);
    set_sdk_native_draft(
        &mut app,
        yoctui_model::SdkNativeDraft {
            mode: yoctui_model::SdkNativeMode::RunNative,
            extracted_root: String::new(),
            recipe: "busybox".into(),
            tool: "sh".into(),
            arguments: Vec::new(),
        },
    );
    let _ = update(&mut app, Action::PreviewSdkNative);
    let Some(Effect::StartSdkSession {
        id: cancel_id,
        operation: cancel_operation,
    }) = update(&mut app, Action::ConfirmSdkNative)
    else {
        panic!("expected cancellable SDK native operation");
    };
    begin_sdk_job(
        &mut app,
        &mut owned,
        Some(&tool_adapter),
        Duration::from_millis(30),
        Duration::from_secs(2),
        cancel_id,
        cancel_operation,
    );
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let _ = poll_sdk_job(&mut app, &mut owned).await;
            let ready = app
                .sdk_session(cancel_id)
                .and_then(|session| app.background_jobs.get(session.background_job_id))
                .is_some_and(|job| job.output.iter().any(|entry| entry.message == "ready"));
            if ready {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let _ = update(&mut app, Action::BeginActiveSdkSessionCancellation);
    let Some(Effect::CancelSdkSession(effect_id)) =
        update(&mut app, Action::ConfirmSdkSessionCancellation)
    else {
        panic!("expected SDK cancellation");
    };
    begin_sdk_cancellation(&mut app, &mut owned, effect_id);
    let _ = sdk_workflow_poll_job(&mut app, &mut owned).await;
    let cancelled = app
        .background_jobs
        .get(app.sdk_session(cancel_id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(
        cancelled.status,
        yoctui_model::BackgroundJobStatus::Cancelled
    );
    assert!(
        cancelled
            .output
            .iter()
            .any(|entry| entry.message.contains("forced termination"))
    );

    let _ = update(&mut app, Action::BeginSdkNative);
    set_sdk_native_draft(
        &mut app,
        yoctui_model::SdkNativeDraft {
            mode: yoctui_model::SdkNativeMode::FindSysroot,
            extracted_root: String::new(),
            recipe: "busybox".into(),
            tool: String::new(),
            arguments: Vec::new(),
        },
    );
    let _ = update(&mut app, Action::PreviewSdkNative);
    let Some(Effect::StartSdkSession {
        id: rejected_id,
        operation: rejected_operation,
    }) = update(&mut app, Action::ConfirmSdkNative)
    else {
        panic!("expected rejected SDK operation");
    };
    let _ = update(
        &mut app,
        Action::SdkSessionStarting {
            id: rejected_id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::SdkSessionRunning { id: rejected_id });
    let _ = update(&mut app, Action::BeginActiveSdkSessionCancellation);
    let Some(Effect::CancelSdkSession(rejected_effect_id)) =
        update(&mut app, Action::ConfirmSdkSessionCancellation)
    else {
        panic!("expected rejected cancellation effect");
    };
    begin_sdk_cancellation(&mut app, &mut None, rejected_effect_id);
    let rejected = app
        .background_jobs
        .get(app.sdk_session(rejected_id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(rejected.status, yoctui_model::BackgroundJobStatus::Running);

    let lost_handle = tokio::spawn(async {
        std::future::pending::<(SdkToolJobRunner, Result<(), SdkToolAdapterError>)>().await
    });
    lost_handle.abort();
    let mut lost_operation = Some(SdkCliOperation {
        id: rejected_id,
        operation: rejected_operation,
        starting: Some(lost_handle),
        runner: None,
        timeout_wait: None,
        cancellation: None,
    });
    tokio::task::yield_now().await;
    poll_sdk_job(&mut app, &mut lost_operation).await;
    let lost = app
        .background_jobs
        .get(app.sdk_session(rejected_id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(lost.status, yoctui_model::BackgroundJobStatus::Lost);
    fs::remove_dir_all(directory).unwrap();
}
