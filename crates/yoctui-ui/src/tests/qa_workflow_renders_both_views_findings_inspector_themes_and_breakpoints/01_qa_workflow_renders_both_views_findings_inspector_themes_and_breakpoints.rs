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
