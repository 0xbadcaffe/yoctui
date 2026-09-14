//! Regression tests grouped around qa_workflow_renders_both_views_findings_inspector_themes_and_breakpoints.
use super::*;

#[test]
fn qa_workflow_renders_both_views_findings_inspector_themes_and_breakpoints() {
    let mut app = qa_workflow_ui_app();
    for (width, height) in [(160, 40), (120, 30), (90, 24), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Recipe & Kernel"), "{width}: {output}");
        assert!(output.contains("do_package_qa"), "{width}: {output}");
        assert!(output.contains("Partial"), "{width}: {output}");
    }
    app.qa.drilled = true;
    app.focus = FocusTarget::Inspector;
    let inspector = rendered_text(&app, 120, 30);
    assert!(
        inspector.contains("installed-vs-shipped mismatch"),
        "{inspector}"
    );
    assert!(inspector.contains("add the installed file"), "{inspector}");
    assert!(inspector.contains("insane.bbclass"), "{inspector}");

    app.focus = FocusTarget::Workspace;
    app.qa.view = QaView::LayerQa;
    let layer = rendered_text(&app, 160, 40);
    assert!(layer.contains("meta-demo"), "{layer}");
    assert!(layer.contains("/layers/meta-demo"), "{layer}");
    for (theme, color) in [
        (Theme::DarkPro, true),
        (Theme::WhiteClassic, true),
        (Theme::HighContrast, true),
        (Theme::Monochrome, false),
    ] {
        app.theme = theme;
        app.color_enabled = color;
        assert!(rendered_text(&app, 100, 24).contains("Layer QA"));
    }
}

#[test]
fn qa_workflow_renders_every_report_and_capability_state_distinctly() {
    let mut app = qa_workflow_ui_app();
    let request = yoctui_model::QaReportRequest::new(8, vec!["/build/reports".into()]).unwrap();
    for (inventory, expected) in [
        (
            yoctui_model::QaReportInventoryState::NotLoaded,
            "Reports not loaded",
        ),
        (
            yoctui_model::QaReportInventoryState::Loading {
                request: request.clone(),
            },
            "Loading report generation 8",
        ),
        (
            yoctui_model::QaReportInventoryState::AvailableEmpty {
                request: request.clone(),
            },
            "available-empty",
        ),
        (
            yoctui_model::QaReportInventoryState::Failed {
                request: request.clone(),
                kind: yoctui_model::QaReportFailureKind::PermissionDenied,
                message: "access denied".into(),
            },
            "permission denied",
        ),
        (
            yoctui_model::QaReportInventoryState::Cancelled {
                request: request.clone(),
            },
            "acquisition cancelled",
        ),
        (
            yoctui_model::QaReportInventoryState::TimedOut {
                request: request.clone(),
            },
            "acquisition timed out",
        ),
        (
            yoctui_model::QaReportInventoryState::Lost {
                request,
                message: "worker channel closed".into(),
            },
            "worker lost",
        ),
    ] {
        app.qa.inventory = inventory;
        let output = rendered_text(&app, 120, 28);
        assert!(output.contains(expected), "{expected}: {output}");
    }
    app.qa.capability = QaCapability::Inspecting;
    assert!(rendered_text(&app, 120, 28).contains("Inspecting recipe"));
    app.qa.capability = QaCapability::Failed("metadata denied".into());
    assert!(rendered_text(&app, 120, 28).contains("metadata denied"));
    app.qa.view = QaView::LayerQa;
    app.qa.layer_capability = QaLayerCapability::Failed("tool unsafe".into());
    assert!(rendered_text(&app, 120, 28).contains("tool unsafe"));
}

#[test]
fn qa_workflow_dialogs_render_exact_previews_at_responsive_boundaries() {
    let mut app = qa_workflow_ui_app();
    let scope = app.qa.scope.clone().unwrap();
    let check = app.qa.check_selection.clone().unwrap();
    let operation = yoctui_model::QaOperationPreview {
        id: yoctui_model::QaOperationId(11),
        check,
        family: yoctui_model::QaCheckFamily::RecipePackage,
        scope,
        request: BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("do_package_qa".into()),
            force: false,
        },
        indexed_arguments: vec![
            "0: bitbake".into(),
            "1: busybox".into(),
            "2: -c".into(),
            "3: package_qa".into(),
        ],
        report_roots: vec!["/build/tmp/log/qa".into()],
        limitations: Vec::new(),
    };
    let layer = app.qa.selected_layer().unwrap();
    let QaLayerRunCapability::Available {
        executable,
        arguments,
        report_roots,
    } = &layer.run
    else {
        panic!("expected layer capability")
    };
    let layer_operation = yoctui_model::QaLayerOperationPreview {
        id: yoctui_model::QaLayerOperationId(12),
        check: layer.check.clone(),
        layer: layer.identity.clone(),
        executable: executable.clone(),
        arguments: arguments.clone(),
        indexed_arguments: vec![
            format!("0: {}", executable.path.display()),
            format!("1: {}", layer.identity.root.display()),
        ],
        report_roots: report_roots.clone(),
        limitations: Vec::new(),
    };
    for (dialog, expected) in [
        (QaDialog::Operation(operation), "Indexed BitBake request"),
        (
            QaDialog::LayerOperation(layer_operation),
            "Indexed native vector",
        ),
        (
            QaDialog::Import {
                editor: {
                    let mut editor = yoctui_model::PopupEditor::new(
                        "# exact absolute report\nroot = \"/build/reports\"\n".into(),
                    );
                    editor.select_toml_value("root").unwrap();
                    editor
                },
                validation_error: Some("normalized absolute path required".into()),
            },
            "QA import.toml",
        ),
        (
            QaDialog::Cancellation {
                session: yoctui_model::QaSessionId(13),
                background_job: yoctui_model::BackgroundJobId(4),
            },
            "attached to build job 4",
        ),
        (
            QaDialog::LayerCancellation(yoctui_model::QaLayerSessionId(14)),
            "exact layer-QA session 14",
        ),
    ] {
        app.dialogs.clear();
        app.dialogs.push_front(Dialog::Qa(dialog));
        app.focus = FocusTarget::Dialog;
        for (width, height) in [(160, 40), (100, 26), (80, 24)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains(expected), "{width}: {output}");
            if expected == "QA import.toml" {
                assert!(output.contains("Home/End line"), "{width}: {output}");
                assert!(
                    output.contains("normalized absolute path required"),
                    "{width}: {output}"
                );
            }
        }
    }
}

#[test]
fn maintenance_workflow_renders_every_view_and_exact_typed_inspector() {
    let mut app = maintenance_workflow_ui_app();
    for (view, expected, action) in [
        (MaintenanceView::Sstate, "oe-check-sstate", "c check"),
        (
            MaintenanceView::Services,
            "bitbake-prserv-tool",
            "e PR export",
        ),
        (
            MaintenanceView::Release,
            "gen-lockedsig-cache",
            "l locked cache",
        ),
        (
            MaintenanceView::Integrations,
            "create-pull-request",
            "detection/inspection only",
        ),
    ] {
        app.maintenance.view = view;
        let output = rendered_text(&app, 120, 70);
        assert!(output.contains(expected), "{view:?}: {output}");
        let contextual_footer = footer_shortcuts(&app);
        assert!(
            contextual_footer.contains(action),
            "{view:?}: {contextual_footer}"
        );
        assert!(output.contains("Partial capability"), "{output}");
    }

    app.focus = FocusTarget::Inspector;
    app.maintenance.view = MaintenanceView::Services;
    let services = rendered_text(&app, 180, 120);
    assert!(services.contains("localhost:8585"), "{services}");
    assert!(services.contains("PID 42 bitbake-prserv"), "{services}");
    assert!(services.contains("observational"), "{services}");
    assert!(services.contains("Indexed native vector"), "{services}");
    assert!(services.contains("Selected evidence"), "{services}");

    app.maintenance.view = MaintenanceView::Integrations;
    let integrations = rendered_text(&app, 180, 120);
    for expected in [
        "/sources/poky/.git/HEAD",
        "/workspace/.repo/manifests/default.xml",
        "/config/toaster.conf",
        "process evidence is observational only",
    ] {
        assert!(integrations.contains(expected), "{integrations}");
    }
}

#[test]
fn maintenance_workflow_renders_terminal_states_responsively_in_every_theme() {
    let mut app = maintenance_workflow_ui_app();
    for status in [
        MaintenanceSessionStatus::Queued,
        MaintenanceSessionStatus::Running,
        MaintenanceSessionStatus::Cancelling,
        MaintenanceSessionStatus::Succeeded,
        MaintenanceSessionStatus::Failed,
        MaintenanceSessionStatus::Cancelled,
        MaintenanceSessionStatus::TimedOut,
        MaintenanceSessionStatus::Lost,
    ] {
        app.maintenance.sessions.back_mut().unwrap().status = status;
        let output = rendered_text(&app, 160, 40);
        assert!(output.contains(&format!("{status:?}")), "{output}");
    }
    for (width, expected) in [
        (130, "Inspector"),
        (100, "Maintenance"),
        (99, "Panes:"),
        (80, "Maintenance"),
    ] {
        let output = rendered_text(&app, width, 24);
        assert!(output.contains(expected), "{width}: {output}");
    }
    assert!(rendered_text(&app, 79, 24).contains("needs at least 80x24"));

    for theme in [
        Theme::DarkPro,
        Theme::WhiteClassic,
        Theme::MatrixGreen,
        Theme::HighContrast,
        Theme::Monochrome,
    ] {
        app.theme = theme;
        assert!(rendered_text(&app, 130, 30).contains("Maintenance"));
    }
    app.color_enabled = false;
    let mut terminal = Terminal::new(TestBackend::new(130, 30)).unwrap();
    terminal.draw(|frame| render(frame, &app)).unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| { cell.symbol() == "▶" && cell.modifier.contains(Modifier::REVERSED) })
    );
}

#[test]
fn maintenance_workflow_renders_loading_failed_disabled_and_unavailable_states() {
    let mut app = maintenance_workflow_ui_app();
    app.maintenance.view = MaintenanceView::Services;
    app.maintenance.capability = MaintenanceCapability::Loading(41);
    app.maintenance.services = MaintenanceServiceDiagnostics::Loading(42);
    let loading = rendered_text(&app, 160, 40);
    assert!(
        loading.contains("Inspecting capability (request 41)"),
        "{loading}"
    );
    assert!(loading.contains("loading request 42"), "{loading}");

    app.maintenance.capability = MaintenanceCapability::Failed {
        request: 43,
        message: "metadata unavailable".into(),
    };
    app.maintenance.services = MaintenanceServiceDiagnostics::Failed {
        request: 44,
        message: "process inspection unavailable".into(),
    };
    let failed = rendered_text(&app, 160, 40);
    assert!(failed.contains("metadata unavailable"), "{failed}");
    assert!(
        failed.contains("process inspection unavailable"),
        "{failed}"
    );

    let disabled = yoctui_model::ServiceDiagnostic::new(
        yoctui_model::ServiceKind::Hash,
        yoctui_model::ServiceState::Disabled,
        Vec::new(),
        Vec::new(),
        vec!["not configured".into()],
    )
    .unwrap();
    app.maintenance.services = MaintenanceServiceDiagnostics::Available {
        request: 45,
        services: vec![disabled],
    };
    let disabled = rendered_text(&app, 160, 40);
    assert!(disabled.contains("Hash: Disabled"), "{disabled}");

    app.maintenance.view = MaintenanceView::Integrations;
    app.maintenance.integrations = MaintenanceIntegrationDiagnostics::Loading(46);
    assert!(rendered_text(&app, 160, 40).contains("loading request 46"));
    app.maintenance.integrations = MaintenanceIntegrationDiagnostics::Failed {
        request: 47,
        message: "optional tools unavailable".into(),
    };
    assert!(rendered_text(&app, 160, 40).contains("optional tools unavailable"));
}

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

#[test]
fn raw_form_routes_manual_selector_and_argv_edits_to_exact_preview() {
    let mut app = raw_command_list_app();
    set_raw_command_query(&mut app, "--continue <target>");
    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::OpenSelected),
    );

    let request = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Enter).unwrap();
    assert_eq!(update(&mut app, Action::RawMode(request)), None);
    let invalid = rendered_text(&app, 100, 30);
    assert!(
        invalid.contains("ERROR: Raw parameter target is required"),
        "{invalid}"
    );
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Form);

    let choose = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Right).unwrap();
    assert!(matches!(
        choose,
        yoctui_model::RawModeAction::ChooseParameter { .. }
    ));
    let _ = update(&mut app, Action::RawMode(choose));
    let edit = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Char('e')).unwrap();
    let _ = update(&mut app, Action::RawMode(edit));
    for character in "vé".chars() {
        let action = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Char(character)).unwrap();
        let _ = update(&mut app, Action::RawMode(action));
    }
    let unicode_error = rendered_text(&app, 100, 30);
    assert!(unicode_error.contains("ERROR:"), "{unicode_error}");
    assert!(!unicode_error.contains('�'), "{unicode_error}");
    let normal = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Esc).unwrap();
    let _ = update(&mut app, Action::RawMode(normal));
    let choose = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Right).unwrap();
    let _ = update(&mut app, Action::RawMode(choose));

    let next = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Tab).unwrap();
    let _ = update(&mut app, Action::RawMode(next));
    let insert = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Char('i')).unwrap();
    let _ = update(&mut app, Action::RawMode(insert));
    for character in "--verbose".chars() {
        let action = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Char(character)).unwrap();
        let _ = update(&mut app, Action::RawMode(action));
    }
    let preview = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Enter).unwrap();
    assert_eq!(update(&mut app, Action::RawMode(preview)), None);
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Preview);
    assert_eq!(app.focus, FocusTarget::Dialog);
    let output = rendered_text(&app, 160, 50);
    for expected in [
        "Exact indexed native argv:",
        "[0] bitbake",
        "[1] --continue",
        "[2] busybox",
        "[3] --verbose",
    ] {
        assert!(output.contains(expected), "missing {expected:?}: {output}");
    }

    let back = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Esc).unwrap();
    let _ = update(&mut app, Action::RawMode(back));
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Form);
    let normal = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Esc).unwrap();
    let _ = update(&mut app, Action::RawMode(normal));
    let close = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Char('q')).unwrap();
    let _ = update(&mut app, Action::RawMode(close));
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Browser);
    assert_eq!(app.focus, FocusTarget::Workspace);
}
