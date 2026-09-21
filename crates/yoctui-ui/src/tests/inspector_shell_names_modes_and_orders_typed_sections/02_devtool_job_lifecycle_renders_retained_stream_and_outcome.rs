#[test]
fn devtool_job_lifecycle_renders_retained_stream_and_outcome() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Recipes;
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "demo".into(),
        ..yoctui_model::Recipe::default()
    });
    let id = yoctui_model::BackgroundJobId(1_u64 << 63);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
            id,
            kind: BackgroundJobKind::Devtool,
            title: "Devtool modify demo".into(),
            context: yoctui_model::BackgroundJobContext {
                workspace: Some(Screen::Recipes),
                recipe: Some("demo".into()),
                ..yoctui_model::BackgroundJobContext::default()
            },
            cancellation_supported: true,
            queued_at: UNIX_EPOCH,
        }),
    );
    let _ = update(
        &mut app,
        Action::StartBackgroundJob {
            id,
            started_at: UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::RunBackgroundJob { id });
    let _ = update(
        &mut app,
        Action::AppendBackgroundJobOutput {
            id,
            entry: yoctui_model::BackgroundJobOutputEntry {
                severity: Severity::Info,
                message: "workspace prepared".into(),
                source: yoctui_model::BackgroundJobOutputSource::Stderr,
                truncated: true,
                timestamp: UNIX_EPOCH,
            },
        },
    );
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id,
            result: yoctui_model::BackgroundJobResult {
                summary: "Devtool completed successfully".into(),
                artifacts: vec![],
            },
            finished_at: UNIX_EPOCH,
        },
    );

    let output = rendered_text(&app, 200, 44);
    assert!(
        output.contains("Devtool modify demo [Succeeded]"),
        "{output}"
    );
    assert!(
        output.contains("Stderr: workspace prepared [truncated]"),
        "{output}"
    );
    assert!(
        output.contains("outcome: Devtool completed") && output.contains("successfully."),
        "{output}"
    );
}
#[test]
fn recipe_qa_action_renders_capabilities_confirmation_and_honest_results() {
    for (width, height) in [(160, 32), (110, 28), (90, 25)] {
        let mut app = App::new(10, 1_000);
        app.dialogs
            .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["busybox".into()],
                task: Some("cve_check".into()),
                force: false,
            }));
        let output = rendered_text(&app, width, height);
        assert!(output.contains("bitbake busybox -c cve_check"), "{output}");
    }

    let mut app = App::new(20, 4_000);
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Workspace;
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "busybox".into(),
        ..yoctui_model::Recipe::default()
    });
    app.recipe_metadata.insert(
        "busybox".into(),
        yoctui_model::RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec!["do_cve_check".into(), "do_create_spdx".into()]),
            ..yoctui_model::RecipeMetadata::default()
        },
    );
    let id = yoctui_model::BackgroundJobId(7);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
            id,
            kind: BackgroundJobKind::Spdx,
            title: "SPDX generation busybox".into(),
            context: yoctui_model::BackgroundJobContext {
                workspace: Some(Screen::Recipes),
                recipe: Some("busybox".into()),
                task: Some("create_spdx".into()),
                ..yoctui_model::BackgroundJobContext::default()
            },
            cancellation_supported: true,
            queued_at: SystemTime::UNIX_EPOCH,
        }),
    );
    let _ = update(
        &mut app,
        Action::StartBackgroundJob {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::RunBackgroundJob { id });
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id,
            result: yoctui_model::BackgroundJobResult {
                summary: "SPDX generation completed; BitBake reported no result path".into(),
                artifacts: vec![],
            },
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let output = rendered_text(&app, 200, 40);
    assert!(output.contains("QA actions: CVE check enabled"), "{output}");
    assert!(output.contains("SPDX generation enabled"), "{output}");
    assert!(
        output.contains("SPDX generation busybox [Succeeded]"),
        "{output}"
    );
    assert!(
        output.contains("artifacts: none") && output.contains("reported."),
        "{output}"
    );
    let contextual_footer = footer_shortcuts(&app);
    assert!(contextual_footer.contains("V CVE"), "{contextual_footer}");
    assert!(contextual_footer.contains("X SPDX"), "{contextual_footer}");

    app.recipe_metadata.get_mut("busybox").unwrap().tasks = Some(vec![]);
    let output = rendered_text(&app, 200, 40);
    assert!(
        output.contains("CVE check unavailable") && output.contains("do_cve_check not reported"),
        "{output}"
    );
    assert!(
        output.contains("SPDX generation unavailable")
            && output.contains("do_create_spdx not reported"),
        "{output}"
    );
}
#[test]
fn build_completion_is_modal_but_running_builds_keep_the_shell_visible() {
    let mut terminal = Terminal::new(TestBackend::new(160, 32)).unwrap();
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());
    app.build.status = yoctui_model::BuildStatus::Running;
    app.build.completed = 1;
    app.build.total = Some(2);
    app.host_telemetry.cpu_utilization_percent = Some(50);
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Dashboard"));
    assert!(output.contains("Overall"));
    assert!(output.contains("50%"));

    app.build.status = yoctui_model::BuildStatus::Completed;
    app.build.started = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1));
    app.build_history.push_back(yoctui_model::BuildRecord {
        target: app.build.target.clone(),
        success: true,
        exit_code: Some(0),
        elapsed: Some(Duration::from_secs(5)),
        completed_tasks: 1,
        warnings: 0,
        errors: 0,
    });
    app.dialogs.push_back(Dialog::BuildCompletion);
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Build finished"));
    assert!(output.contains("Press any key"));
    assert!(output.contains("Elapsed: 00:00:05"), "{output}");
}
#[test]
fn build_cancellation_completion_is_distinct_from_failure() {
    let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());
    app.build.status = yoctui_model::BuildStatus::Cancelled;
    app.dialogs.push_back(Dialog::BuildCompletion);
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Build was cancelled"));
    assert!(!output.contains("Build failed"));
}
#[test]
fn configuration_renders_bridge_provenance() {
    let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Configuration;
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemuarm".into());
    app.workspace
        .variable_provenance
        .insert("MACHINE".into(), "conf/local.conf:12".into());
    app.workspace.variable_provenance_chain.insert(
        "MACHINE".into(),
        vec![
            "meta/conf/bitbake.conf:1".into(),
            "conf/local.conf:12".into(),
        ],
    );
    let identity = yoctui_model::VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        yoctui_model::VariableDetail {
            identity,
            effective_value: Some("qemuarm".into()),
            unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
            provenance: Some("conf/local.conf:12".into()),
            operations: vec![
                yoctui_model::VariableOperation {
                    operation: "set".into(),
                    file: Some("meta/conf/bitbake.conf".into()),
                    line: Some(1),
                    value: Some("${DEFAULT_MACHINE}".into()),
                },
                yoctui_model::VariableOperation {
                    operation: "set".into(),
                    file: Some("conf/local.conf".into()),
                    line: Some(12),
                    value: Some("qemuarm".into()),
                },
            ],
            active_overrides: vec!["qemuarm".into(), "poky".into()],
        },
    );
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("conf/local.conf:12"));
    assert!(output.contains("meta/conf/bitbake.conf:1"));
}

#[test]
fn config_workspace_renders_lazy_partial_and_error_states_responsively() {
    for (width, height) in [(160, 30), (110, 26), (90, 24), (70, 20)] {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
        app.focus = FocusTarget::Workspace;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let identity = yoctui_model::VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_detail_loading.insert(identity.clone());
        let output = rendered_text(&app, width, height);
        if width >= 80 && height >= 24 {
            assert!(output.contains("Loading authoritative detail"), "{output}");
        }

        app.variable_detail_loading.clear();
        app.variable_detail_errors
            .insert(identity, "Tinfoil unavailable".into());
        let output = rendered_text(&app, width, height);
        if width >= 80 && height >= 24 {
            assert!(output.contains("Detail unavailable"), "{output}");
            assert!(output.contains("Tinfoil unavailable"), "{output}");
        }

        app.variable_detail_errors.clear();
        let identity = yoctui_model::VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_details.insert(
            identity.clone(),
            yoctui_model::VariableDetail {
                identity,
                effective_value: Some("qemux86-64".into()),
                unexpanded_value: None,
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            },
        );
        let output = rendered_text(&app, width, height);
        if width >= 110 && height >= 24 {
            assert!(output.contains("Unexpanded value: unavailable"), "{output}");
            assert!(output.contains("Operations:"), "{output}");
            assert!(output.contains("none reported"), "{output}");
        }
    }
}

#[test]
fn config_copy_renders_shortcuts_and_exact_availability_responsively() {
    for (width, height) in [(160, 32), (110, 28), (90, 24)] {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
        app.focus = FocusTarget::Workspace;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let identity = yoctui_model::VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_details.insert(
            identity.clone(),
            yoctui_model::VariableDetail {
                identity,
                effective_value: Some("qemux86-64".into()),
                unexpanded_value: None,
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            },
        );
        let output = rendered_text(&app, width, height);
        if width >= 80 && height >= 24 {
            assert!(output.contains("C effective: enabled"), "{output}");
            assert!(output.contains("U unexpanded: disabled"), "{output}");
        }
    }
}

#[test]
fn config_source_renders_typed_picker_and_disabled_reason_responsively() {
    for (width, height) in [(140, 30), (100, 26), (90, 24)] {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
        app.focus = FocusTarget::Workspace;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let unloaded = rendered_text(&app, width, height);
        if width >= 80 && height >= 24 {
            assert!(unloaded.contains("o source: disabled"), "{unloaded}");
        }
        app.dialogs.push_back(Dialog::ConfigSourcePicker(
            yoctui_model::ConfigSourcePicker {
                identity: yoctui_model::VariableIdentity {
                    name: "MACHINE".into(),
                    recipe: None,
                },
                sources: vec![
                    yoctui_model::ConfigSourceChoice {
                        operation: "set".into(),
                        path: "meta/conf/bitbake.conf".into(),
                        line: Some(10),
                    },
                    yoctui_model::ConfigSourceChoice {
                        operation: "override".into(),
                        path: "conf/local.conf".into(),
                        line: Some(12),
                    },
                ],
                selection: 1,
            },
        ));
        let output = rendered_text(&app, width, height);
        assert!(output.contains("MACHINE defining sources"), "{output}");
        assert!(output.contains("local.conf"), "{output}");
        assert!(output.contains("12"), "{output}");
    }
}
