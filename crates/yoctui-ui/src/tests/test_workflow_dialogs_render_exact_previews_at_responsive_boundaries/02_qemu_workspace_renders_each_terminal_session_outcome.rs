#[test]
fn qemu_workspace_renders_each_terminal_session_outcome() {
    let (mut succeeded, succeeded_id) = qemu_running_workspace_app();
    let _ = yoctui_model::update(
        &mut succeeded,
        yoctui_model::Action::CompleteQemuSession {
            id: succeeded_id,
            exit_code: 0,
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(rendered_text(&succeeded, 160, 40).contains("Status: succeeded"));

    let (mut failed, failed_id) = qemu_running_workspace_app();
    let _ = yoctui_model::update(
        &mut failed,
        yoctui_model::Action::FailQemuSession {
            id: failed_id,
            message: "failed display".into(),
            exit_code: Some(1),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(rendered_text(&failed, 160, 40).contains("Status: failed"));

    let (mut lost, lost_id) = qemu_running_workspace_app();
    let _ = yoctui_model::update(
        &mut lost,
        yoctui_model::Action::LoseQemuSession {
            id: lost_id,
            message: "runner lost".into(),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(rendered_text(&lost, 160, 40).contains("Status: lost"));

    let (mut cancelled, cancelled_id) = qemu_running_workspace_app();
    let _ = yoctui_model::update(
        &mut cancelled,
        yoctui_model::Action::BeginQemuSessionCancellation { id: cancelled_id },
    );
    let _ = yoctui_model::update(
        &mut cancelled,
        yoctui_model::Action::ConfirmQemuSessionCancellation,
    );
    let _ = yoctui_model::update(
        &mut cancelled,
        yoctui_model::Action::CancelQemuSession {
            id: cancelled_id,
            exit_code: Some(130),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(rendered_text(&cancelled, 160, 40).contains("Status: cancelled"));
}

#[test]
fn wic_workspace_renders_capability_dialogs_jobs_outputs_and_responsive_states() {
    let mut app = wic_workspace_app();
    for (width, height) in [(80, 24), (100, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        if width == 160 {
            assert!(output.contains("Wic capability"), "{output}");
            assert!(output.contains("ready for selected image"), "{output}");
        }
    }
    assert!(rendered_text(&app, 70, 20).contains("needs at least 80x24"));

    let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedWicCreate);
    for (width, height) in [(80, 24), (100, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Wic create.toml"), "{output}");
        assert!(output.contains("machine = \"qemux86-64\""), "{output}");
        assert!(output.contains("⟦/deploy/qemux86-64⟧▏"), "{output}");
        assert!(output.contains("Ctrl+V paste"), "{output}");
    }
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::ToggleWicCreateTomlEditor);
    assert!(rendered_text(&app, 80, 24).contains("INSERT"));
    if let Some(Dialog::WicCreateTomlEditor { editor, .. }) = app.active_dialog_mut() {
        editor.text = "machine = \"qemux86-64\"\nimage = \"core-image-minimal\"\nkickstart = \"directdisk\"\noutput_directory = \"relative-output\"\ngenerate_bmap = true\ncompression = \"none\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewWicCreate);
    assert!(rendered_text(&app, 80, 24).contains("Validation:"));
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::DismissNotification);
    if let Some(Dialog::WicCreateTomlEditor { editor, .. }) = app.active_dialog_mut() {
        editor.text = "machine = \"qemux86-64\"\nimage = \"core-image-minimal\"\nkickstart = \"directdisk\"\noutput_directory = \"/deploy/qemux86-64\"\ngenerate_bmap = true\ncompression = \"none\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewWicCreate);
    for (width, height) in [(80, 24), (100, 30), (160, 40)] {
        let confirmation = rendered_text(&app, width, height);
        assert!(
            confirmation.contains("Exact argument vector"),
            "{confirmation}"
        );
        assert!(confirmation.contains("Partitions"), "{confirmation}");
        assert!(
            confirmation.contains("part / --source=rootfs"),
            "{confirmation}"
        );
    }
    let Some(yoctui_model::Effect::StartWicSession { id, .. }) =
        yoctui_model::update(&mut app, yoctui_model::Action::ConfirmWicCreate)
    else {
        panic!("Wic start");
    };
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::WicSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::WicSessionRunning { id });
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::AppendWicSessionOutput {
            id,
            stream: yoctui_model::WicOutputStream::Stderr,
            line: "creation warning".into(),
            truncated: true,
            timestamp: SystemTime::UNIX_EPOCH,
        },
    );
    app.dialogs.push_front(Dialog::WicCancellationConfirmation {
        id,
        incomplete_device_warning: false,
    });
    assert!(rendered_text(&app, 80, 24).contains("Confirm Wic cancellation"));
    app.dialogs.clear();
    let output = yoctui_model::WicOutput {
        identity: yoctui_model::WicOutputIdentity {
            path: "/deploy/qemux86-64/core-image-minimal.wic".into(),
            size_bytes: 4096,
            modified_unix_seconds: 1,
        },
        kind: yoctui_model::WicOutputKind::Wic,
    };
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::CompleteWicSession {
            id,
            exit_code: 0,
            outputs: vec![output],
            limitations: vec!["one dynamic field".into()],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let rendered = rendered_text(&app, 160, 40);
    assert!(rendered.contains("Status: succeeded"), "{rendered}");
    assert!(rendered.contains("core-image-minimal.wic"), "{rendered}");
    assert!(
        rendered.contains("creation warning [truncated]"),
        "{rendered}"
    );

    for capability in [
        WicCapability::MissingTool,
        WicCapability::MissingKickstarts {
            executable: "/usr/bin/wic".into(),
        },
        WicCapability::Failed {
            message: "inspection denied".into(),
        },
    ] {
        app.wic_capability = capability;
        let rendered = rendered_text(&app, 160, 40);
        assert!(rendered.contains("Wic capability"));
    }
}

#[test]
fn wic_workspace_renders_all_capability_inventory_and_terminal_states() {
    let mut app = wic_workspace_app();
    for (capability, expected) in [
        (WicCapability::NotInspected, "not inspected"),
        (WicCapability::MissingTool, "missing wic executable"),
        (
            WicCapability::MissingKickstarts {
                executable: "/usr/bin/wic".into(),
            },
            "no kickstarts available",
        ),
        (
            WicCapability::Failed {
                message: "permission denied".into(),
            },
            "inspection failed: permission denied",
        ),
    ] {
        app.wic_capability = capability;
        let rendered = rendered_text(&app, 160, 40);
        assert!(rendered.contains(expected), "{rendered}");
    }

    let request = yoctui_model::WicOutputInventoryRequest {
        generation: 7,
        output_directory: "/deploy/qemux86-64".into(),
    };
    for (inventory, expected) in [
        (WicOutputInventoryState::NotLoaded, "not loaded"),
        (
            WicOutputInventoryState::Loading {
                request: request.clone(),
            },
            "loading generation 7",
        ),
        (
            WicOutputInventoryState::Failed {
                request: request.clone(),
                message: "scan denied".into(),
            },
            "failed generation 7",
        ),
        (
            WicOutputInventoryState::Available {
                request: request.clone(),
                outputs: Vec::new(),
            },
            "none generated",
        ),
        (
            WicOutputInventoryState::Partial {
                request,
                outputs: Vec::new(),
                limitations: vec!["symlink skipped".into()],
            },
            "symlink skipped",
        ),
    ] {
        app.wic_outputs = inventory;
        let rendered = rendered_text(&app, 160, 40);
        assert!(rendered.contains(expected), "{rendered}");
    }

    let output = yoctui_model::WicOutput {
        identity: yoctui_model::WicOutputIdentity {
            path: "/deploy/qemux86-64/selected.direct".into(),
            size_bytes: 8_192,
            modified_unix_seconds: 9,
        },
        kind: yoctui_model::WicOutputKind::Direct,
    };
    app.wic_output_selection = Some(output.identity.clone());
    app.wic_outputs = WicOutputInventoryState::Available {
        request: yoctui_model::WicOutputInventoryRequest {
            generation: 8,
            output_directory: "/deploy/qemux86-64".into(),
        },
        outputs: vec![output],
    };
    let rendered = rendered_text(&app, 160, 40);
    assert!(
        rendered.contains("▶ Direct /deploy/qemux86-64/selected.direct (8192 bytes, 9s)"),
        "{rendered}"
    );

    let mut lifecycle = wic_workspace_app();
    let _ = yoctui_model::update(&mut lifecycle, yoctui_model::Action::BeginSelectedWicCreate);
    let _ = yoctui_model::update(&mut lifecycle, yoctui_model::Action::PreviewWicCreate);
    let Some(yoctui_model::Effect::StartWicSession {
        id: lifecycle_id, ..
    }) = yoctui_model::update(&mut lifecycle, yoctui_model::Action::ConfirmWicCreate)
    else {
        panic!("expected Wic session");
    };
    assert!(rendered_text(&lifecycle, 160, 40).contains("Status: queued"));
    let _ = yoctui_model::update(
        &mut lifecycle,
        yoctui_model::Action::WicSessionStarting {
            id: lifecycle_id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(rendered_text(&lifecycle, 160, 40).contains("Status: starting"));
    let _ = yoctui_model::update(
        &mut lifecycle,
        yoctui_model::Action::WicSessionRunning { id: lifecycle_id },
    );
    assert!(rendered_text(&lifecycle, 160, 40).contains("Status: running"));
    let _ = yoctui_model::update(
        &mut lifecycle,
        yoctui_model::Action::BeginActiveWicSessionCancellation,
    );
    let _ = yoctui_model::update(
        &mut lifecycle,
        yoctui_model::Action::ConfirmWicSessionCancellation {
            id: lifecycle_id,
            acknowledge_incomplete_device: false,
        },
    );
    assert!(rendered_text(&lifecycle, 160, 40).contains("Status: cancelling"));

    let (mut succeeded, succeeded_id) = wic_running_workspace_app();
    let _ = yoctui_model::update(
        &mut succeeded,
        yoctui_model::Action::CompleteWicSession {
            id: succeeded_id,
            exit_code: 0,
            outputs: Vec::new(),
            limitations: Vec::new(),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(rendered_text(&succeeded, 160, 40).contains("Status: succeeded"));

    let (mut failed, failed_id) = wic_running_workspace_app();
    let _ = yoctui_model::update(
        &mut failed,
        yoctui_model::Action::FailWicSession {
            id: failed_id,
            message: "creator failed".into(),
            exit_code: Some(2),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let rendered = rendered_text(&failed, 160, 40);
    assert!(rendered.contains("Status: failed"), "{rendered}");
    assert!(rendered.contains("creator failed"), "{rendered}");

    let (mut lost, lost_id) = wic_running_workspace_app();
    let _ = yoctui_model::update(
        &mut lost,
        yoctui_model::Action::LoseWicSession {
            id: lost_id,
            message: "creator disappeared".into(),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(rendered_text(&lost, 160, 40).contains("Status: lost"));

    let (mut cancelled, cancelled_id) = wic_running_workspace_app();
    let _ = yoctui_model::update(
        &mut cancelled,
        yoctui_model::Action::BeginActiveWicSessionCancellation,
    );
    let _ = yoctui_model::update(
        &mut cancelled,
        yoctui_model::Action::ConfirmWicSessionCancellation {
            id: cancelled_id,
            acknowledge_incomplete_device: false,
        },
    );
    let _ = yoctui_model::update(
        &mut cancelled,
        yoctui_model::Action::CancelWicSession {
            id: cancelled_id,
            exit_code: Some(130),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(rendered_text(&cancelled, 160, 40).contains("Status: cancelled"));
}
