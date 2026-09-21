use super::*;

#[test]
fn test_workflow_dialogs_render_exact_previews_at_responsive_boundaries() {
    let (mut app, baseline, candidate) = test_workflow_results_app();
    app.focus = FocusTarget::Dialog;
    let mut editor = yoctui_model::PopupEditor::new("family = \"OE selftest\"\nmachine = \"qemux86-64\"\ndistro = \"poky\"\nimage = \"core-image-minimal\"\nscope = \"all\"\nselector = \"\"\nparallelism = 1\nverbose = false\nskip_network = false\n".into());
    editor.select_toml_value("scope").unwrap();
    app.dialogs.push_front(Dialog::TestLaunchTomlEditor {
        family: yoctui_model::TestFamily::OeSelftest,
        editor,
        validation_error: Some("typed launch validation".into()),
    });
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("Test launch.toml"), "{output}");
    assert!(output.contains("Home/End line"), "{output}");
    assert!(output.contains("typed launch validation"), "{output}");
    assert!(output.contains("⟦all⟧▏"), "{output}");

    app.dialogs.clear();
    let request = yoctui_model::TestSelftestRequest::new(
        "/workspace/oe-selftest".into(),
        yoctui_model::TestFamily::OeSelftest,
        Some("tinfoil.Case.test_one".into()),
        4,
        false,
        false,
    )
    .unwrap();
    app.dialogs.push_front(Dialog::TestLaunchConfirmation(
        yoctui_model::TestLaunchPreview::Selftest(request),
    ));
    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("Confirm Testing launch"), "{output}");
    assert!(output.contains("[0] /workspace/oe-selftest"), "{output}");

    app.dialogs.clear();
    app.dialogs.push_front(Dialog::TestCancellationConfirmation(
        yoctui_model::TestSessionId(9),
    ));
    assert!(rendered_text(&app, 80, 24).contains("Confirm Testing cancellation"));

    app.dialogs.clear();
    let mut import_editor =
        yoctui_model::PopupEditor::new("root = \"/results/testresults.json\"\n".into());
    import_editor.select_toml_value("root").unwrap();
    app.dialogs.push_front(Dialog::TestResultImportTomlEditor {
        editor: import_editor,
        validation_error: Some("normalized absolute path required".into()),
    });
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("Test result import.toml"), "{output}");
    assert!(output.contains("Home/End line"), "{output}");
    assert!(
        output.contains("normalized absolute path required"),
        "{output}"
    );

    app.dialogs.clear();
    let mut comparison_editor = yoctui_model::PopupEditor::new(format!(
        "baseline = \"{}\"\ncandidate = \"{}\"\n",
        baseline.identity.path.display(),
        candidate.identity.path.display()
    ));
    comparison_editor.select_toml_value("baseline").unwrap();
    app.dialogs.push_front(Dialog::TestComparisonTomlEditor {
        editor: comparison_editor,
        validation_error: None,
    });
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("Test comparison.toml"), "{output}");
    assert!(output.contains("Ctrl+C copy"), "{output}");

    app.dialogs.clear();
    let import = yoctui_model::TestResultImportDialog {
        input: "/results/testresults.json".into(),
        ..Default::default()
    };
    app.dialogs.push_front(Dialog::TestResultImport(import));
    assert!(rendered_text(&app, 80, 24).contains("Import structured test results"));

    app.dialogs.clear();
    let records = app.test_results.records().to_vec();
    app.dialogs.push_front(Dialog::TestComparison(
        yoctui_model::TestComparisonPicker::new(Some(baseline.identity.clone()), &records),
    ));
    assert!(rendered_text(&app, 100, 30).contains("Choose exact comparison inputs"));

    app.dialogs.clear();
    let comparison_request = yoctui_model::TestComparisonRequest::new(
        2,
        baseline.identity.clone(),
        candidate.identity.clone(),
    )
    .unwrap();
    app.dialogs.push_front(Dialog::TestComparisonConfirmation(
        yoctui_model::TestComparisonPreview::new(
            "/workspace/resulttool".into(),
            comparison_request,
        )
        .unwrap(),
    ));
    let output = rendered_text(&app, 160, 40);
    assert!(output.contains("Confirm result comparison"), "{output}");
    assert!(output.contains("[1] regression-file"), "{output}");

    app.dialogs.clear();
    let mut junit = yoctui_model::TestJunitExportDialog::new(candidate.identity.clone());
    junit.destination_input = "/exports/results.xml".into();
    app.dialogs.push_front(Dialog::TestJunitExport(junit));
    assert!(rendered_text(&app, 80, 24).contains("JUnit export destination"));

    app.dialogs.clear();
    let export = yoctui_model::TestJunitExportRequest {
        generation: 3,
        result: candidate.identity,
        destination: "/exports/results.xml".into(),
    };
    app.dialogs.push_front(Dialog::TestJunitExportConfirmation(
        yoctui_model::TestJunitExportPreview::new("/workspace/resulttool".into(), export).unwrap(),
    ));
    for (width, height) in [(80, 24), (100, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Confirm JUnit export"), "{output}");
        assert!(output.contains("never overwrites"), "{output}");
    }
}

#[test]
fn popup_editor_renders_persistent_shortcut_row() {
    let mut app = App::new(10, 1_000);
    let mut editor =
        yoctui_model::PopupEditor::new("destination = \"/exports/result.xml\"\n".into());
    editor.select_toml_value("destination").unwrap();
    app.dialogs.push_front(Dialog::TestJunitTomlEditor {
        result: yoctui_model::TestResultIdentity {
            path: "/results/testresults.json".into(),
            fingerprint: "fixture".into(),
            byte_size: 1,
            modified_at: SystemTime::UNIX_EPOCH,
        },
        editor,
        validation_error: None,
    });
    for (width, height) in [(80, 24), (100, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("⟦/exports/result.xml⟧▏"), "{output}");
        assert!(output.contains("Home/End line"), "{output}");
        assert!(output.contains("Ctrl+C copy"), "{output}");
        assert!(output.contains("Ctrl+V paste"), "{output}");
    }
}

#[test]
fn ux_textarea_renders_model_modes_lines_search_validation_and_diff_responsively() {
    let mut app = App::new(10, 1_000);
    let mut editor = yoctui_model::PopupEditor::new("name = \"猫\"\nvalue = \"café\"\n".into());
    editor.move_cursor(yoctui_model::TextAreaMotion::DocumentStart);
    editor.set_mode(yoctui_model::TextAreaMode::Insert);
    editor.insert("# edited\n");
    editor.search("café", true).unwrap();
    editor.set_mode(yoctui_model::TextAreaMode::Visual);
    editor.move_cursor(yoctui_model::TextAreaMotion::WordRight);
    editor.layout_mut().wrap_width = Some(24);
    editor.set_validation([yoctui_model::TextAreaValidationSpan {
        start: 0,
        end: 8,
        severity: yoctui_model::TextAreaValidationSeverity::Warning,
        message: "review the edited header".into(),
    }]);
    editor.preview_diff();
    app.dialogs
        .push_front(Dialog::BuildEnvironmentEditor(editor));

    for (width, height) in [(160, 50), (100, 30), (80, 24), (40, 10)] {
        let output = rendered_text(&app, width, height);
        if height >= 24 {
            assert!(output.contains("VISUAL"), "{width}x{height}: {output}");
            assert!(output.contains("UTF-8"), "{output}");
            assert!(output.contains("diff preview"), "{output}");
            assert!(output.contains("find 1/1"), "{output}");
            assert!(output.contains("Warning bytes 0..8"), "{output}");
            assert!(output.contains("1 │"), "{output}");
            assert!(output.contains("v visual"), "{output}");
            assert!(output.contains("u undo"), "{output}");
        } else {
            assert!(output.contains("needs at least 80x24"), "{output}");
        }
        assert!(!output.contains('�'), "{output}");
    }

    app.color_enabled = false;
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("VISUAL"), "{output}");
    assert!(output.contains("Warning bytes 0..8"), "{output}");
}

#[test]
fn ux_textarea_save_failure_and_conflict_keep_textual_recovery_meaning() {
    let mut app = App::new(10, 1_000);
    let mut editor = yoctui_model::PopupEditor::new("value = 1\n".into());
    editor.set_mode(yoctui_model::TextAreaMode::Insert);
    editor.insert("# local\n");
    let base = yoctui_model::TextAreaRevision::of("value = 1\n");
    let request = editor.begin_atomic_save("/tmp/value.conf", base).unwrap();
    assert!(editor.mark_save_failed("disk full", true));
    app.dialogs
        .push_front(Dialog::BuildEnvironmentEditor(editor));
    let failed = rendered_text(&app, 100, 30);
    assert!(failed.contains("save failed · retry available"), "{failed}");
    assert!(failed.contains("SAVE FAILED: disk full"), "{failed}");

    let Dialog::BuildEnvironmentEditor(editor) = app.dialogs.front_mut().unwrap() else {
        unreachable!()
    };
    let retry = editor.retry_save().unwrap();
    assert_eq!(retry, request);
    assert!(editor.mark_saved(&retry));
    editor.insert("changed");
    assert!(editor.begin_atomic_save("/tmp/value.conf", base).is_none());
    let conflict = rendered_text(&app, 100, 30);
    assert!(conflict.contains("external conflict"), "{conflict}");
    assert!(conflict.contains("CONFLICT:"), "{conflict}");
}

#[test]
fn ux_checkbox_renderer_preserves_unicode_ascii_focus_and_disabled_meaning() {
    let mut row = yoctui_model::CheckboxState::new("pkg:busybox", "busybox");
    row.focused = true;
    row.toggle();
    assert_eq!(checkbox_text(&row, true), "> ☑ busybox (checked)");
    assert_eq!(checkbox_text(&row, false), "> [x] busybox (checked)");
    row.set_disabled("dependency is required");
    let disabled = checkbox_text(&row, false);
    assert!(disabled.contains("(disabled)"), "{disabled}");
    assert!(disabled.contains("dependency is required"), "{disabled}");
}

#[test]
fn image_console_dialog_renders_qemu_and_ssh_authority_and_safety() {
    let mut app = qemu_workspace_app();
    app.ssh_client_capability = yoctui_model::SshClientCapability::Available {
        executable: "/usr/bin/ssh".into(),
    };
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedImageConsole);
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(
            output.contains("Image Console"),
            "{width}x{height}: {output}"
        );
        assert!(output.contains("Boot with QEMU"), "{output}");
        assert!(output.contains("nographic +"), "{output}");
        assert!(output.contains("serialstdio"), "{output}");
        assert!(output.contains("[Enter] Launch"), "{output}");
    }

    let Dialog::ImageConsole(dialog) = app.dialogs.front_mut().unwrap() else {
        unreachable!()
    };
    dialog.draft.mode = ImageConsoleMode::Ssh;
    dialog.selected_field = ImageConsoleField::Host;
    dialog.draft.host = "target.example".into();
    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("Connect over SSH"), "{output}");
    assert!(output.contains("already-running target"), "{output}");
    assert!(output.contains("host-key policy stays enabled"), "{output}");
    assert!(output.contains("/usr/bin/ssh"), "{output}");
    assert!(output.contains("never stored"), "{output}");

    let Dialog::ImageConsole(dialog) = app.dialogs.front_mut().unwrap() else {
        unreachable!()
    };
    dialog.validation_error = Some("SSH host is required".into());
    let output = rendered_text(&app, 80, 24);
    assert!(
        output.contains("Cannot launch: SSH host is required"),
        "{output}"
    );
}

#[test]
fn qemu_workspace_renders_capability_dialogs_session_and_responsive_states() {
    let mut app = qemu_workspace_app();
    app.focus = FocusTarget::Workspace;
    for (width, height) in [(80, 24), (100, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        if width == 160 {
            assert!(output.contains("runqemu capability"), "{output}");
            assert!(output.contains("ready for selected artifact"), "{output}");
        }
    }
    assert!(rendered_text(&app, 70, 20).contains("needs at least 80x24"));

    let artifact = app.selected_image_artifact().unwrap().clone();
    let mut launch = QemuLaunchDialog::new(yoctui_model::QemuLaunchDraft::for_artifact(
        artifact.identity,
        artifact.kind,
    ));
    launch.selected_field = QemuLaunchField::Kernel;
    launch.editing = true;
    launch.draft.kernel = "relative/kernel".into();
    launch.validation_error = Some("kernel path must be absolute".into());
    app.dialogs.push_front(Dialog::QemuLaunch(launch.clone()));
    for (width, height) in [(80, 24), (100, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Launch runqemu"), "{output}");
        assert!(output.contains("Kernel"), "{output}");
        assert!(output.contains("editing"), "{output}");
        assert!(output.contains("Validation"), "{output}");
    }

    app.dialogs.clear();
    let preview = launch.draft.preview(&app.qemu_capability).unwrap_err();
    assert!(preview.contains("normalized absolute"));
    launch.draft.kernel.clear();
    let preview = launch.draft.preview(&app.qemu_capability).unwrap();
    app.dialogs
        .push_front(Dialog::QemuLaunchConfirmation(preview));
    let confirmation = rendered_text(&app, 100, 30);
    assert!(
        confirmation.contains("Exact argument vector"),
        "{confirmation}"
    );
    assert!(confirmation.contains("qemumemory=1024"), "{confirmation}");

    app.dialogs.clear();
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedQemuLaunch);
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewQemuLaunch);
    let Some(yoctui_model::Effect::StartQemuSession { id, .. }) =
        yoctui_model::update(&mut app, yoctui_model::Action::ConfirmQemuLaunch)
    else {
        panic!("expected session");
    };
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::QemuSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::QemuSessionRunning { id });
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::AppendQemuSessionOutput {
            id,
            stream: yoctui_model::QemuOutputStream::Stderr,
            line: "guest warning".into(),
            truncated: true,
            timestamp: SystemTime::UNIX_EPOCH,
        },
    );
    app.focus = FocusTarget::Workspace;
    let running = rendered_text(&app, 160, 40);
    assert!(running.contains("Status: running"), "{running}");
    assert!(
        running.contains("[stderr] guest warning [truncated]"),
        "{running}"
    );

    app.dialogs
        .push_front(Dialog::QemuCancellationConfirmation(id));
    let cancellation = rendered_text(&app, 80, 24);
    assert!(
        cancellation.contains("Confirm runqemu cancellation"),
        "{cancellation}"
    );
    app.dialogs.clear();
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::FailQemuSession {
            id,
            message: "display unavailable".into(),
            exit_code: Some(1),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let failed = rendered_text(&app, 160, 40);
    assert!(failed.contains("Status: failed"), "{failed}");
    assert!(failed.contains("display unavailable"), "{failed}");

    for capability in [
        QemuCapability::MissingTool,
        QemuCapability::MissingCompatibleImage,
        QemuCapability::Failed {
            message: "inspection denied".into(),
        },
    ] {
        app.qemu_capability = capability;
        let output = rendered_text(&app, 160, 40);
        assert!(output.contains("runqemu capability"), "{output}");
    }
}
