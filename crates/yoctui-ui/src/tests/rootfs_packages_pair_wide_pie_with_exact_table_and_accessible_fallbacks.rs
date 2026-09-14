//! Regression tests grouped around ux_rootfs_packages_pair_wide_pie_with_exact_table_and_accessible_fallbacks.
use super::*;

#[test]
fn ux_rootfs_packages_pair_wide_pie_with_exact_table_and_accessible_fallbacks() {
    let mut app = ux_rootfs_ui_app();
    app.images_view = ImagesView::RootfsPackages;
    let wide = rendered_text(&app, 200, 60);
    assert!(wide.contains("Rootfs packages · installed bytes"), "{wide}");
    assert!(wide.contains("Exact bytes"), "{wide}");
    assert!(wide.contains("Other"), "{wide}");
    assert!(wide.contains("Other membership"), "{wide}");
    assert!(wide.contains("remains inspectable"), "{wide}");
    assert!(wide.contains("56320"), "{wide}");
    assert!(wide.contains("exact bytes"), "{wide}");
    assert!(
        wide.chars()
            .filter(|glyph| ('\u{2801}'..='\u{28ff}').contains(glyph))
            .count()
            > 8,
        "the wide production chart must contain tui-piechart Braille cells: {wide}"
    );

    app.theme = Theme::Monochrome;
    app.color_enabled = false;
    for (width, height) in [(100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Installed-package authority"), "{output}");
        assert!(output.contains("Exact bytes"), "{output}");
        assert!(output.contains("Other"), "{output}");
        assert!(
            !output.contains("Rootfs packages · installed bytes"),
            "{output}"
        );
    }

    app.theme = Theme::DarkPro;
    app.color_enabled = true;
    app.preferences.symbols = SymbolPreference::Ascii;
    let ascii = rendered_text(&app, 100, 30);
    assert!(ascii.contains("Installed-package authority"), "{ascii}");
    assert!(ascii.contains("Exact bytes"), "{ascii}");
    assert!(
        !ascii.contains("Rootfs packages · installed bytes"),
        "{ascii}"
    );
    assert!(
        !ascii
            .chars()
            .any(|glyph| ('\u{2800}'..='\u{28ff}').contains(&glyph)),
        "ASCII fallback must not depend on Braille chart cells: {ascii}"
    );

    app.preferences.symbols = SymbolPreference::Unicode;
    let short_wide = rendered_text(&app, 200, 42);
    assert!(
        short_wide.contains("Installed-package authority"),
        "{short_wide}"
    );
    assert!(short_wide.contains("Exact bytes"), "{short_wide}");
    assert!(
        !short_wide.contains("Rootfs packages · installed bytes"),
        "the pie layout must yield to the explorable table when all three panes do not fit: {short_wide}"
    );
}

#[test]
fn concept_rootfs_composition_keeps_chart_table_selection_and_tree_visible() {
    let app = concept_rootfs_app();
    let output = rendered_text_at(&app, 160, 50, literal_now());

    for anchor in [
        "Rootfs packages · installed bytes",
        "Exact composition table",
        "46800000",
        "Other",
        "Accessible package selection",
        "base system group · 90 packages (indeterminate)",
        "base-system-000 · 520000 B · 3 files (checked)",
        "base-system-001 · 520000 B · 4 files (unchecked)",
        "Filesystem ownership (disabled)",
        "partial; Tab for evidence",
        "Filesystem tree: partial",
        "/bin/busybox",
        "Tab drill-down",
        "package ownership is partial",
    ] {
        assert!(output.contains(anchor), "missing {anchor:?}: {output}");
    }
}

#[test]
fn concept_editor_application_menu_composes_focus_validation_and_diff() {
    let app = concept_editor_menu_app();
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(app.menu.kind, Some(yoctui_model::MenuKind::Application));
    assert!(matches!(app.active_dialog(), Some(Dialog::RecipeEditor(_))));
    let output = rendered_text_at(&app, 160, 50, literal_now());

    for anchor in [
        "Recipe Inspector",
        "Name: bash",
        "File: bash_5.2.bb",
        "Language: BitBake",
        "State: modified",
        "Application menu · focus trapped",
        "Build  Navigate  View",
        "Cancel active build",
        "No active build is avai",
        "Validation and diff state",
        "Local validation: ✕ line 9: assignment has no value",
        "Diff preview: loaded → buffer",
        "BROKEN_OVERRIDE =",
        "BitBake/compiler output remains authoritative",
        "Ctrl+S save",
        "Ctrl+B build",
        "Tab files",
    ] {
        assert!(output.contains(anchor), "missing {anchor:?}: {output}");
    }
}

#[test]
fn ux_rootfs_filesystem_tree_and_inspector_preserve_exact_separate_authority() {
    let mut app = ux_rootfs_ui_app();
    app.images_view = ImagesView::RootfsFilesystem;
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Filesystem authority"), "{output}");
        assert!(output.contains("exact bytes"), "{output}");
        assert!(output.contains("symlinks 1"), "{output}");
        assert!(output.contains("special 1"), "{output}");
        assert!(output.contains("tool"), "{output}");
    }
    app.focus = FocusTarget::Inspector;
    let inspector = rendered_text(&app, 200, 60);
    assert!(
        inspector.contains("Selected path: /usr/bin/tool"),
        "{inspector}"
    );
    assert!(inspector.contains("Package bytes: 56320"), "{inspector}");
    assert!(inspector.contains("Filesystem bytes: 4100"), "{inspector}");
    assert!(
        inspector.contains("package ownership is partial"),
        "{inspector}"
    );
}

#[test]
fn ux_rootfs_system_tabs_show_offline_service_and_bus_file_evidence() {
    let mut app = ux_rootfs_ui_app();
    let composition = match &mut app.rootfs_composition {
        RootfsCompositionState::Partial { composition, .. } => composition,
        _ => unreachable!(),
    };
    composition.system_inventory =
        yoctui_model::RootfsAuthority::Available(yoctui_model::RootfsSystemInventory {
            udev_rules: Vec::new(),
            systemd_services: vec![yoctui_model::RootfsSystemdService {
                name: "example.service".into(),
                logical_path: yoctui_model::RootfsPathIdentity(
                    "/usr/lib/systemd/system/example.service".into(),
                ),
                host_path: "/build/rootfs/usr/lib/systemd/system/example.service".into(),
                description: Some("Example daemon".into()),
                bus_name: Some("org.example.Daemon".into()),
                enabled_by: vec!["multi-user.target.wants".into()],
                preview: "[Service]\nBusName=org.example.Daemon\n".into(),
                preview_truncated: false,
            }],
            dbus_services: vec![yoctui_model::RootfsDbusService {
                name: "org.example.Daemon".into(),
                logical_path: yoctui_model::RootfsPathIdentity(
                    "/usr/share/dbus-1/system-services/org.example.Daemon.service".into(),
                ),
                host_path:
                    "/build/rootfs/usr/share/dbus-1/system-services/org.example.Daemon.service"
                        .into(),
                exec: Some("/usr/bin/example".into()),
                user: Some("root".into()),
                systemd_service: Some("example.service".into()),
                policy_files: vec![yoctui_model::RootfsPathIdentity(
                    "/usr/share/dbus-1/system.d/example.conf".into(),
                )],
                preview: "[D-BUS Service]\nName=org.example.Daemon\n".into(),
                preview_truncated: false,
            }],
        });
    app.images_view = ImagesView::SystemdServices;
    let systemd = rendered_text(&app, 160, 50);
    assert!(systemd.contains("example.service"), "{systemd}");
    assert!(systemd.contains("Example daemon"), "{systemd}");
    app.images_view = ImagesView::SystemDbus;
    let dbus = rendered_text(&app, 160, 50);
    assert!(dbus.contains("org.example.Daemon"), "{dbus}");
    assert!(dbus.contains("example.service"), "{dbus}");
}

#[test]
fn ux_rootfs_lifecycle_states_are_explicit_without_empty_data_fabrication() {
    let mut app = ux_rootfs_ui_app();
    app.images_view = ImagesView::RootfsPackages;
    let request = app.rootfs_composition.request().unwrap().clone();
    app.rootfs_composition = RootfsCompositionState::Loading {
        request: request.clone(),
    };
    assert!(rendered_text(&app, 100, 30).contains("Loading rootfs composition"));
    app.rootfs_composition = RootfsCompositionState::Unavailable {
        request: request.clone(),
        reason: "IMAGE_ROOTFS was cleaned".into(),
    };
    let unavailable = rendered_text(&app, 100, 30);
    assert!(unavailable.contains("Rootfs composition unavailable"));
    assert!(unavailable.contains("IMAGE_ROOTFS was cleaned"));
    app.rootfs_composition = RootfsCompositionState::Failed {
        request,
        message: "source correlation failed".into(),
    };
    assert!(rendered_text(&app, 100, 30).contains("source correlation failed"));
}

#[test]
fn sdk_workflow_renders_every_inventory_state_and_responsive_selection() {
    let mut app = sdk_workflow_ui_app();
    let request = yoctui_model::SdkArtifactInventoryRequest {
        generation: 2,
        root: "/deploy/sdk".into(),
        machine: "qemux86-64".into(),
    };

    app.sdk_artifacts = SdkArtifactInventoryState::NotLoaded;
    assert!(rendered_text(&app, 100, 25).contains("not loaded"));
    app.sdk_artifacts = SdkArtifactInventoryState::Loading {
        request: request.clone(),
    };
    assert!(rendered_text(&app, 100, 25).contains("generation 2"));
    app.sdk_artifacts = SdkArtifactInventoryState::AvailableEmpty {
        request: request.clone(),
    };
    assert!(rendered_text(&app, 100, 25).contains("No SDK artifacts were found"));
    app.sdk_artifacts = SdkArtifactInventoryState::Failed {
        request: request.clone(),
        message: "permission denied".into(),
    };
    assert!(rendered_text(&app, 100, 25).contains("permission denied"));

    app = sdk_workflow_ui_app();
    app.sdk_artifact_query = "missing".into();
    assert!(rendered_text(&app, 100, 25).contains("No SDK artifacts match"));
    app.sdk_artifact_query.clear();
    app.sdk_artifact_searching = true;
    let artifact = app.selected_sdk_artifact().unwrap().clone();
    app.sdk_artifacts = SdkArtifactInventoryState::Partial {
        request,
        artifacts: vec![artifact],
        limitations: vec!["one SDK symlink was not followed".into()],
    };
    app.focus = FocusTarget::Inspector;
    for (width, height, theme, color) in [
        (80, 24, Theme::Monochrome, false),
        (100, 30, Theme::WhiteClassic, true),
        (160, 40, Theme::MatrixGreen, true),
    ] {
        app.theme = theme;
        app.color_enabled = color;
        let output = rendered_text(&app, width, height);
        assert!(output.contains("SDK"), "{output}");
        if width == 160 {
            assert!(output.contains("Host tuple"), "{output}");
            assert!(output.contains("x86_64-pokysdk-linux"), "{output}");
            assert!(output.contains("Published: unavailable"), "{output}");
            assert!(
                output.contains("one SDK symlink was not followed"),
                "{output}"
            );
        }
    }
    for (theme, color) in [
        (Theme::DarkPro, true),
        (Theme::WhiteClassic, true),
        (Theme::MatrixGreen, true),
        (Theme::HighContrast, true),
        (Theme::Monochrome, true),
        (Theme::DarkPro, false),
    ] {
        app.theme = theme;
        app.color_enabled = color;
        let output = rendered_text(&app, 160, 40);
        assert!(output.contains("SDK tool capability"), "{output}");
        assert!(output.contains("Selected artifact"), "{output}");
    }
    app.focus = FocusTarget::Workspace;
    app.color_enabled = false;
    let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
    terminal.draw(|frame| render(frame, &app)).unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| cell.modifier.contains(Modifier::REVERSED))
    );
    let narrow = rendered_text(&app, 80, 24);
    assert!(narrow.contains("s/E:SDK"), "{narrow}");
    assert!(narrow.contains("c:cancel"), "{narrow}");
}

#[test]
fn sdk_workflow_renders_lifecycle_output_and_every_terminal_outcome() {
    let (mut running, id) = sdk_workflow_running_ui_app();
    let _ = update(
        &mut running,
        Action::AppendSdkSessionOutput {
            id,
            stream: yoctui_model::SdkOutputStream::Stderr,
            line: "publication warning".into(),
            truncated: true,
            timestamp: SystemTime::UNIX_EPOCH,
        },
    );
    let output = rendered_text(&running, 180, 44);
    assert!(output.contains("Status: running"), "{output}");
    assert!(
        output.contains("[stderr] publication warning [truncated]"),
        "{output}"
    );

    let mut succeeded = running.clone();
    let _ = update(
        &mut succeeded,
        Action::CompleteSdkSession {
            id,
            exit_code: 0,
            artifacts: vec!["/srv/sdk-publish/toolchain.sh".into()],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(rendered_text(&succeeded, 180, 44).contains("Status: succeeded"));

    let (mut failed, failed_id) = sdk_workflow_running_ui_app();
    let _ = update(
        &mut failed,
        Action::FailSdkSession {
            id: failed_id,
            message: "destination denied".into(),
            exit_code: Some(7),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let output = rendered_text(&failed, 180, 44);
    assert!(output.contains("Status: failed"), "{output}");
    assert!(output.contains("destination denied"), "{output}");

    let (mut lost, lost_id) = sdk_workflow_running_ui_app();
    let _ = update(
        &mut lost,
        Action::LoseSdkSession {
            id: lost_id,
            message: "runner channel lost".into(),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(rendered_text(&lost, 180, 44).contains("Status: lost"));

    let (mut cancelled, cancelled_id) = sdk_workflow_running_ui_app();
    let _ = update(&mut cancelled, Action::BeginActiveSdkSessionCancellation);
    let _ = update(&mut cancelled, Action::ConfirmSdkSessionCancellation);
    let _ = update(
        &mut cancelled,
        Action::CancelSdkSession {
            id: cancelled_id,
            exit_code: Some(130),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(rendered_text(&cancelled, 180, 44).contains("Status: cancelled"));
}

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
