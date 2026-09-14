//! Regression tests grouped around log_workspace_exposes_search_filters_pressure_and_narrow_wrap_safely.
use super::*;

#[test]
fn log_workspace_exposes_search_filters_pressure_and_narrow_wrap_safely() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Logs;
    app.logs.wrap = true;
    app.logs.searching = true;
    app.logs.query = "needle".into();
    app.logs.recipe_filter = Some("busybox".into());
    app.logs.task_filter = Some("do_compile".into());
    app.logs.build_filter = Some("core-image-minimal".into());
    app.logs.coalesced = 7;
    app.logs.dropped = 3;
    app.logs.dropped_warnings = 0;
    app.logs.dropped_errors = 0;
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Info,
        message: "needle in a long wrapped line".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: false,
        diagnostic: None,
    });
    let output = rendered_text(&app, 220, 24);
    assert!(output.contains("7 coalesced"), "{output}");
    assert!(
        output.contains("[EDITING] Query: needle▏ · 1/1"),
        "{output}"
    );
    assert!(output.contains("B:core-image-minimal"), "{output}");
    let _ = rendered_text(&app, 50, 16);
}

#[test]
fn next_generation_log_viewer_exposes_context_positions_hits_and_real_actions() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Logs;
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Warning,
        message: "first needle warning".into(),
        recipe: Some("busybox".into()),
        task: Some("do_patch".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: true,
        diagnostic: None,
    });
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "xx compile NEEDLE context".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: Some("/tmp/log.do_compile".into()),
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: true,
        diagnostic: None,
    });
    app.logs.query = "needle".into();
    app.logs.follow = false;
    app.logs.paused_len = Some(app.logs.entries.len());
    app.logs.selection = 1;
    app.logs.horizontal_offset = 2;

    let output = rendered_text(&app, 180, 30);
    for expected in [
        "Log Viewer — busybox:do_compile · paused · V 2/2 · H 2/",
        "Ⅱ Paused",
        "! Warning",
        "✕ Error",
        "C Copy",
        "o Open source log",
        "[FILTERED] Query: needle · 2/2",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }

    let spans = log_search_spans(&app, "prefix NeEdLe suffix");
    assert_eq!(
        spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>(),
        "prefix NeEdLe suffix"
    );
    assert_eq!(spans.len(), 3);
    assert_eq!(
        spans[1].style.fg,
        Some(ThemePalette::for_app(&app).accent),
        "the exact normalized search hit uses the semantic accent role"
    );
    app.color_enabled = false;
    let spans = log_search_spans(&app, "prefix needle suffix");
    assert!(
        spans[1]
            .style
            .add_modifier
            .contains(Modifier::BOLD | Modifier::UNDERLINED)
    );

    app.logs.entries.back_mut().unwrap().path = None;
    let no_source = rendered_text(&app, 180, 30);
    assert!(!no_source.contains("o Open source log"), "{no_source}");
    assert!(no_source.contains("C Copy"), "{no_source}");

    let mut empty = App::new(20, 4_000);
    empty.screen = Screen::Logs;
    let output = rendered_text(&empty, 100, 24);
    assert!(output.contains("No retained log entries."), "{output}");
    empty.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Info,
        message: "ordinary output".into(),
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: None,
        protected: false,
        diagnostic: None,
    });
    empty.logs.query = "absent".into();
    let filtered_empty = rendered_text(&empty, 100, 24);
    assert!(
        filtered_empty.contains("No log entries match the active filters or search."),
        "{filtered_empty}"
    );
}

#[test]
fn next_generation_log_activity_is_compact_complete_and_embedded() {
    let mut app = App::new(20, 4_000);
    assert_eq!(compact_log_activity(&app, 100), "▶ Following");

    app.logs.follow = false;
    app.logs.paused_len = Some(0);
    app.logs.filter = Some(Severity::Warning);
    app.logs.recipe_filter = Some("busybox".into());
    app.logs.query = "compile".into();
    app.logs.searching = true;
    app.logs.dropped = 4;
    app.logs.dropped_warnings = 1;
    app.logs.dropped_errors = 2;
    app.logs.coalesced = 7;

    let wide = compact_log_activity(&app, 100);
    for expected in [
        "Ⅱ Paused",
        "◆ Filtered",
        "/ Search compile",
        "! Evicted 4 [W 1 E 2]",
        "↺ 7 coalesced",
    ] {
        assert!(wide.contains(expected), "missing {expected}: {wide}");
    }
    let compact = compact_log_activity(&app, 40);
    for expected in ["Ⅱ Paused", "◆ Filtered", "/ Search", "! Evicted 4"] {
        assert!(compact.contains(expected), "missing {expected}: {compact}");
    }
    assert!(!compact.contains("[W 1 E 2]"), "{compact}");
    assert!(!compact.contains("compile"), "{compact}");
    assert!(!compact.contains("coalesced"), "{compact}");

    app.screen = Screen::Tasks;
    let task = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    let row = TaskRowRef::Task {
        task: &task,
        state: TaskState::Active,
    };
    let mut terminal = Terminal::new(TestBackend::new(100, 8)).unwrap();
    terminal
        .draw(|frame| render_task_log(frame, &app, frame.area(), Some(&row)))
        .unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        output.contains("Log Viewer — do_compile (busybox)"),
        "{output}"
    );
    assert!(output.contains("Ⅱ Paused"), "{output}");
    assert!(output.contains("◆ Filtered"), "{output}");
}

#[test]
fn ux_logs_workspace_renders_virtualized_bookmarks_filter_chips_and_bounded_actions() {
    let mut app = App::new(1_000, 500_000);
    app.screen = Screen::Logs;
    for index in 0..300 {
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: if index == 299 {
                Severity::Warning
            } else {
                Severity::Info
            },
            message: format!("bounded-log-{index:03}-日志"),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: Some("/logs/build.log".into()),
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(index),
            build: Some("core-image-minimal".into()),
            protected: index == 299,
            diagnostic: None,
        });
    }
    app.logs.follow = false;
    app.logs.paused_len = Some(app.logs.entries.len());
    app.logs.selection = 299;
    app.logs.source_filter = Some("/logs/build.log".into());
    app.logs.time_range = yoctui_model::LogTimeRange::LastFiveMinutes;
    assert!(app.logs.toggle_selected_bookmark());

    for (width, height, wrap, color) in [
        (160, 40, false, true),
        (100, 30, true, true),
        (80, 24, false, false),
    ] {
        app.logs.wrap = wrap;
        app.color_enabled = color;
        let output = rendered_text(&app, width, height);
        assert!(output.contains("bounded-log-299"), "{output}");
        assert!(output.contains("★"), "{output}");
        if width >= 120 {
            assert!(output.contains("/logs/build.log"), "{output}");
        } else {
            assert!(output.contains("S:on"), "{output}");
        }
        assert!(output.contains("I:5m"), "{output}");
        assert!(output.contains("E Export"), "{output}");
        assert!(output.contains("m Remove"), "{output}");
        assert!(!output.contains('\u{fffd}'), "{output}");
    }

    let window = app.logs.window(5);
    assert_eq!(window.entries.len(), 5);
    assert_eq!(
        window.entries.last().unwrap().message,
        "bounded-log-299-日志"
    );
}

#[test]
fn ux_internal_log_view_is_separate_bounded_responsive_and_nonvisual() {
    let mut app = App::new(512, 256 * 1024);
    app.screen = Screen::Logs;
    app.log_workspace_view = LogWorkspaceView::Yoctui;
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "BITBAKE-DOMAIN-ONLY".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: None,
        protected: true,
        diagnostic: None,
    });
    for index in 0..2_000 {
        app.internal_logs.insert(yoctui_model::InternalLogRecord {
            id: 0,
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(index),
            level: if index == 1_999 {
                InternalLogLevel::Error
            } else {
                InternalLogLevel::Debug
            },
            target: if index % 2 == 0 {
                "yoctui::runtime".into()
            } else {
                "yoctui::adapter".into()
            },
            message: format!("d{index:04}-self-diagnostic-日志"),
        });
    }
    app.internal_logs.follow = false;
    app.internal_logs.paused_len = Some(app.internal_logs.entries.len());
    app.internal_logs.selection = app.internal_logs.visible_count().saturating_sub(1);
    app.internal_logs.ingress_dropped = 3;

    for (width, height, color) in [(160, 40, true), (100, 30, true), (80, 24, false)] {
        app.color_enabled = color;
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Yoctui diagnostics"), "{output}");
        assert!(output.contains("local tracing"), "{output}");
        assert!(output.contains("d1999"), "{output}");
        assert!(output.contains("✕ Error"), "{output}");
        assert!(output.contains("ingress dropped 3"), "{output}");
        assert!(output.contains("E Export"), "{output}");
        assert!(!output.contains("BITBAKE-DOMAIN-ONLY"), "{output}");
        assert!(!output.contains('\u{fffd}'), "{output}");
    }

    let window = app.internal_logs.window(7);
    assert_eq!(window.entries.len(), 7);
    app.internal_logs.level_filter = Some(InternalLogLevel::Warning);
    let filtered_empty = rendered_text(&app, 100, 30);
    assert!(
        filtered_empty.contains("No Yoctui diagnostics match"),
        "{filtered_empty}"
    );
}

#[test]
fn error_workspace_renders_structured_columns_inspector_and_related_entries() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Errors;
    app.build.target = Some("core-image-minimal".into());
    let mut first = yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "compile failed\nfull compiler context".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: Some("/tmp/log.do_compile".into()),
        timestamp: SystemTime::UNIX_EPOCH,
        build: None,
        protected: true,
        diagnostic: None,
    };
    first.build = app.build.target.clone();
    app.logs.insert(first);
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Warning,
        message: "busybox follow-up warning".into(),
        recipe: Some("busybox".into()),
        task: Some("do_package".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: true,
        diagnostic: None,
    });
    let output = rendered_text(&app, 220, 36);
    assert!(output.contains("Time"), "{output}");
    assert!(output.contains("Severity"), "{output}");
    assert!(output.contains("Summary"), "{output}");
    assert!(output.contains("Category: BitBake error"), "{output}");
    assert!(output.contains("full compiler context"), "{output}");
    assert!(output.contains("Suggested actions"), "{output}");
    assert!(output.contains("busybox follow-up warning"), "{output}");
    assert!(output.contains("/tmp/log.do_compile"), "{output}");
}

#[test]
fn concept_failed_build_composes_summary_filters_correlated_log_and_recovery() {
    let app = concept_failed_errors_app();
    let output = rendered_text_at(&app, 160, 50, literal_now());

    for anchor in [
        "Failed build summary",
        "Result: Failed (exit 1)",
        "Diagnostics: 1 error / 1 warning",
        "Correlated-log filters",
        "Errors (checked)",
        "Warnings (checked)",
        "Related task context (indeterminate)",
        "Errors and warnings",
        "Correlated · Paused",
        "match 3/3",
        "loss 2 W1 E1",
        "1-3/3",
        "bash:do_compile failed with exit code 1",
        "Recovery actions",
        "confirmation required",
    ] {
        assert!(output.contains(anchor), "missing {anchor:?}: {output}");
    }
}

#[test]
fn error_workspace_and_actionable_failure_completion_are_narrow_safe() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Errors;
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "backend connection lost".into(),
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: None,
        protected: true,
        diagnostic: None,
    });
    let _ = rendered_text(&app, 50, 16);
    app.build.status = yoctui_model::BuildStatus::Failed;
    app.build.errors = 1;
    app.dialogs.push_back(Dialog::BuildCompletion);
    let output = rendered_text(&app, 100, 24);
    assert!(
        output.contains("Press Enter to investigate Errors"),
        "{output}"
    );
}

#[test]
fn signature_workspace_renders_typed_records_differences_limitations_and_footer() {
    let target = yoctui_model::SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let left = yoctui_model::SignatureIdentity {
        target: target.clone(),
        hash: Some("aaa".into()),
        path: Some("/build/tmp/stamps/busybox/do_compile.sigdata.aaa".into()),
    };
    let right = yoctui_model::SignatureIdentity {
        target: target.clone(),
        hash: Some("bbb".into()),
        path: Some("/build/tmp/stamps/busybox/do_compile.sigdata.bbb".into()),
    };
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Signatures;
    app.signature_selection = Some(left.clone());
    app.signature_dump = SignatureDumpState::Partial {
        target,
        records: vec![
            yoctui_model::SignatureRecord {
                identity: left.clone(),
                base_hash: Some("base-aaa".into()),
                task_hash: Some("aaa".into()),
                variables: vec![yoctui_model::SignatureValue {
                    name: "CC".into(),
                    value: Some("gcc".into()),
                }],
                dependencies: vec!["busybox:do_configure=dep-a".into()],
            },
            yoctui_model::SignatureRecord {
                identity: right.clone(),
                base_hash: Some("base-bbb".into()),
                task_hash: Some("bbb".into()),
                variables: Vec::new(),
                dependencies: Vec::new(),
            },
        ],
        limitations: vec!["one malformed artifact was omitted".into()],
    };
    app.signature_comparison = SignatureComparisonState::Partial {
        request: yoctui_model::SignatureComparisonRequest { left, right },
        differences: vec![yoctui_model::SignatureDifference {
            category: SignatureDifferenceCategory::ChangedValue,
            key: "CC".into(),
            left: Some("gcc".into()),
            right: Some("clang".into()),
        }],
        limitations: vec!["recursive detail unavailable".into()],
    };

    let wide = rendered_text(&app, 160, 34);
    assert!(wide.contains("Signatures"), "{wide}");
    assert!(wide.contains("busybox:do_compile"), "{wide}");
    assert!(wide.contains("base-aaa"), "{wide}");
    assert!(wide.contains("CC = gcc"), "{wide}");
    assert!(wide.contains("[value] CC: gcc"), "{wide}");
    assert!(wide.contains("one malformed artifact"), "{wide}");
    assert!(wide.contains("recursive detail unavailable"), "{wide}");
    assert!(wide.contains("F10 Menu"), "{wide}");
    let contextual_footer = rendered_text(&app, 120, 34);
    assert!(
        contextual_footer.contains("1/2 sides"),
        "{contextual_footer}"
    );

    let narrow = rendered_text(&app, 90, 30);
    assert!(narrow.contains("Signatures"), "{narrow}");
    assert!(narrow.contains("Selected record"), "{narrow}");
    let tiny = rendered_text(&app, 50, 16);
    assert!(tiny.contains("needs at least 80x24"), "{tiny}");
}

#[test]
fn signature_workspace_renders_explicit_loading_empty_failure_and_picker_states() {
    let target = yoctui_model::SignatureTarget {
        recipe: "busybox".into(),
        task: "do_fetch".into(),
    };
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Signatures;
    app.signature_dump = SignatureDumpState::Loading {
        target: target.clone(),
    };
    assert!(rendered_text(&app, 100, 24).contains("Loading authoritative signature artifacts"));
    app.signature_dump = SignatureDumpState::AvailableEmpty {
        target: target.clone(),
    };
    assert!(rendered_text(&app, 100, 24).contains("no signature artifacts"));
    app.signature_dump = SignatureDumpState::Failed {
        target,
        message: "tool missing".into(),
    };
    assert!(rendered_text(&app, 100, 24).contains("tool missing"));

    app.screen = Screen::Recipes;
    app.dialogs.push_back(Dialog::SignatureTaskPicker(
        yoctui_model::SignatureTaskPicker {
            recipe: RecipeIdentity {
                name: "busybox".into(),
                file: "/layers/meta/busybox.bb".into(),
            },
            tasks: vec!["do_fetch".into(), "do_compile".into()],
            selection: 1,
        },
    ));
    let picker = rendered_text(&app, 80, 24);
    assert!(picker.contains("Inspect signatures: busybox"), "{picker}");
    assert!(picker.contains("Authoritative signature tasks"), "{picker}");
}

#[test]
fn pkgdata_workspace_renders_typed_partial_details_footer_and_responsive_modes() {
    let request = yoctui_model::PackageInventoryRequest { generation: 1 };
    let identity = PackageIdentity::new("busybox");
    let package = yoctui_model::PackageSummary {
        identity: identity.clone(),
        recipe: PackageField::Available("busybox".into()),
        provider: PackageField::Available("/layers/meta/recipes-core/busybox.bb".into()),
        version: PackageField::Available("1.37.0-r0".into()),
        installed_size_bytes: PackageField::Available(1_024),
        license: PackageField::Available("GPL-2.0-only".into()),
        image_membership: PackageField::Unavailable,
    };
    let detail_request = yoctui_model::PackageDetailRequest {
        identity: identity.clone(),
        generation: 2,
    };
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Packages;
    app.package_selection = Some(identity.clone());
    app.package_inventory = PackageInventoryState::Partial {
        request,
        packages: vec![package],
        limitations: vec!["image membership unavailable".into()],
    };
    app.package_details.insert(
        identity.clone(),
        PackageDetailState::Partial {
            request: detail_request,
            detail: yoctui_model::PackageDetail {
                identity,
                files: PackageField::Available(vec!["/bin/busybox".into()]),
                runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
                reverse_dependencies: PackageField::Available(Vec::new()),
            },
            limitations: vec!["reverse scan bounded".into()],
        },
    );

    let wide = rendered_text(&app, 160, 34);
    assert!(wide.contains("Packages"), "{wide}");
    assert!(wide.contains("busybox"), "{wide}");
    assert!(wide.contains("1.37.0-r0"), "{wide}");
    assert!(wide.contains("GPL-2.0-only"), "{wide}");
    assert!(wide.contains("/bin/busybox"), "{wide}");
    assert!(wide.contains("libc6"), "{wide}");
    assert!(wide.contains("Image membership: unavailable"), "{wide}");
    assert!(wide.contains("image membership unavailable"), "{wide}");
    assert!(wide.contains("F10 Menu"), "{wide}");
    let contextual_footer = rendered_text(&app, 120, 30);
    assert!(
        contextual_footer.contains("Enter detail"),
        "{contextual_footer}"
    );

    for (width, height) in [(120, 30), (90, 28)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Packages"), "{width}: {output}");
        assert!(output.contains("busybox"), "{width}: {output}");
    }
    for (theme, color) in [(Theme::WhiteClassic, true), (Theme::DarkPro, false)] {
        app.theme = theme;
        app.color_enabled = color;
        let output = rendered_text(&app, 140, 30);
        assert!(output.contains("busybox"), "{output}");
    }
    assert!(rendered_text(&app, 50, 16).contains("needs at least 80x24"));
}

#[test]
fn ux_scrollable_collection_matrix_keeps_the_last_highlighted_row_visible() {
    let package_request = yoctui_model::PackageInventoryRequest { generation: 1 };
    let packages = (0..40)
        .map(|index| yoctui_model::PackageSummary {
            identity: PackageIdentity::new(format!("package-{index:02}")),
            recipe: PackageField::Available(format!("recipe-{index:02}")),
            provider: PackageField::Unavailable,
            version: PackageField::Available("1.0".into()),
            installed_size_bytes: PackageField::Available(index),
            license: PackageField::Available("MIT".into()),
            image_membership: PackageField::Unavailable,
        })
        .collect::<Vec<_>>();
    let mut packages_app = App::new(10, 1_000);
    packages_app.package_selection = Some(PackageIdentity::new("package-39"));
    packages_app.package_inventory = PackageInventoryState::Available {
        request: package_request,
        packages,
    };
    let package_rows = rendered_region_rows(90, 12, |frame, area| {
        packages_workspace(frame, &packages_app, area)
    });
    assert!(
        package_rows.iter().any(|row| row.contains("package-39")),
        "{}",
        package_rows.join("\n")
    );
    assert!(!package_rows.iter().any(|row| row.contains("package-00")));

    let artifacts = (0..40)
        .map(|index| {
            let identity = yoctui_model::ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: format!("scroll-image-{index:02}"),
                path: format!("/deploy/scroll-image-{index:02}.ext4").into(),
            };
            yoctui_model::ImageArtifact {
                identity,
                kind: yoctui_model::ImageArtifactKind::RootFilesystem,
                size_bytes: ImageArtifactField::Available(4_096),
                modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
                checksums: ImageArtifactField::Available(Vec::new()),
                manifests: ImageArtifactField::Available(Vec::new()),
                licenses: ImageArtifactField::Available(Vec::new()),
                spdx: ImageArtifactField::Available(Vec::new()),
                wic_files: ImageArtifactField::Available(Vec::new()),
            }
        })
        .collect::<Vec<_>>();
    let mut images_app = App::new(10, 1_000);
    images_app.image_artifact_selection = Some(artifacts.last().unwrap().identity.clone());
    images_app.image_artifacts = ImageArtifactInventoryState::Available {
        request: yoctui_model::ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        },
        inventory: yoctui_model::ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: ImageArtifactField::Available("/deploy".into()),
            artifacts,
        },
    };
    let image_rows = rendered_region_rows(100, 14, |frame, area| {
        image_artifacts_workspace(frame, &images_app, area)
    });
    assert!(
        image_rows.iter().any(|row| row.contains("scroll-image-39")),
        "{}",
        image_rows.join("\n")
    );
    assert!(!image_rows.iter().any(|row| row.contains("scroll-image-00")));

    let mut config_app = App::new(10, 1_000);
    for index in 0..40 {
        config_app.workspace.variables.insert(
            format!("SCROLL_VARIABLE_{index:02}"),
            format!("value-{index:02}"),
        );
    }
    config_app.config_selection = 39;
    let config_rows = rendered_region_rows(100, 20, |frame, area| config(frame, &config_app, area));
    assert!(
        config_rows
            .iter()
            .take(8)
            .any(|row| row.contains("SCROLL_VARIABLE_39")),
        "{}",
        config_rows.join("\n")
    );

    let signature_rows_data = (0..40)
        .map(|index| yoctui_model::SignatureRecord {
            identity: yoctui_model::SignatureIdentity {
                target: yoctui_model::SignatureTarget {
                    recipe: "busybox".into(),
                    task: "do_compile".into(),
                },
                hash: Some(format!("scroll-signature-{index:02}")),
                path: Some(format!("/build/scroll-signature-{index:02}").into()),
            },
            base_hash: None,
            task_hash: None,
            variables: Vec::new(),
            dependencies: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut signatures_app = App::new(10, 1_000);
    signatures_app.signature_selection = Some(signature_rows_data.last().unwrap().identity.clone());
    signatures_app.signature_dump = SignatureDumpState::Available {
        target: signature_rows_data[0].identity.target.clone(),
        records: signature_rows_data,
    };
    let signature_rows = rendered_region_rows(90, 12, |frame, area| {
        signature_records(frame, &signatures_app, area)
    });
    assert!(
        signature_rows
            .iter()
            .any(|row| row.contains("scroll-signature-39")),
        "{}",
        signature_rows.join("\n")
    );

    let mut picker_app = App::new(10, 1_000);
    picker_app
        .dialogs
        .push_back(Dialog::RecipeTaskPicker(yoctui_model::RecipeTaskPicker {
            recipe: "busybox".into(),
            tasks: (0..40)
                .map(|index| format!("do_scroll_{index:02}"))
                .collect(),
            selection: 39,
            force: false,
        }));
    let picker = rendered_text(&picker_app, 100, 30);
    assert!(picker.contains("do_scroll_39"), "{picker}");
    assert!(!picker.contains("do_scroll_00"), "{picker}");

    let mut compatibility_app = compatibility_ui_inspector_app();
    let mut authority = compatibility_app
        .workspace_compatibility
        .authority()
        .expect("compatibility fixture authority")
        .clone();
    authority.snapshot.generation += 1;
    authority.snapshot.capabilities = yoctui_model::CapabilityId::ALL
        .into_iter()
        .map(|id| yoctui_model::CapabilityRecord {
            id,
            state: yoctui_model::CapabilityState::Unknown {
                reason: yoctui_model::CapabilityReason::new(
                    "test.inconclusive",
                    "Bounded viewport test evidence is intentionally inconclusive.",
                    None,
                )
                .unwrap(),
            },
            evidence: Vec::new(),
        })
        .collect();
    authority.implementations.clear();
    let authority = authority.normalize().unwrap();
    yoctui_model::install_workspace_compatibility(&mut compatibility_app, authority).unwrap();
    let authority = compatibility_app
        .workspace_compatibility
        .authority()
        .cloned();
    compatibility_app
        .compatibility_ui
        .select(isize::MAX, authority.as_ref());
    let last_capability = yoctui_model::CapabilityId::ALL
        .last()
        .expect("capability inventory")
        .as_str();
    let first_capability = yoctui_model::CapabilityId::ALL
        .first()
        .expect("capability inventory")
        .as_str();
    let compatibility_rows = rendered_region_rows(100, 24, |frame, area| {
        compatibility_workspace(frame, &compatibility_app, area)
    });
    assert!(
        compatibility_rows
            .iter()
            .any(|row| row.contains(last_capability)),
        "{}",
        compatibility_rows.join("\n")
    );
    assert!(
        !compatibility_rows
            .iter()
            .any(|row| row.contains(first_capability)),
        "{}",
        compatibility_rows.join("\n")
    );
    assert!(
        compatibility_rows.join("\n").contains(&format!(
            "{0}/{0} · ↑ · rows",
            yoctui_model::CapabilityId::ALL.len()
        )),
        "{}",
        compatibility_rows.join("\n")
    );
}

#[test]
fn pkgdata_workspace_renders_loading_empty_failed_and_unavailable_states() {
    let request = yoctui_model::PackageInventoryRequest { generation: 1 };
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Packages;
    app.package_inventory = PackageInventoryState::Loading { request };
    assert!(rendered_text(&app, 100, 25).contains("Loading authoritative package inventory"));
    app.package_inventory = PackageInventoryState::AvailableEmpty { request };
    assert!(rendered_text(&app, 100, 25).contains("No built runtime packages"));
    app.package_inventory = PackageInventoryState::Failed {
        request,
        message: "generated pkgdata is unavailable".into(),
    };
    let failed = rendered_text(&app, 100, 25);
    assert!(
        failed.contains("generated pkgdata is unavailable"),
        "{failed}"
    );
    assert!(failed.contains("do_package"), "{failed}");
}
