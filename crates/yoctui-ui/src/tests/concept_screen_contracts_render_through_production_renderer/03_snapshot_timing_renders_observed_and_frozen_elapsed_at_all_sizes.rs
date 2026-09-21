#[test]
fn snapshot_timing_renders_observed_and_frozen_elapsed_at_all_sizes() {
    use yoctui_protocol::daemon::DaemonBuildEvent as B;
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        1,
        "boot".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = yoctui_app::daemon_protocol_snapshot(&state);
    snapshot.build_events = vec![
        B::Reset {
            targets: vec!["image".into()],
        },
        B::Started {
            started_unix_ms: Some(1000),
        },
        B::TaskStarted {
            recipe: "llvm-native".into(),
            task: "do_compile".into(),
            started_unix_ms: Some(2000),
            pid: Some(42),
            worker: None,
            log_path: None,
            stats: None,
        },
    ];
    let mut app = App::new(64, 64 * 1024);
    app.screen = Screen::Tasks;
    app.focus = yoctui_model::FocusTarget::Workspace;
    let mut replica = yoctui_app::DaemonClientSnapshot::default();
    replica.replace_app(&mut app, snapshot.clone());
    assert_eq!(
        app.tasks[&yoctui_model::TaskId("llvm-native:do_compile".into())]
            .elapsed_at(UNIX_EPOCH + Duration::from_secs(3602)),
        Some(Duration::from_secs(3600))
    );
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let text = rendered_text_at(&app, width, height, UNIX_EPOCH + Duration::from_secs(3602));
        assert!(text.contains("01:00:01"), "{width}x{height}: {text}");
    }
    snapshot.build_events.push(B::Completed {
        success: true,
        exit_code: Some(0),
        finished_unix_ms: Some(65000),
    });
    replica.replace_app(&mut app, snapshot.clone());
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        for now in [3602, 9999] {
            let text = rendered_text_at(&app, width, height, UNIX_EPOCH + Duration::from_secs(now));
            assert!(text.contains("00:01:04"), "{width}x{height}: {text}");
            if width == 160 {
                assert!(text.contains("Elapsed: 00:01:04"), "{text}");
            }
        }
    }
    snapshot.build_events[1] = B::Started {
        started_unix_ms: None,
    };
    replica.replace_app(&mut app, snapshot);
    assert_eq!(app.build_summary_at(UNIX_EPOCH).elapsed, None);
    let text = rendered_text_at(&app, 160, 50, UNIX_EPOCH + Duration::from_secs(9999));
    // Earlier build-history rows may retain their known duration; the
    // current snapshot's header must not borrow it for missing timing.
    assert!(text.contains("Elapsed: --:--:--"), "{text}");
    assert!(!text.contains("Elapsed: 00:01:04"), "{text}");
}

#[test]
fn snapshot_progress_renders_aggregate_instead_of_retained_row_count() {
    use yoctui_protocol::daemon::{DaemonBuildEvent, DaemonBuildProgress};
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = yoctui_app::daemon_protocol_snapshot(&state);
    snapshot.build_events = vec![
        DaemonBuildEvent::Reset {
            targets: vec!["obmc-phosphor-image".into()],
        },
        DaemonBuildEvent::Started {
            started_unix_ms: None,
        },
        DaemonBuildEvent::TaskCompleted {
            recipe: "util-linux".into(),
            task: "do_compile".into(),
            success: true,
            started_unix_ms: None,
            finished_unix_ms: None,
        },
    ];
    snapshot.build_progress = Some(DaemonBuildProgress {
        completed: 2_340,
        total: Some(6_812),
        ..Default::default()
    });
    let mut app = App::new(64, 64 * 1024);
    app.screen = Screen::Tasks;
    app.focus = yoctui_model::FocusTarget::Workspace;
    let mut replica = yoctui_app::DaemonClientSnapshot::default();
    replica.replace_app(&mut app, snapshot.clone());
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("2340/6812"), "{width}x{height}: {text}");
    }
    snapshot.build_progress.as_mut().unwrap().total = None;
    replica.replace_app(&mut app, snapshot);
    let text = rendered_text(&app, 160, 50);
    assert!(text.contains("2340/—"), "{text}");
    assert!(text.contains("progress unknown"), "{text}");
}

#[test]
fn readme_repaired_workflows_render_through_production_renderer() {
    let scenes = [
        (
            "cloning",
            "Cloning…",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-cloning-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-cloning-160x50.cells"
            )),
        ),
        (
            "cancelling",
            "Cancelling…",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-cancelling-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-cancelling-160x50.cells"
            )),
        ),
        (
            "search-empty",
            "Type a regular expression",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-search-empty-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-search-empty-160x50.cells"
            )),
        ),
        (
            "gitui-diff",
            "SUMMARY",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-gitui-diff-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-gitui-diff-160x50.cells"
            )),
        ),
        (
            "gitui-commit",
            "Update example recipe",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-gitui-commit-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-gitui-commit-160x50.cells"
            )),
        ),
    ];
    let update_goldens = std::env::var_os("YOCTUI_UPDATE_README_GOLDENS").is_some();
    for (name, anchor, path, fixture) in scenes {
        let app = readme_repaired_workflow_app(name);
        let mut terminal = Terminal::new(TestBackend::new(160, 50)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        let text = concept_text_capture(&terminal);
        assert!(text.contains(anchor), "{name} missing {anchor}: {text}");
        let cells = literal_cells(&terminal);
        if update_goldens {
            fs::write(path, serialize_target_golden(&cells)).unwrap();
        } else {
            assert_target_golden(name, &parse_target_golden(fixture), &cells);
        }
    }
}

#[test]
fn readme_offline_archive_screens() {
    let scenes = [
        (
            "offline-dashboard",
            "Saved build · F3 details",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-offline-dashboard-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-offline-dashboard-160x50.cells"
            )),
        ),
        (
            "saved-build-history",
            "Target · Enter details",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-saved-build-history-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-saved-build-history-160x50.cells"
            )),
        ),
        (
            "saved-build-logs",
            "compiler reported a missing header",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-saved-build-logs-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-saved-build-logs-160x50.cells"
            )),
        ),
    ];
    for (name, anchor, path, fixture) in scenes {
        let app = readme_offline_history_app(name);
        let mut terminal = Terminal::new(TestBackend::new(160, 50)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        let text = concept_text_capture(&terminal);
        assert!(text.contains(anchor), "{text}");
        if name != "offline-dashboard" {
            assert!(text.contains("Saved build · read-only"), "{text}");
        }
        let cells = literal_cells(&terminal);
        if std::env::var_os("YOCTUI_UPDATE_README_GOLDENS").is_some() {
            fs::write(path, serialize_target_golden(&cells)).unwrap();
        } else {
            assert_target_golden(name, &parse_target_golden(fixture), &cells);
        }
    }
}
