#[test]
fn sdk_workflow_renders_all_dialogs_at_responsive_boundaries() {
    let mut app = sdk_workflow_ui_app();
    let _ = update(
        &mut app,
        Action::BeginSdkBuild(SdkBuildAction::Populate(SdkKind::Extensible)),
    );
    for (width, height) in [(80, 24), (100, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Confirm SDK build"), "{output}");
        assert!(output.contains("populate_sdk_ext"), "{output}");
    }

    app.dialogs.clear();
    app.focus = FocusTarget::Workspace;
    let _ = update(&mut app, Action::BeginSelectedSdkPublish);
    assert_eq!(app.focus, FocusTarget::Dialog);
    let editor = rendered_text(&app, 80, 24);
    assert!(editor.contains("destination = \"⟦▏⟧\""), "{editor}");
    assert!(editor.contains("Ctrl+V paste"), "{editor}");
    if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut() {
        editor.text = format!(
            "destination = \"/srv/{}\"\n",
            "long-destination-".repeat(80)
        );
        editor.cursor = editor.text.len();
    }
    assert!(rendered_text(&app, 80, 24).contains("SDK publish.toml"));
    if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut() {
        editor.text = "destination = \"/srv/sdk-publish\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::PreviewSdkPublish);
    let publish = rendered_text(&app, 80, 24);
    assert!(publish.contains("Confirm SDK publication"), "{publish}");
    assert!(publish.contains("[0]"), "{publish}");

    app.dialogs.clear();
    app.focus = FocusTarget::Dialog;
    let native_draft = yoctui_model::SdkNativeDraft {
        mode: SdkNativeMode::RunNative,
        extracted_root: format!("/opt/{}", "sdk-root-".repeat(80)),
        recipe: "cmake-native".into(),
        tool: "cmake".into(),
        arguments: vec!["--version".into(), "--trace".into()],
    };
    app.dialogs
        .push_front(Dialog::SdkNative(SdkNativeDialog::new(native_draft)));
    let native = rendered_text(&app, 80, 24);
    assert!(native.contains("SDK native tool"), "{native}");
    assert!(native.contains("▶ Mode"), "{native}");
    let _ = update(&mut app, Action::SelectSdkNativeField { delta: 2 });
    let _ = update(&mut app, Action::ActivateSdkNativeField);
    let editing = rendered_text(&app, 80, 24);
    assert!(editing.contains("[editing]"), "{editing}");

    let preview = yoctui_model::SdkNativePreview::new(yoctui_model::SdkNativeRequest {
        executable: "/workspace/scripts/oe-run-native".into(),
        mode: SdkNativeMode::RunNative,
        extracted_root: Some("/opt/extracted-sdk".into()),
        recipe: "cmake-native".into(),
        tool: Some("cmake".into()),
        arguments: vec!["--version".into(), "x".repeat(1024)],
    })
    .unwrap();
    app.dialogs.clear();
    app.dialogs
        .push_front(Dialog::SdkNativeConfirmation(preview));
    for (width, height) in [(80, 24), (100, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Confirm SDK native tool"), "{output}");
        assert!(output.contains("Exact indexed"), "{output}");
    }

    let (mut running, id) = sdk_workflow_running_ui_app();
    running
        .dialogs
        .push_front(Dialog::SdkCancellationConfirmation(id));
    running.focus = FocusTarget::Dialog;
    let output = rendered_text(&running, 80, 24);
    assert!(output.contains("Confirm SDK cancellation"), "{output}");
    assert!(output.contains("Enter requests cancellation"), "{output}");
}

#[test]
fn test_workflow_screen_renders_identity_capability_and_selection_responsively() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Testing;
    app.focus = FocusTarget::Workspace;
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    app.build.target = Some("core-image-minimal".into());
    app.test_capability = yoctui_model::TestCapability {
        oe_selftest: yoctui_model::TestExecutableCapability::Available(
            "/workspace/oe-selftest".into(),
        ),
        bitbake_selftest: yoctui_model::TestExecutableCapability::Missing,
        ptest: yoctui_model::PtestCapability::Configured,
    };

    for (width, height, theme, color) in [
        (80, 24, Theme::Monochrome, false),
        (100, 30, Theme::WhiteClassic, true),
        (160, 40, Theme::HighContrast, true),
    ] {
        app.theme = theme;
        app.color_enabled = color;
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Testing"), "{output}");
        assert!(output.contains("qemux86-64"), "{output}");
        assert!(output.contains("core-image-minimal"), "{output}");
        assert!(output.contains("OE selftest"), "{output}");
    }

    app.test_family_selection = yoctui_model::TestFamily::Ptest;
    app.focus = FocusTarget::Inspector;
    let output = rendered_text(&app, 160, 40);
    assert!(output.contains("Package tests"), "{output}");
    assert!(output.contains("Configured"), "{output}");
}

#[test]
fn test_workflow_results_render_inventory_drill_partial_and_terminal_states() {
    let (mut app, _baseline, candidate) = test_workflow_results_app();
    for (width, height, theme, color) in [
        (80, 24, Theme::Monochrome, false),
        (100, 30, Theme::WhiteClassic, true),
        (160, 40, Theme::HighContrast, true),
    ] {
        app.theme = theme;
        app.color_enabled = color;
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Results"), "{output}");
        assert!(output.contains("candidate"), "{output}");
        assert!(output.contains("Partial"), "{output}");
    }

    app.test_result_drilled = true;
    app.test_case_selection = Some(candidate.suites[0].cases[0].identity.clone());
    app.focus = FocusTarget::Inspector;
    let output = rendered_text(&app, 160, 40);
    assert!(output.contains("Case.test_one"), "{output}");
    assert!(output.contains("Failed"), "{output}");
    assert!(output.contains("/logs/candidate.log"), "{output}");
    assert!(output.contains("fixture limitation"), "{output}");

    let request = app.test_results.request().unwrap().clone();
    for (state, expected) in [
        (
            TestResultInventoryState::AvailableEmpty {
                request: request.clone(),
            },
            "No structured test results",
        ),
        (
            TestResultInventoryState::Failed {
                request: request.clone(),
                message: "invalid JSON".into(),
            },
            "Result import failed",
        ),
        (
            TestResultInventoryState::Cancelled {
                request: request.clone(),
            },
            "Result import cancelled",
        ),
        (
            TestResultInventoryState::TimedOut {
                request: request.clone(),
            },
            "Result import timed out",
        ),
        (
            TestResultInventoryState::Lost {
                request,
                message: "worker closed".into(),
            },
            "worker lost",
        ),
    ] {
        app.test_result_drilled = false;
        app.focus = FocusTarget::Workspace;
        app.test_results = state;
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains(expected), "{expected}: {output}");
    }
}

#[test]
fn test_workflow_comparison_renders_categories_limitations_and_outcomes() {
    let (mut app, baseline, candidate) = test_workflow_results_app();
    let request = yoctui_model::TestComparisonRequest::new(
        2,
        baseline.identity.clone(),
        candidate.identity.clone(),
    )
    .unwrap();
    let comparison = yoctui_model::TestComparison::between(&baseline, &candidate).unwrap();
    app.test_view = TestWorkspaceView::Comparison;
    app.test_comparison_selection = Some(comparison.transitions[0].identity.clone());
    app.test_comparison = TestComparisonState::Partial {
        request: request.clone(),
        comparison,
        limitations: vec!["resulttool detail unavailable".into()],
    };
    for (width, height) in [(80, 24), (100, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("regression"), "{output}");
        assert!(output.contains("resulttool detail unavailable"), "{output}");
    }
    app.focus = FocusTarget::Inspector;
    let output = rendered_text(&app, 160, 40);
    assert!(output.contains("Baseline log"), "{output}");
    assert!(output.contains("JUnit export"), "{output}");

    for (state, expected) in [
        (
            TestComparisonState::Failed {
                request: request.clone(),
                message: "nonzero".into(),
            },
            "Comparison failed",
        ),
        (
            TestComparisonState::Cancelled {
                request: request.clone(),
            },
            "Comparison cancelled",
        ),
        (
            TestComparisonState::TimedOut {
                request: request.clone(),
            },
            "Comparison timed out",
        ),
        (
            TestComparisonState::Lost {
                request,
                message: "worker closed".into(),
            },
            "worker lost",
        ),
    ] {
        app.focus = FocusTarget::Workspace;
        app.test_comparison = state;
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains(expected), "{expected}: {output}");
    }
}

#[test]
fn test_workflow_lifecycle_and_junit_outcomes_remain_visibly_distinct() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Testing;
    app.focus = FocusTarget::Workspace;
    for (index, outcome) in [
        yoctui_model::TestSessionOutcome::Succeeded,
        yoctui_model::TestSessionOutcome::Failed,
        yoctui_model::TestSessionOutcome::Cancelled,
        yoctui_model::TestSessionOutcome::TimedOut,
        yoctui_model::TestSessionOutcome::Lost,
    ]
    .into_iter()
    .enumerate()
    {
        app.test_sessions.clear();
        app.test_sessions.push_back(yoctui_model::TestSession {
            id: yoctui_model::TestSessionId(index as u64 + 1),
            background_job_id: None,
            operation: yoctui_model::TestOperation::Build {
                family: yoctui_model::TestFamily::TestImage,
                request: BuildRequest {
                    targets: vec!["core-image-minimal".into()],
                    task: Some("testimage".into()),
                    force: false,
                },
            },
            exit_code: (outcome == yoctui_model::TestSessionOutcome::Failed).then_some(3),
            result_paths: if outcome == yoctui_model::TestSessionOutcome::Succeeded {
                vec!["/results/testresults.json".into()]
            } else {
                Vec::new()
            },
            error_detail: (outcome != yoctui_model::TestSessionOutcome::Succeeded)
                .then(|| format!("{outcome:?} detail")),
            outcome: Some(outcome),
        });
        let output = rendered_text(&app, 100, 30);
        assert!(output.contains(&format!("{outcome:?}")), "{output}");
    }

    let (mut app, _baseline, candidate) = test_workflow_results_app();
    app.test_view = TestWorkspaceView::Comparison;
    app.focus = FocusTarget::Inspector;
    let request = yoctui_model::TestJunitExportRequest {
        generation: 7,
        result: candidate.identity.clone(),
        destination: "/exports/results.xml".into(),
    };
    let preview =
        yoctui_model::TestJunitExportPreview::new("/workspace/resulttool".into(), request.clone())
            .unwrap();
    let states = [
        (
            TestJunitExportState::Inspecting {
                result: candidate.identity,
                destination: request.destination.clone(),
            },
            "validating",
        ),
        (TestJunitExportState::Ready(preview), "ready"),
        (TestJunitExportState::Running(request.clone()), "running"),
        (
            TestJunitExportState::Succeeded(request.clone()),
            "succeeded",
        ),
        (
            TestJunitExportState::Failed {
                request: request.clone(),
                message: "nonzero".into(),
            },
            "failed",
        ),
        (
            TestJunitExportState::Cancelled(request.clone()),
            "cancelled",
        ),
        (TestJunitExportState::TimedOut(request.clone()), "timed out"),
        (
            TestJunitExportState::Lost {
                request,
                message: "worker closed".into(),
            },
            "lost",
        ),
    ];
    for (state, expected) in states {
        app.test_junit_export = state;
        let output = rendered_text(&app, 160, 40);
        assert!(output.contains(expected), "{expected}: {output}");
    }
}
