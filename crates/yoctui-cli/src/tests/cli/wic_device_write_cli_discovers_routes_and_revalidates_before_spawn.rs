use super::*;

#[cfg(unix)]
#[tokio::test]
async fn wic_device_write_cli_discovers_routes_and_revalidates_before_spawn() {
    let (directory, build_dir, mut app) =
        wic_workspace_fixture("device-write-cli", "printf 'write-started\\n'; exit 0").await;
    let image_path = directory.join("deploy/device-image.wic");
    fs::write(&image_path, b"image").unwrap();
    let image_path = fs::canonicalize(image_path).unwrap();
    let image = yoctui_model::WicOutput {
        identity: yoctui_model::WicOutputIdentity {
            path: image_path,
            size_bytes: 5,
            modified_unix_seconds: 7,
        },
        kind: yoctui_model::WicOutputKind::Wic,
    };
    app.wic_output_selection = Some(image.identity.clone());
    app.wic_outputs = yoctui_model::WicOutputInventoryState::Available {
        request: yoctui_model::WicOutputInventoryRequest {
            generation: 1,
            output_directory: directory.join("deploy"),
        },
        outputs: vec![image],
    };

    let lsblk = directory.join("lsblk");
    let inventory = r#"{"blockdevices":[{"path":"/dev/sda","type":"disk","maj:min":"8:0","size":8192,"model":"root","serial":"root-serial","tran":null,"rm":false,"ro":false,"mountpoints":[],"children":[{"path":"/dev/sda1","type":"part","maj:min":"8:1","size":4096,"model":null,"serial":null,"tran":null,"rm":false,"ro":false,"mountpoints":["/"],"children":[]}]},{"path":"/dev/sdz","type":"disk","maj:min":"8:240","size":16384,"model":"Protected USB","serial":"SERIAL-123","tran":"usb","rm":true,"ro":false,"mountpoints":[],"children":[]}]}"#;
    write_test_executable(&lsblk, &format!("#!/bin/sh\nprintf '%s' '{}'\n", inventory));
    let inspector = WicDeviceInspector::with_program(fs::canonicalize(&lsblk).unwrap())
        .without_device_node_validation_for_tests();

    let Some(effect) = update(&mut app, Action::BeginSelectedWicDeviceWrite) else {
        panic!("expected device discovery effect");
    };
    let Effect::GetWicDevices(request) = &effect else {
        panic!("expected device discovery effect");
    };
    let request = request.clone();
    let mut discovery = None;
    begin_wic_device_operation(&inspector, &mut discovery, effect);
    tokio::time::timeout(Duration::from_secs(2), async {
        while discovery.is_some() {
            poll_wic_device_operation(&mut app, &mut discovery).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let yoctui_model::WicDeviceInventoryState::Partial {
        request: discovered,
        devices,
        limitations,
    } = &app.wic_devices
    else {
        panic!("fake discovery must retain a partial inventory");
    };
    assert_eq!(discovered, &request);
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].identity.path, Path::new("/dev/sdz"));
    assert!(limitations.iter().any(
        |limitation| limitation.contains("/dev/sda") && limitation.contains("root filesystem")
    ));

    let mut failed_app = app.clone();
    let _ = update(&mut failed_app, Action::CancelWicDevicePicker);
    let Some(failed_effect) = update(&mut failed_app, Action::BeginSelectedWicDeviceWrite) else {
        panic!("expected replacement discovery");
    };
    let Effect::GetWicDevices(failed_request) = &failed_effect else {
        panic!("expected replacement discovery");
    };
    let failed_request = failed_request.clone();
    let missing = WicDeviceInspector::with_program(directory.join("missing-lsblk"));
    begin_wic_device_operation(&missing, &mut discovery, failed_effect);
    tokio::time::timeout(Duration::from_secs(2), async {
        while discovery.is_some() {
            poll_wic_device_operation(&mut failed_app, &mut discovery).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        &failed_app.wic_devices,
        yoctui_model::WicDeviceInventoryState::Failed { request, message }
            if request == &failed_request && message.contains("unavailable")
    ));

    assert_eq!(
        wic_device_picker_action(Input::Char('D')),
        None,
        "modal input must not leak to the Images workspace"
    );
    let picker_action = wic_device_picker_action(Input::Enter).unwrap();
    let _ = update(&mut app, picker_action);
    for character in "WRITE /dev/sdz".chars() {
        let action = wic_write_phrase_action(Input::Char(character)).unwrap();
        let _ = update(&mut app, action);
    }
    let preview_action = wic_write_phrase_action(Input::Enter).unwrap();
    let _ = update(&mut app, preview_action);
    let confirm_action = wic_write_confirmation_action(Input::Enter).unwrap();
    let Some(Effect::StartWicSession { id, operation }) = update(&mut app, confirm_action) else {
        panic!("expected confirmed write effect");
    };
    assert!(matches!(&operation, WicOperation::Write(request)
            if request.image.path == directory.join("deploy/device-image.wic")
                && request.image.size_bytes == 5
                && request.device.path == Path::new("/dev/sdz")));
    let cancellation = wic_cancellation_confirmation_action(id, true, Input::Enter);
    assert_eq!(
        cancellation,
        Some(Action::ConfirmWicSessionCancellation {
            id,
            acknowledge_incomplete_device: true,
        })
    );

    let mut running = None;
    begin_wic_job(
        &mut app,
        &mut running,
        &inspector,
        &build_dir,
        Duration::from_millis(100),
        id,
        operation,
    )
    .await;
    assert!(running.is_some());
    tokio::time::timeout(Duration::from_secs(2), async {
        while running.is_some() {
            poll_wic_job(&mut app, &mut running).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let session = app.wic_session(id).unwrap();
    let job = app.background_jobs.get(session.background_job_id).unwrap();
    assert!(
        job.output
            .iter()
            .any(|entry| entry.message == "write-started")
    );
    assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
    assert_eq!(session.exit_code, Some(0));

    let wic = match &app.wic_capability {
        WicCapability::Available { executable, .. } => executable.clone(),
        capability => panic!("unexpected Wic capability: {capability:?}"),
    };
    write_test_executable(
        &wic,
        "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
    );
    let (cancel_id, cancel_request) = wic_device_write_start_effect(&mut app, &inspector).await;
    let mut cancel_operation = None;
    begin_wic_job(
        &mut app,
        &mut cancel_operation,
        &inspector,
        &build_dir,
        Duration::from_secs(2),
        cancel_id,
        cancel_request,
    )
    .await;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            poll_wic_job(&mut app, &mut cancel_operation).await;
            let ready = app
                .wic_session(cancel_id)
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
    let _ = update(&mut app, Action::BeginActiveWicSessionCancellation);
    let Some(Dialog::WicCancellationConfirmation {
        id: dialog_id,
        incomplete_device_warning: true,
    }) = app.active_dialog().cloned()
    else {
        panic!("write cancellation must retain its incomplete-device warning");
    };
    let effect = wic_cancellation_confirmation_action(dialog_id, true, Input::Enter)
        .and_then(|action| update(&mut app, action));
    let Some(Effect::CancelWicSession(effect_id)) = effect else {
        panic!("expected write cancellation effect");
    };
    begin_wic_cancellation(&mut app, &mut cancel_operation, effect_id);
    tokio::time::timeout(Duration::from_secs(4), async {
        while cancel_operation.is_some() {
            poll_wic_job(&mut app, &mut cancel_operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let cancelled = app.wic_session(cancel_id).unwrap();
    assert_eq!(
        app.background_jobs
            .get(cancelled.background_job_id)
            .unwrap()
            .status,
        yoctui_model::BackgroundJobStatus::Cancelled
    );

    write_test_executable(&wic, "#!/bin/sh\nprintf 'write-error\\n' >&2\nexit 9\n");
    let (failed_id, failed_request) = wic_device_write_start_effect(&mut app, &inspector).await;
    let mut failed_operation = None;
    begin_wic_job(
        &mut app,
        &mut failed_operation,
        &inspector,
        &build_dir,
        Duration::from_millis(100),
        failed_id,
        failed_request,
    )
    .await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while failed_operation.is_some() {
            poll_wic_job(&mut app, &mut failed_operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let failed = app.wic_session(failed_id).unwrap();
    let failed_job = app.background_jobs.get(failed.background_job_id).unwrap();
    assert_eq!(failed_job.status, yoctui_model::BackgroundJobStatus::Failed);
    assert_eq!(failed.exit_code, Some(9));
    assert!(
        failed_job
            .output
            .iter()
            .any(|entry| entry.message == "write-error")
    );

    write_test_executable(&wic, "#!/bin/sh\nsleep 30\n");
    let (lost_id, lost_request) = wic_device_write_start_effect(&mut app, &inspector).await;
    let mut lost_operation = None;
    begin_wic_job(
        &mut app,
        &mut lost_operation,
        &inspector,
        &build_dir,
        Duration::from_millis(100),
        lost_id,
        lost_request,
    )
    .await;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            poll_wic_job(&mut app, &mut lost_operation).await;
            let running = app
                .wic_session(lost_id)
                .and_then(|session| app.background_jobs.get(session.background_job_id))
                .is_some_and(|job| job.status == yoctui_model::BackgroundJobStatus::Running);
            if running {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let lost_handle = tokio::spawn(async {
        std::future::pending::<(WicJobRunner, Result<bool, WicAdapterError>)>().await
    });
    lost_handle.abort();
    lost_operation.as_mut().unwrap().cancellation = Some(lost_handle);
    tokio::task::yield_now().await;
    poll_wic_job(&mut app, &mut lost_operation).await;
    let lost = app.wic_session(lost_id).unwrap();
    assert_eq!(
        app.background_jobs
            .get(lost.background_job_id)
            .unwrap()
            .status,
        yoctui_model::BackgroundJobStatus::Lost
    );

    let (stale_id, stale_request) = wic_device_write_start_effect(&mut app, &inspector).await;
    let changed_inventory = inventory.replace("SERIAL-123", "SERIAL-CHANGED");
    write_test_executable(
        &lsblk,
        &format!("#!/bin/sh\nprintf '%s' '{}'\n", changed_inventory),
    );
    let mut stale_operation = None;
    begin_wic_job(
        &mut app,
        &mut stale_operation,
        &inspector,
        &build_dir,
        Duration::from_millis(100),
        stale_id,
        stale_request,
    )
    .await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while stale_operation.is_some() {
            poll_wic_job(&mut app, &mut stale_operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let stale = app.wic_session(stale_id).unwrap();
    assert!(
        stale
            .error_detail
            .as_deref()
            .is_some_and(|message| message.contains("identity changed"))
    );

    let (reject_id, _) = wic_device_write_start_effect(&mut app, &inspector).await;
    let _ = update(
        &mut app,
        Action::WicSessionStarting {
            id: reject_id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::WicSessionRunning { id: reject_id });
    let _ = update(&mut app, Action::BeginActiveWicSessionCancellation);
    let effect = wic_cancellation_confirmation_action(reject_id, true, Input::Enter)
        .and_then(|action| update(&mut app, action));
    let Some(Effect::CancelWicSession(effect_id)) = effect else {
        panic!("expected unowned write cancellation effect");
    };
    let mut unowned = None;
    begin_wic_cancellation(&mut app, &mut unowned, effect_id);
    let rejected = app.wic_session(reject_id).unwrap();
    assert_eq!(
        app.background_jobs
            .get(rejected.background_job_id)
            .unwrap()
            .status,
        yoctui_model::BackgroundJobStatus::Running
    );
    fs::remove_dir_all(directory).unwrap();
}
