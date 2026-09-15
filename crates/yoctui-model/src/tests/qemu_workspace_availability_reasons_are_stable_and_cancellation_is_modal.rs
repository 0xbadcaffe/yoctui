//! Regression tests grouped around qemu_workspace_availability_reasons_are_stable_and_cancellation_is_modal.
use super::*;

#[test]
fn qemu_workspace_availability_reasons_are_stable_and_cancellation_is_modal() {
    let mut app = qemu_model_app();
    let mut unsupported = qemu_model_app();
    if let ImageArtifactInventoryState::Available { inventory, .. } =
        &mut unsupported.image_artifacts
    {
        inventory.artifacts[0].kind = ImageArtifactKind::Manifest;
    }
    assert_eq!(
        unsupported.qemu_launch_unavailable_reason().as_deref(),
        Some("runqemu requires a root filesystem or Wic artifact.")
    );
    app.qemu_capability = QemuCapability::NotInspected;
    assert_eq!(
        app.qemu_launch_unavailable_reason().as_deref(),
        Some("runqemu capability has not been inspected.")
    );
    app.qemu_capability = QemuCapability::MissingTool;
    assert_eq!(
        app.qemu_launch_unavailable_reason().as_deref(),
        Some("runqemu is not available.")
    );
    app.qemu_capability = QemuCapability::MissingCompatibleImage;
    assert_eq!(
        app.qemu_launch_unavailable_reason().as_deref(),
        Some("No compatible deployed runqemu image is available.")
    );
    app.qemu_capability = QemuCapability::Failed {
        message: "inspection denied".into(),
    };
    assert_eq!(
        app.qemu_launch_unavailable_reason().as_deref(),
        Some("runqemu capability inspection failed: inspection denied")
    );
    app.qemu_capability = QemuCapability::Available {
        executable: "/opt/poky/scripts/runqemu".into(),
        compatible_images: Vec::new(),
    };
    assert_eq!(
        app.qemu_launch_unavailable_reason().as_deref(),
        Some("The selected artifact is not in the inspected runqemu capability.")
    );
    app.qemu_capability = QemuCapability::Available {
        executable: "/opt/poky/scripts/runqemu".into(),
        compatible_images: vec![qemu_model_artifact().identity],
    };
    let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
    let _ = update(&mut app, Action::PreviewQemuLaunch);
    let Some(Effect::StartQemuSession { id, .. }) = update(&mut app, Action::ConfirmQemuLaunch)
    else {
        panic!("start effect");
    };
    let _ = update(
        &mut app,
        Action::QemuSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::QemuSessionRunning { id });
    let _ = update(&mut app, Action::BeginActiveQemuSessionCancellation);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QemuCancellationConfirmation(candidate)) if *candidate == id
    ));
    let _ = update(&mut app, Action::CancelQemuSessionCancellation);
    assert!(app.active_dialog().is_none());
    assert_eq!(
        app.background_jobs
            .get(app.qemu_session(id).expect("session").background_job_id)
            .map(|job| job.status),
        Some(BackgroundJobStatus::Running)
    );
}

#[test]
fn wic_model_reducer_correlates_creation_inventory_and_lifecycle() {
    let mut app = App::new(20, 20_000);
    app.wic_capability = wic_model_capability();
    let draft = WicCreateDraft {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        kickstart: WicKickstartIdentity {
            name: "directdisk".into(),
            path: Some("/layers/meta/wic/directdisk.wks".into()),
        },
        output_directory: "/build/wic-output".into(),
        generate_bmap: true,
        compression: WicCompression::None,
    };
    let preview = draft.preview(&app.wic_capability).unwrap();
    let Some(Effect::StartWicSession { id, operation }) =
        update(&mut app, Action::StartConfirmedWicCreate(preview))
    else {
        panic!("Wic start effect");
    };
    assert!(matches!(operation, WicOperation::Create(_)));
    let background_job_id = app.wic_session(id).unwrap().background_job_id;
    assert_eq!(
        background_job_id,
        BackgroundJobId(WIC_BACKGROUND_JOB_NAMESPACE | id.0)
    );
    assert_ne!(
        background_job_id,
        qemu_background_job_id(QemuSessionId(id.0))
    );
    let _ = update(
        &mut app,
        Action::WicSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::WicSessionRunning { id });
    let _ = update(
        &mut app,
        Action::AppendWicSessionOutput {
            id,
            stream: WicOutputStream::Stdout,
            line: "creating".into(),
            truncated: false,
            timestamp: SystemTime::UNIX_EPOCH,
        },
    );
    let output = WicOutput {
        identity: WicOutputIdentity {
            path: "/build/wic-output/image.wic".into(),
            size_bytes: 1024,
            modified_unix_seconds: 1,
        },
        kind: WicOutputKind::Wic,
    };
    let _ = update(
        &mut app,
        Action::CompleteWicSession {
            id,
            exit_code: 0,
            outputs: vec![output.clone()],
            limitations: vec!["dynamic partition size".into()],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(
        app.background_jobs
            .get(background_job_id)
            .map(|job| job.status),
        Some(BackgroundJobStatus::Succeeded)
    );
    assert!(matches!(
        &app.wic_outputs,
        WicOutputInventoryState::Partial { outputs, .. } if outputs == &vec![output]
    ));

    let ignored = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::WicSessionRunning {
            id: WicSessionId(99_999),
        },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
}

#[test]
fn wic_device_write_requires_exact_phrase_and_cancellation_warning() {
    let mut app = App::new(20, 20_000);
    app.wic_capability = wic_model_capability();
    let image = WicOutputIdentity {
        path: "/build/wic-output/image.wic".into(),
        size_bytes: 1024,
        modified_unix_seconds: 1,
    };
    app.wic_outputs = WicOutputInventoryState::Available {
        request: WicOutputInventoryRequest {
            generation: 1,
            output_directory: "/build/wic-output".into(),
        },
        outputs: vec![WicOutput {
            identity: image.clone(),
            kind: WicOutputKind::Wic,
        }],
    };
    app.wic_output_selection = Some(image.clone());
    let Some(Effect::GetWicDevices(request)) =
        update(&mut app, Action::BeginSelectedWicDeviceWrite)
    else {
        panic!("Wic device discovery effect");
    };
    assert_eq!(request.image, image);
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicDevicePicker(dialog)) if dialog.request == request
    ));
    let ignored = app.background_jobs.ignored_transitions;
    let mut stale = request.clone();
    stale.generation += 1;
    let _ = update(
        &mut app,
        Action::WicDeviceInventoryLoaded {
            request: stale,
            devices: vec![wic_model_device("/dev/sdy", "8:239")],
            limitations: Vec::new(),
        },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
    let first = wic_model_device("/dev/sdy", "8:239");
    let selected = wic_model_device("/dev/sdz", "8:240");
    let _ = update(
        &mut app,
        Action::WicDeviceInventoryLoaded {
            request: request.clone(),
            devices: vec![first, selected.clone()],
            limitations: vec!["system device excluded".into()],
        },
    );
    let _ = update(&mut app, Action::SelectWicDevice { delta: 1 });
    assert_eq!(app.wic_device_selection, Some(selected.identity.clone()));
    let _ = update(&mut app, Action::CancelWicDevicePicker);
    let Some(Effect::GetWicDevices(refreshed_request)) =
        update(&mut app, Action::BeginSelectedWicDeviceWrite)
    else {
        panic!("refreshed Wic device discovery effect");
    };
    assert!(refreshed_request.generation > request.generation);
    let _ = update(
        &mut app,
        Action::WicDeviceInventoryLoaded {
            request: refreshed_request,
            devices: vec![selected.clone(), wic_model_device("/dev/sdy", "8:239")],
            limitations: Vec::new(),
        },
    );
    assert_eq!(app.wic_device_selection, Some(selected.identity.clone()));
    let _ = update(&mut app, Action::ConfirmWicDeviceSelection);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicWritePhrase(dialog)) if dialog.device == selected.identity
    ));
    for character in "WRITE /dev/sdy".chars() {
        let _ = update(&mut app, Action::AppendWicWritePhrase(character));
    }
    assert!(update(&mut app, Action::PreviewWicDeviceWrite).is_none());
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicWritePhrase(WicWritePhraseDialog {
            validation_error: Some(_),
            ..
        }))
    ));
    if let Some(Dialog::WicWritePhrase(dialog)) = app.active_dialog_mut() {
        dialog.input = "WRITE /dev/sdz".into();
    }
    let _ = update(&mut app, Action::PreviewWicDeviceWrite);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicWriteConfirmation(_))
    ));
    app.wic_capability = WicCapability::MissingTool;
    assert!(update(&mut app, Action::ConfirmWicDeviceWrite).is_none());
    assert!(app.active_wic_session().is_none());
    app.wic_capability = WicCapability::MissingKickstarts {
        executable: "/opt/poky/scripts/wic".into(),
    };
    let Some(Effect::StartWicSession { id, operation }) =
        update(&mut app, Action::ConfirmWicDeviceWrite)
    else {
        panic!("Wic write start effect");
    };
    assert!(matches!(operation, WicOperation::Write(_)));
    assert!(app.active_dialog().is_none());
    let _ = update(
        &mut app,
        Action::WicSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::WicSessionRunning { id });
    let _ = update(&mut app, Action::BeginActiveWicSessionCancellation);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicCancellationConfirmation {
            id: candidate,
            incomplete_device_warning: true,
        }) if *candidate == id
    ));
    assert!(
        update(
            &mut app,
            Action::ConfirmWicSessionCancellation {
                id,
                acknowledge_incomplete_device: false,
            },
        )
        .is_none()
    );
    assert_eq!(
        app.background_jobs
            .get(app.wic_session(id).unwrap().background_job_id)
            .map(|job| job.status),
        Some(BackgroundJobStatus::Running)
    );
    assert_eq!(
        update(
            &mut app,
            Action::ConfirmWicSessionCancellation {
                id,
                acknowledge_incomplete_device: true,
            },
        ),
        Some(Effect::CancelWicSession(id))
    );
    let _ = update(
        &mut app,
        Action::CancelWicSession {
            id,
            exit_code: Some(130),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let job = app
        .background_jobs
        .get(app.wic_session(id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Cancelled);
    assert!(
        job.error
            .as_ref()
            .and_then(|error| error.detail.as_deref())
            .is_some_and(|detail| detail.contains("incomplete"))
    );
}

#[test]
fn wic_device_write_discovers_only_exact_images_and_keeps_empty_failure_typed() {
    let mut app = qemu_model_app();
    app.wic_capability = wic_model_capability();
    let Some(Effect::GetWicDevices(request)) =
        update(&mut app, Action::BeginSelectedWicDeviceWrite)
    else {
        panic!("deployed Wic discovery");
    };
    assert!(request.image.path.ends_with("core-image-minimal.wic"));
    let _ = update(
        &mut app,
        Action::WicDeviceInventoryLoaded {
            request: request.clone(),
            devices: Vec::new(),
            limitations: Vec::new(),
        },
    );
    assert!(matches!(
        app.wic_devices,
        WicDeviceInventoryState::Available {
            ref devices,
            ..
        } if devices.is_empty()
    ));
    assert!(app.wic_device_selection.is_none());
    assert!(update(&mut app, Action::ConfirmWicDeviceSelection).is_none());
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicDevicePicker(_))
    ));
    let _ = update(&mut app, Action::CancelWicDevicePicker);
    let Some(Effect::GetWicDevices(second_request)) =
        update(&mut app, Action::BeginSelectedWicDeviceWrite)
    else {
        panic!("second discovery");
    };
    assert!(second_request.generation > request.generation);
    let _ = update(
        &mut app,
        Action::WicDeviceInventoryFailed {
            request: second_request.clone(),
            message: "root identity ambiguous".into(),
        },
    );
    assert!(matches!(
        &app.wic_devices,
        WicDeviceInventoryState::Failed { request, message }
            if request == &second_request && message == "root identity ambiguous"
    ));

    let mut compressed = qemu_model_app();
    compressed.wic_capability = wic_model_capability();
    if let ImageArtifactInventoryState::Available { inventory, .. } =
        &mut compressed.image_artifacts
    {
        inventory.artifacts[0].identity.path =
            "/build/tmp/deploy/images/qemux86-64/core-image-minimal.wic.gz".into();
        compressed.image_artifact_selection = Some(inventory.artifacts[0].identity.clone());
    }
    assert!(update(&mut compressed, Action::BeginSelectedWicDeviceWrite).is_none());

    let mut direct = qemu_model_app();
    direct.wic_capability = wic_model_capability();
    if let ImageArtifactInventoryState::Available { inventory, .. } = &mut direct.image_artifacts {
        inventory.artifacts[0].identity.path =
            "/build/tmp/deploy/images/qemux86-64/core-image-minimal.direct".into();
        direct.image_artifact_selection = Some(inventory.artifacts[0].identity.clone());
    }
    assert!(matches!(
        update(&mut direct, Action::BeginSelectedWicDeviceWrite),
        Some(Effect::GetWicDevices(WicDeviceInventoryRequest {
            image: WicOutputIdentity { ref path, .. },
            ..
        })) if path.ends_with("core-image-minimal.direct")
    ));
}

#[test]
fn wic_workspace_dialog_is_bounded_modal_and_stale_safe() {
    let mut app = qemu_model_app();
    app.wic_capability = wic_model_capability();
    if let WicCapability::Available { kickstarts, .. } = &mut app.wic_capability {
        kickstarts.push(WicKickstart {
            identity: WicKickstartIdentity {
                name: "configured".into(),
                path: Some("/layers/custom/configured.wks".into()),
            },
            source: "part /boot --source=bootimg-partition".into(),
            partitions: Vec::new(),
            limitations: Vec::new(),
        });
    }
    app.workspace
        .variables
        .insert("WKS_FILE".into(), "/layers/custom/configured.wks".into());
    let _ = update(&mut app, Action::BeginSelectedWicCreate);
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicCreateTomlEditor { editor, .. })
            if !editor.editing
                && editor.text.contains("kickstart = \"configured\"")
                && editor.selected_text() == Some("/build/tmp/deploy/images/qemux86-64")
    ));
    if let Some(Dialog::WicCreateTomlEditor { editor, .. }) = app.active_dialog_mut() {
        editor.text = "machine = \"qemux86-64\"\nimage = \"core-image-minimal\"\nkickstart = \"configured\"\noutput_directory = \"relative/output\"\ngenerate_bmap = true\ncompression = \"none\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::PreviewWicCreate);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicCreateTomlEditor {
            validation_error: Some(_),
            ..
        })
    ));
    let _ = update(&mut app, Action::CancelWicCreate);
    assert!(app.active_dialog().is_none());
    assert_eq!(app.focus, FocusTarget::Navigator);

    let _ = update(&mut app, Action::BeginSelectedWicCreate);
    let _ = update(&mut app, Action::PreviewWicCreate);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicCreateConfirmation(_))
    ));
    app.wic_capability = WicCapability::MissingTool;
    assert!(update(&mut app, Action::ConfirmWicCreate).is_none());
    assert!(app.active_wic_session().is_none());
}

#[test]
fn wic_workspace_output_selection_and_creation_cancellation_are_typed() {
    let mut app = qemu_model_app();
    app.wic_capability = wic_model_capability();
    let _ = update(&mut app, Action::BeginSelectedWicCreate);
    let _ = update(&mut app, Action::PreviewWicCreate);
    let Some(Effect::StartWicSession { id, .. }) = update(&mut app, Action::ConfirmWicCreate)
    else {
        panic!("Wic start");
    };
    let _ = update(
        &mut app,
        Action::WicSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::WicSessionRunning { id });
    let _ = update(&mut app, Action::BeginActiveImageRuntimeCancellation);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicCancellationConfirmation {
            id: candidate,
            incomplete_device_warning: false,
        }) if *candidate == id
    ));
    let _ = update(&mut app, Action::CancelWicSessionCancellation);
    assert!(app.active_dialog().is_none());

    let output = WicOutput {
        identity: WicOutputIdentity {
            path: "/build/out/image.wic".into(),
            size_bytes: 1,
            modified_unix_seconds: 1,
        },
        kind: WicOutputKind::Wic,
    };
    app.wic_outputs = WicOutputInventoryState::Available {
        request: WicOutputInventoryRequest {
            generation: 1,
            output_directory: "/build/out".into(),
        },
        outputs: vec![output.clone()],
    };
    let _ = update(&mut app, Action::SelectWicOutput { delta: 1 });
    assert_eq!(app.wic_output_selection, Some(output.identity.clone()));
    assert_eq!(
        update(&mut app, Action::OpenSelectedWicOutput),
        Some(Effect::OpenInEditor(output.identity.path))
    );
}

#[test]
fn sdk_workflow_navigates_and_previews_exact_managed_builds() {
    let mut app = sdk_workflow_app();
    let sdk_index = NAVIGATOR_SCREENS
        .iter()
        .position(|screen| *screen == Screen::Sdk)
        .unwrap();
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = sdk_index;
    app.sdk_tool_capability = SdkToolCapability::NotInspected;
    assert_eq!(
        update(&mut app, Action::ActivateNavigator),
        Some(Effect::InspectSdkTools)
    );
    assert_eq!(app.screen, Screen::Sdk);
    let _ = update(
        &mut app,
        Action::BeginSdkBuild(SdkBuildAction::Populate(SdkKind::Extensible)),
    );
    let Some(Dialog::SdkBuildConfirmation(preview)) = app.active_dialog() else {
        panic!("SDK build confirmation");
    };
    assert_eq!(preview.machine, "qemux86-64");
    assert_eq!(preview.distro, "poky");
    assert_eq!(preview.request.task.as_deref(), Some("populate_sdk_ext"));
    assert_eq!(
        update(&mut app, Action::ConfirmSdkBuild),
        Some(Effect::Start(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: Some("populate_sdk_ext".into()),
            force: false,
        }))
    );
    assert!(app.active_dialog().is_none());
}

#[test]
fn sdk_workflow_inventory_publication_native_and_lifecycle_are_correlated() {
    let mut app = sdk_workflow_app();
    let Some(Effect::GetSdkArtifacts(request)) =
        update(&mut app, Action::BeginSdkArtifactInventory)
    else {
        panic!("SDK inventory effect");
    };
    assert_eq!(
        update(&mut app, Action::BeginActiveSdkSessionCancellation),
        Some(Effect::CancelSdkArtifactOperation)
    );
    let stale = SdkArtifactInventoryRequest {
        generation: request.generation + 1,
        ..request.clone()
    };
    let ignored = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::SdkArtifactInventoryFailed {
            request: stale,
            message: "stale".into(),
        },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
    let artifact = SdkArtifact {
        identity: SdkArtifactIdentity {
            path: "/build/deploy/sdk/poky.sh".into(),
            size_bytes: 42,
            modified_unix_seconds: 7,
        },
        kind: SdkArtifactKind::Installer,
        sdk_kind: Some(SdkKind::Standard),
        machine: Some("qemux86-64".into()),
        host_tuple: Some("x86_64-pokysdk-linux".into()),
        target_tuple: Some("x86_64-poky-linux".into()),
        checksums: Vec::new(),
        manifests: Vec::new(),
        published: None,
    };
    let _ = update(
        &mut app,
        Action::SdkArtifactInventoryLoaded {
            request,
            artifacts: vec![artifact.clone()],
            limitations: vec!["one unrelated record skipped".into()],
        },
    );
    assert_eq!(app.sdk_artifact_selection, Some(artifact.identity.clone()));
    assert!(matches!(
        app.sdk_artifacts,
        SdkArtifactInventoryState::Partial { .. }
    ));
    assert_eq!(
        update(&mut app, Action::OpenSelectedSdkArtifact),
        Some(Effect::OpenInEditor(artifact.identity.path.clone()))
    );

    let _ = update(&mut app, Action::BeginSelectedSdkPublish);
    if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut() {
        editor.text = "destination = \"/srv/sdk\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::PreviewSdkPublish);
    let Some(Effect::StartSdkSession { id, operation }) =
        update(&mut app, Action::ConfirmSdkPublish)
    else {
        panic!("SDK publication effect");
    };
    assert!(matches!(
        operation,
        SdkOperation::Publish(SdkPublishRequest { destination, .. })
            if destination == Path::new("/srv/sdk")
    ));
    let _ = update(
        &mut app,
        Action::SdkSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::SdkSessionRunning { id });
    let _ = update(
        &mut app,
        Action::AppendSdkSessionOutput {
            id,
            stream: SdkOutputStream::Stderr,
            line: "publication warning".into(),
            truncated: true,
            timestamp: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(
        &mut app,
        Action::CompleteSdkSession {
            id,
            exit_code: 0,
            artifacts: vec!["/srv/sdk/poky.sh".into()],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let session = app.sdk_session(id).unwrap();
    let job = app.background_jobs.get(session.background_job_id).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Succeeded);
    assert!(job.output[0].truncated);

    let _ = update(&mut app, Action::BeginSdkNative);
    let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut() else {
        panic!("SDK native dialog");
    };
    editor.text = "mode = \"run-native\"\nworkspace = \"/opt/sdk\"\nrecipe = \"cmake-native\"\ntool = \"cmake\"\narguments = \"--version\"\n".into();
    editor.cursor = editor.text.len();
    let _ = update(&mut app, Action::PreviewSdkNative);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::SdkNativeConfirmation(_))
    ));
    let _ = update(&mut app, Action::CancelSdkNativePreview);

    let _ = update(&mut app, Action::BeginSdkNative);
    if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut() {
        editor.text = "mode = \"run-native\"\nworkspace = \"/opt/sdk\"\nrecipe = \"cmake-native\"\ntool = \"cmake\"\narguments = \"--version\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::PreviewSdkNative);
    let Some(Effect::StartSdkSession { id: native_id, .. }) =
        update(&mut app, Action::ConfirmSdkNative)
    else {
        panic!("SDK native effect");
    };
    let _ = update(
        &mut app,
        Action::SdkSessionStarting {
            id: native_id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::SdkSessionRunning { id: native_id });
    let _ = update(&mut app, Action::BeginActiveSdkSessionCancellation);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::SdkCancellationConfirmation(id)) if *id == native_id
    ));
    assert_eq!(
        update(&mut app, Action::ConfirmSdkSessionCancellation),
        Some(Effect::CancelSdkSession(native_id))
    );
    let _ = update(
        &mut app,
        Action::CancelSdkSession {
            id: native_id,
            exit_code: Some(130),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(
        app.background_jobs
            .get(app.sdk_session(native_id).unwrap().background_job_id)
            .unwrap()
            .status,
        BackgroundJobStatus::Cancelled
    );
}

#[test]
fn test_workflow_model_navigates_previews_and_runs_bounded_selftests() {
    let mut app = test_workflow_app();
    let testing_index = NAVIGATOR_SCREENS
        .iter()
        .position(|screen| *screen == Screen::Testing)
        .unwrap();
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = testing_index;
    app.test_capability = TestCapability::default();
    assert_eq!(
        update(&mut app, Action::ActivateNavigator),
        Some(Effect::InspectTestCapability)
    );
    assert_eq!(app.screen, Screen::Testing);
    let capability = test_workflow_app().test_capability;
    let _ = update(&mut app, Action::TestCapabilityLoaded(capability));
    let _ = update(&mut app, Action::BeginSelectedTestLaunch);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestLaunchTomlEditor { editor, .. })
            if editor.selected_text() == Some("all")
    ));
    if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog_mut() {
        editor.text = "family = \"OE selftest\"\nmachine = \"qemux86-64\"\ndistro = \"poky\"\nimage = \"core-image-minimal\"\nscope = \"selected\"\nselector = \"tinfoil.TinfoilTests.test_getvar\"\nparallelism = 8\nverbose = false\nskip_network = false\n".into();
    }
    let _ = update(&mut app, Action::PreviewTestLaunch);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestLaunchConfirmation(
            TestLaunchPreview::Selftest(request)
        )) if request.parallelism == 8
            && request.selector.as_deref() == Some("tinfoil.TinfoilTests.test_getvar")
    ));
    let Some(Effect::StartTestSession { id, operation }) =
        update(&mut app, Action::ConfirmTestLaunch)
    else {
        panic!("selftest effect");
    };
    assert!(matches!(
        operation,
        TestOperation::Selftest(TestSelftestRequest {
            family: TestFamily::OeSelftest,
            ..
        })
    ));
    let job_id = app.test_session(id).unwrap().background_job_id.unwrap();
    assert_eq!(
        app.background_jobs.get(job_id).unwrap().kind,
        BackgroundJobKind::Test
    );
    let _ = update(
        &mut app,
        Action::TestSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::TestSessionRunning { id });
    let _ = update(
        &mut app,
        Action::AppendTestSessionOutput {
            id,
            stream: TestOutputStream::Stderr,
            line: "one warning".into(),
            truncated: true,
            timestamp: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::BeginActiveTestSessionCancellation);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestCancellationConfirmation(candidate)) if *candidate == id
    ));
    assert_eq!(
        update(&mut app, Action::ConfirmTestSessionCancellation),
        Some(Effect::CancelTestSession(id))
    );
    let _ = update(
        &mut app,
        Action::RejectTestSessionCancellation {
            id,
            message: "still stopping".into(),
        },
    );
    assert_eq!(
        app.background_jobs.get(job_id).unwrap().status,
        BackgroundJobStatus::Running
    );
    let _ = update(&mut app, Action::BeginActiveTestSessionCancellation);
    let _ = update(&mut app, Action::ConfirmTestSessionCancellation);
    let _ = update(
        &mut app,
        Action::CancelTestSession {
            id,
            exit_code: Some(130),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let job = app.background_jobs.get(job_id).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Cancelled);
    assert!(job.output[0].truncated);
}

#[test]
fn test_workflow_launch_editor_rejects_changed_authoritative_context() {
    let mut app = test_workflow_app();
    let _ = update(&mut app, Action::BeginSelectedTestLaunch);
    let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog_mut() else {
        panic!("test launch TOML editor");
    };
    editor.text = editor.text.replace(
        "machine = \"qemux86-64\"",
        "machine = \"untrusted-machine\"",
    );

    let _ = update(&mut app, Action::PreviewTestLaunch);

    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestLaunchTomlEditor {
            validation_error: Some(message),
            ..
        }) if message.contains("must match the current Testing context")
    ));
}
