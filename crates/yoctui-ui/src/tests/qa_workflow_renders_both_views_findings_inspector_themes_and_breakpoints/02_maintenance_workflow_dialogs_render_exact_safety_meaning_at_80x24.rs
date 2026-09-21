#[test]
fn maintenance_workflow_dialogs_render_exact_safety_meaning_at_80x24() {
    let mut app = maintenance_workflow_ui_app();
    let ordinary = maintenance_preview(21);
    let cleanup_request = yoctui_model::SstateCleanupRequest::new(
        "/cache/sstate".into(),
        Vec::new(),
        vec![yoctui_model::SstateCleanupMode::Duplicates],
        1,
    )
    .unwrap();
    let cleanup = yoctui_model::MaintenanceOperationPreview::new(
        22,
        7,
        yoctui_model::MaintenanceOperation::SstateCleanup(
            yoctui_model::SstateCleanupPreview::new(
                cleanup_request,
                vec![maintenance_identity("/cache/sstate/a.tgz")],
            )
            .unwrap(),
        ),
        vec![
            "/tools/sstate-cache-management.py".into(),
            "--remove-duplicated".into(),
        ],
        vec!["files may be removed".into()],
    )
    .unwrap();
    let archive = yoctui_model::GitArchiveRequest::new(yoctui_model::GitArchiveRequest {
        data_dir: "/data".into(),
        git_dir: "/archive/release.git".into(),
        create: false,
        bare: false,
        create_tag: false,
        branch_name: "main".into(),
        tag_name: None,
        commit_subject: "archive".into(),
        commit_body: String::new(),
        tag_subject: "tag".into(),
        tag_body: String::new(),
        exclusions: Vec::new(),
        notes: Vec::new(),
        push_remote: Some("origin".into()),
    })
    .unwrap();
    let network = yoctui_model::MaintenanceOperationPreview::new(
        23,
        7,
        yoctui_model::MaintenanceOperation::GitArchive(archive),
        vec![
            "/tools/oe-git-archive".into(),
            "--push".into(),
            "origin".into(),
        ],
        Vec::new(),
    )
    .unwrap();
    for (dialog, expected) in [
        (
            MaintenanceDialog::Confirm(ordinary),
            "[0] /tools/oe-check-sstate",
        ),
        (
            MaintenanceDialog::Confirm(cleanup.clone()),
            "Confirm destructive Maintenance operation",
        ),
        (
            MaintenanceDialog::CleanupPhrase {
                preview: cleanup,
                input: "DELETE".into(),
            },
            "Typing alone cannot delete files",
        ),
        (
            MaintenanceDialog::ConfirmNetworkPush(network),
            "separately confirmed remote push",
        ),
        (
            MaintenanceDialog::ConfirmCancellation(yoctui_model::MaintenanceSessionId(11)),
            "partially cleaned cache",
        ),
    ] {
        app.dialogs.clear();
        app.dialogs
            .push_front(Dialog::Maintenance(Box::new(dialog)));
        app.focus = FocusTarget::Dialog;
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains(expected), "{output}");
    }
}

#[test]
fn maintenance_sstate_workspace_renders_forms_validation_and_responsive_fields() {
    let mut app = maintenance_workflow_ui_app();
    let readiness = yoctui_model::PopupEditor::new(
            "targets = \"core-image-minimal busybox\"\nmode = \"isolated_tmpdir\"\noutput = \"/build/sstate.txt\"\nlog = \"\"\ntimeout = 0\n".into(),
        );
    app.dialogs.push_front(Dialog::Maintenance(Box::new(
        MaintenanceDialog::ReadinessToml {
            editor: readiness,
            validation_error: Some("timeout must be a positive integer".into()),
        },
    )));
    app.focus = FocusTarget::Dialog;
    for (width, height) in [(160, 40), (100, 26), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(
            output.contains("Sstate readiness.toml"),
            "{width}: {output}"
        );
        assert!(
            output.contains("core-image-minimal busybox"),
            "{width}: {output}"
        );
        assert!(
            output.contains("timeout must be a positive"),
            "{width}: {output}"
        );
        assert!(output.contains("Home/End line"), "{width}: {output}");
    }

    let cleanup = yoctui_model::PopupEditor::new(
            "# Cache (read-only): /cache/sstate\n# Stamps (read-only): /build/tmp/stamps\nduplicates = true\norphans = true\nunreferenced_by_stamps = false\njobs = 1\n".into(),
        );
    app.dialogs.clear();
    app.dialogs.push_front(Dialog::Maintenance(Box::new(
        MaintenanceDialog::CleanupToml {
            editor: cleanup,
            validation_error: Some("jobs must be a positive integer".into()),
        },
    )));
    for theme in [Theme::DarkPro, Theme::WhiteClassic, Theme::Monochrome] {
        app.theme = theme;
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("Sstate cleanup.toml"), "{output}");
        assert!(output.contains("/cache/sstate"), "{output}");
        assert!(output.contains("orphans = true"), "{output}");
        assert!(output.contains("jobs must be a positive"), "{output}");
        assert!(output.contains("Home/End line"), "{output}");
    }
}

#[test]
fn maintenance_service_workspace_renders_exact_context_and_side_effects() {
    let mut app = maintenance_workflow_ui_app();
    let metadata = app
        .maintenance
        .capability
        .snapshot()
        .unwrap()
        .metadata
        .clone();
    for operation in [
        yoctui_model::PrServiceOperation::Export,
        yoctui_model::PrServiceOperation::Import,
    ] {
        let editor = yoctui_model::PopupEditor::new(format!(
            "# Build directory (read-only): {}\n# Endpoint (read-only): {}\nfile = \"/build/pr-data.inc\"\n",
            metadata.build_dir.as_ref().unwrap().display(),
            metadata.prserv_host.as_ref().unwrap(),
        ));
        app.dialogs.clear();
        app.dialogs.push_front(Dialog::Maintenance(Box::new(
            MaintenanceDialog::PrServiceToml {
                operation,
                editor,
                validation_error: (operation == yoctui_model::PrServiceOperation::Export)
                    .then(|| "destination parent is unavailable".into()),
            },
        )));
        app.focus = FocusTarget::Dialog;
        for (width, height) in [(160, 40), (100, 26), (80, 24)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("/build/pr-data.inc"), "{width}: {output}");
            assert!(output.contains("localhost:8585"), "{width}: {output}");
            assert!(output.contains("Home/End line"), "{width}: {output}");
            if operation == yoctui_model::PrServiceOperation::Import {
                assert!(output.contains("PR service import.toml"), "{output}");
            } else {
                assert!(output.contains("destination parent"), "{output}");
            }
        }
    }
}

#[test]
fn maintenance_release_locked_workspace_renders_context_warning_and_validation_responsively() {
    let mut app = maintenance_workflow_ui_app();
    let editor = yoctui_model::PopupEditor::new(
            "# Native LSB (read-only): ubuntu-24.04\nlocked_signatures = \"/build/conf/locked-sigs.inc\"\ninput_cache = \"/cache/input\"\noutput_cache = \"/cache/release\"\nfilter = \"/build/conf/filter.inc\"\n".into(),
        );
    app.dialogs.push_front(Dialog::Maintenance(Box::new(
        MaintenanceDialog::LockedCacheToml {
            editor,
            validation_error: Some("input and output cache must differ".into()),
        },
    )));
    app.focus = FocusTarget::Dialog;
    for (width, height) in [(160, 40), (100, 26), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Locked cache.toml"), "{width}: {output}");
        assert!(output.contains("/cache/release"), "{width}: {output}");
        assert!(output.contains("ubuntu-24.04"), "{width}: {output}");
        assert!(output.contains("Home/End line"), "{width}: {output}");
        assert!(
            output.contains("input and output cache must differ"),
            "{width}: {output}"
        );
    }
}

#[test]
fn maintenance_release_history_workspace_renders_exact_choices_responsively() {
    let mut app = maintenance_workflow_ui_app();
    let editor = yoctui_model::PopupEditor::new("# Repository (read-only): /build/buildhistory\nfrom_revision = \"HEAD~2\"\nto_revision = \"HEAD\"\nreport_version = false\nreport_all = false\nsignatures = true\nsignature_diff = true\nexclude_paths = \"images/*,packages/*\"\nno_colour = true\n".into());
    app.dialogs.push_front(Dialog::Maintenance(Box::new(
        MaintenanceDialog::BuildHistoryToml {
            editor,
            validation_error: Some("repository identity changed".into()),
        },
    )));
    app.focus = FocusTarget::Dialog;
    for (width, height) in [(160, 40), (100, 26), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Build history.toml"), "{width}: {output}");
        assert!(output.contains("/build/buildhistory"), "{width}: {output}");
        assert!(output.contains("HEAD~2"), "{width}: {output}");
        assert!(
            output.contains("signature_diff = true"),
            "{width}: {output}"
        );
        assert!(output.contains("no_colour = true"), "{width}: {output}");
        assert!(
            output.contains("repository identity changed"),
            "{width}: {output}"
        );
    }
}

#[test]
fn maintenance_release_archive_workspace_renders_local_and_network_intent_responsively() {
    let mut app = maintenance_workflow_ui_app();
    let editor = yoctui_model::PopupEditor::new("data_dir = \"/release/data\"\ngit_dir = \"/release/archive.git\"\ncreate = true\nbare = true\ncreate_tag = true\nbranch_name = \"release/{machine}\"\ntag_name = \"release/{tag_number}\"\ncommit_subject = \"Release {commit}\"\ncommit_body = \"\"\ntag_subject = \"Release tag {tag_number}\"\ntag_body = \"\"\nexclusions = \"tmp/*,downloads/*\"\nnotes = \"release=/release/note.txt\"\npush_remote = \"origin\"\n".into());
    app.dialogs.push_front(Dialog::Maintenance(Box::new(
        MaintenanceDialog::GitArchiveToml {
            editor,
            validation_error: Some("note identity changed".into()),
        },
    )));
    app.focus = FocusTarget::Dialog;
    for (width, height) in [(160, 40), (100, 26), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Git archive.toml"), "{width}: {output}");
        assert!(output.contains("/release/archive.git"), "{width}: {output}");
        assert!(output.contains("bare = true"), "{width}: {output}");
        if width == 160 {
            assert!(output.contains("origin"), "{width}: {output}");
        }
        assert!(output.contains("Home/End line"), "{width}: {output}");
        assert!(
            output.contains("note identity changed"),
            "{width}: {output}"
        );
    }
}

#[test]
fn raw_output_renders_exact_job_streams_controls_and_bounds_responsively() {
    let mut app = raw_output_app(yoctui_model::RawInteractionMode::NoninteractiveJob);
    app.raw_mode.output.query = "café".into();
    app.raw_mode.output.follow = false;
    for (width, height) in [(160, 40), (100, 28), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("output.fixture"), "{width}: {output}");
        assert!(output.contains("Noninteractive job"), "{width}: {output}");
        assert!(output.contains("RUNNING"), "{width}: {output}");
        assert!(output.contains("stdout"), "{width}: {output}");
        assert!(output.contains("dropped"), "{width}: {output}");
        assert!(output.contains("c Cancel"), "{width}: {output}");
        assert!(output.contains("d Detach"), "{width}: {output}");
        assert!(output.contains("r Reattach"), "{width}: {output}");
    }
    let wide = rendered_text(&app, 160, 40);
    assert!(wide.contains("matching café output"), "{wide}");
    assert!(wide.contains("warning output"), "{wide}");
    assert!(wide.contains("hits 1"), "{wide}");
}

#[test]
fn raw_output_renders_daemon_pty_screen_without_parsing_terminal_bytes() {
    let app = raw_output_app(yoctui_model::RawInteractionMode::InteractivePty);
    for (width, height) in [(140, 32), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Interactive PTY"), "{width}: {output}");
        assert!(output.contains("interactive prompt"), "{width}: {output}");
        assert!(output.contains("typed terminal row"), "{width}: {output}");
        assert!(output.contains("80x24"), "{width}: {output}");
        assert!(!output.contains("\\x1b"), "{width}: {output}");
    }
}

#[test]
fn raw_output_renders_terminal_lost_replica_without_implying_live_ownership() {
    let mut app = raw_output_app(yoctui_model::RawInteractionMode::NoninteractiveJob);
    let request = app.raw_mode.output.request.clone().unwrap();
    let execution = app.raw_mode.execution_states.get_mut(&request).unwrap();
    let event = yoctui_model::RawExecutionEvent {
        request_id: request,
        sequence: execution.cursor.sequence + 1,
        generation: execution.cursor.generation + 1,
        kind: yoctui_model::RawExecutionEventKind::Finished {
            result: yoctui_model::RawExecutionResult {
                outcome: yoctui_model::RawExecutionOutcome::Lost,
                exit_code: None,
                message: Some("daemon ownership was lost".into()),
                elapsed_ms: 44,
                durable_reference: None,
            },
        },
    };
    yoctui_model::reduce_raw_execution(execution, event).unwrap();
    let output = rendered_text(&app, 120, 28);
    assert!(output.contains("LOST"), "{output}");
    assert!(output.contains("daemon ownership was lost"), "{output}");
    assert!(output.contains("exit --"), "{output}");
}

#[test]
fn raw_preview_renders_exact_indexed_authority_without_a_command_string() {
    let output = rendered_raw_preview(100, 24);
    for expected in [
        "Run BitBake Command",
        "Command: preview.run",
        "Catalog: 7",
        "Capability generation: 9",
        "Build directory: /work/build",
        "bitbake.raw.cli=bitbake.raw.argv",
        "Exact fixture limitation.",
        "Exact indexed native argv:",
        "[0] bitbake",
        "[2] do_compile",
        "[3] EMPTY",
        "[4] café value",
    ] {
        assert!(output.contains(expected), "missing {expected:?}: {output}");
    }
    assert!(!output.contains("bitbake -c do_compile"), "{output}");
}

#[test]
fn raw_preview_degrades_safely_at_narrow_and_tiny_sizes() {
    let narrow = rendered_raw_preview(80, 24);
    assert!(narrow.contains("Run BitBake Command"), "{narrow}");
    assert!(narrow.contains("[4] café value"), "{narrow}");
    let tiny = rendered_raw_preview(12, 3);
    assert!(tiny.contains("Run Bit"), "{tiny}");
}

#[test]
fn raw_form_renders_exact_typed_fields_selectors_and_responsive_focus_trap() {
    let mut app = raw_command_list_app();
    app.focus = FocusTarget::Workspace;
    set_raw_command_query(&mut app, "--continue <target>");
    assert_eq!(
        update(
            &mut app,
            Action::RawMode(yoctui_model::RawModeAction::OpenSelected),
        ),
        None
    );
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Form);
    assert_eq!(app.focus, FocusTarget::Dialog);
    let selection = app.raw_mode.command.clone();
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        for expected in [
            "Run BitBake Command",
            "Template: bitbake --continue <target>",
            "Capability generation: 19",
            "Build directory: /work/build",
            "Target <target> · Target · Required · NORMAL",
            "Value: <required>",
            "Selector: 1 choice (authoritative)",
            "manual allowed",
            "Additional arguments",
            "Enter validates and opens exact preview",
        ] {
            assert!(
                output.contains(expected),
                "{width}x{height} missing {expected:?}: {output}"
            );
        }
        assert_eq!(app.raw_mode.command, selection);
        assert_eq!(app.focus, FocusTarget::Dialog);
    }
    app.color_enabled = false;
    let no_color = rendered_text(&app, 80, 24);
    assert!(no_color.contains("NORMAL · i edit"), "{no_color}");
    assert!(!no_color.contains('�'), "{no_color}");
}
