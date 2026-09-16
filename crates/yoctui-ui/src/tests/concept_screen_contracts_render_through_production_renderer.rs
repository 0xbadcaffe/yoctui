//! Regression tests grouped around concept_screen_contracts_render_through_production_renderer.
use super::*;

#[test]
fn concept_screen_contracts_render_through_production_renderer() {
    let mut active = literal_reference_app();
    active.navigator_selection = 9;
    active.focus = FocusTarget::Workspace;
    let scenes = [
        (
            "idle-dashboard",
            concept_idle_dashboard_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-idle-dashboard-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-idle-dashboard-160x50.cells"
            )),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-idle-dashboard-160x50.txt"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-idle-dashboard-160x50.txt"
            )),
        ),
        (
            "active-build-tasks",
            active,
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-active-build-tasks-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-active-build-tasks-160x50.cells"
            )),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-active-build-tasks-160x50.txt"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-active-build-tasks-160x50.txt"
            )),
        ),
        (
            "failed-build-errors",
            concept_failed_errors_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-failed-build-errors-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-failed-build-errors-160x50.cells"
            )),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-failed-build-errors-160x50.txt"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-failed-build-errors-160x50.txt"
            )),
        ),
        (
            "rootfs-composition",
            concept_rootfs_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-rootfs-composition-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-rootfs-composition-160x50.cells"
            )),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-rootfs-composition-160x50.txt"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-rootfs-composition-160x50.txt"
            )),
        ),
        (
            "editor-application-menu",
            concept_editor_menu_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-editor-application-menu-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-editor-application-menu-160x50.cells"
            )),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-editor-application-menu-160x50.txt"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-editor-application-menu-160x50.txt"
            )),
        ),
        (
            "terminal-sessions",
            concept_terminal_sessions_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-terminal-sessions-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-terminal-sessions-160x50.cells"
            )),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-terminal-sessions-160x50.txt"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-terminal-sessions-160x50.txt"
            )),
        ),
    ];

    let update_goldens = std::env::var_os("YOCTUI_UPDATE_CONCEPT_GOLDENS").is_some();
    for (name, app, cell_path, cell_fixture, text_path, text_fixture) in scenes {
        let mut terminal =
            Terminal::new(TestBackend::new(TARGET_GOLDEN_WIDTH, TARGET_GOLDEN_HEIGHT)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        let actual_cells = literal_cells(&terminal);
        let actual_text = concept_text_capture(&terminal);
        if update_goldens {
            fs::write(cell_path, serialize_target_golden(&actual_cells)).unwrap();
            fs::write(text_path, actual_text).unwrap();
        } else {
            assert_target_golden(name, &parse_target_golden(cell_fixture), &actual_cells);
            assert_eq!(
                text_fixture, actual_text,
                "concept screen {name} semantic capture changed; use the explicit update script only after reviewing the UI change"
            );
        }
    }
}

#[test]
fn readme_gallery_requested_workbenches_render_through_production_renderer() {
    let scenes = [
        (
            "kernel-device-tree",
            readme_platform_app(yoctui_model::PlatformComponent::Kernel),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-kernel-device-tree-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-kernel-device-tree-160x50.cells"
            )),
            ["Kernel", "Device trees", "imx8mp-evk.dts"].as_slice(),
        ),
        (
            "uboot-device-tree",
            readme_platform_app(yoctui_model::PlatformComponent::UBoot),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-uboot-device-tree-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-uboot-device-tree-160x50.cells"
            )),
            ["U-Boot", "Device trees", "imx8mp-evk.dts"].as_slice(),
        ),
        (
            "kernel-menuconfig",
            readme_menuconfig_app(true),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-kernel-menuconfig-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-kernel-menuconfig-160x50.cells"
            )),
            [
                "menuconfig:virtual/kernel",
                "Kernel Configuration",
                "Device Drivers",
            ]
            .as_slice(),
        ),
        (
            "uboot-menuconfig",
            readme_menuconfig_app(false),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-uboot-menuconfig-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-uboot-menuconfig-160x50.cells"
            )),
            [
                "menuconfig:u-boot-fslc",
                "U-Boot 2024.01 Configuration",
                "Boot options",
            ]
            .as_slice(),
        ),
        (
            "device-tree-editor",
            readme_device_tree_editor_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-device-tree-editor-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-device-tree-editor-160x50.cells"
            )),
            [
                "Recipe editor: Kernel device tree",
                "imx8mp-evk.dts",
                "/dts-v1/",
                "compatible",
                "Device Tree",
            ]
            .as_slice(),
        ),
        (
            "device-tree-compile-options",
            readme_device_tree_compile_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-device-tree-compile-options-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-device-tree-compile-options-160x50.cells"
            )),
            [
                "Compile device tree",
                "Generate symbols (-@)",
                "Output padding (-p)",
                "4096 bytes",
                "Enter review launch",
            ]
            .as_slice(),
        ),
    ];
    let update_goldens = std::env::var_os("YOCTUI_UPDATE_README_GOLDENS").is_some();
    for (name, app, cell_path, cell_fixture, anchors) in scenes {
        assert_eq!(app.navigator_screen(), app.screen);
        let mut terminal =
            Terminal::new(TestBackend::new(TARGET_GOLDEN_WIDTH, TARGET_GOLDEN_HEIGHT)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        let actual_cells = literal_cells(&terminal);
        let actual_text = concept_text_capture(&terminal);
        for anchor in anchors {
            assert!(
                actual_text.contains(anchor),
                "{name} missing {anchor}: {actual_text}"
            );
        }
        if update_goldens {
            fs::write(cell_path, serialize_target_golden(&actual_cells)).unwrap();
        } else {
            assert_target_golden(name, &parse_target_golden(cell_fixture), &actual_cells);
        }
    }
}

#[test]
fn concept_screens_keep_navigator_identity_aligned_with_the_visible_workspace() {
    let mut active = literal_reference_app();
    active.navigator_selection = 9;
    for app in [
        concept_idle_dashboard_app(),
        active,
        concept_failed_errors_app(),
        concept_rootfs_app(),
        concept_editor_menu_app(),
        concept_terminal_sessions_app(),
    ] {
        assert_eq!(app.navigator_screen(), app.screen);
    }
}

#[test]
fn literal_reference_cell_and_style_golden() {
    let app = literal_reference_app();
    let mut terminal = Terminal::new(TestBackend::new(LITERAL_WIDTH, LITERAL_HEIGHT)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &app, literal_now()))
        .unwrap();
    let actual = literal_cells(&terminal);
    if std::env::var_os("YOCTUI_UPDATE_LITERAL_GOLDEN").is_some() {
        fs::create_dir_all(
            PathBuf::from(LITERAL_GOLDEN_PATH)
                .parent()
                .expect("golden parent"),
        )
        .unwrap();
        fs::write(LITERAL_GOLDEN_PATH, serialize_literal_golden(&actual)).unwrap();
        return;
    }
    let expected = parse_literal_golden(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/golden/literal-reference-160x48.cells"
    )));
    assert_literal_cells(&expected, &actual);
}

#[test]
fn target_design_golden_canonical_states() {
    let mut idle = literal_reference_app();
    idle.screen = Screen::Dashboard;
    idle.focus = FocusTarget::Workspace;
    idle.build.status = BuildStatus::Idle;
    idle.build.started = None;
    idle.build.completed = 0;
    idle.build.total = None;
    idle.tasks.clear();
    idle.completed_tasks.clear();
    idle.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Exited;
    idle.daemon.jobs.clear();
    idle.daemon.pty_sessions.clear();
    if let Some(telemetry) = idle.daemon.telemetry.as_mut() {
        telemetry.active_jobs = 0;
        telemetry.pty_sessions = 0;
    }

    let mut active = literal_reference_app();
    active.focus = FocusTarget::Workspace;

    let mut failed = literal_reference_app();
    failed.focus = FocusTarget::Workspace;
    failed.build.status = BuildStatus::Failed;
    failed.build.errors = 1;
    failed.build.exit_code = Some(1);
    failed.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Failed;
    let failed_task = failed
        .tasks
        .get_mut(&yoctui_model::TaskId("bash:do_compile".into()))
        .expect("literal fixture has the selected compile task");
    failed_task.state = yoctui_model::TaskState::Failed;
    failed_task.progress = None;
    failed_task.finished = Some(literal_now());
    let _ = update(
        &mut failed,
        Action::Log(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Error,
            message: "ERROR: bash:do_compile failed with exit code 1".into(),
            recipe: Some("bash_5.2.21-2".into()),
            task: Some("do_compile".into()),
            path: Some("/workspace/yocto/build/tmp/log.do_compile.85873".into()),
            timestamp: literal_now(),
            build: Some("core-image-minimal".into()),
            protected: true,
            diagnostic: None,
        }),
    );

    let mut reconnecting = literal_reference_app();
    reconnecting.focus = FocusTarget::Inspector;
    reconnecting.daemon.status = yoctui_model::ClientReplicaStatus::Synchronizing;
    reconnecting.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Connecting;

    let scenes = [
        (
            "idle-dashboard",
            idle,
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/target-idle-dashboard-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/target-idle-dashboard-160x50.cells"
            )),
            [
                "Build Status  : Idle",
                "build not started · 0%",
                "Daemon: ✓ Connected",
            ]
            .as_slice(),
        ),
        (
            "active-tasks",
            active,
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/target-active-tasks-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/target-active-tasks-160x50.cells"
            )),
            ["▶ Running", "do_compile", "72%"].as_slice(),
        ),
        (
            "failed-task",
            failed,
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/target-failed-task-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/target-failed-task-160x50.cells"
            )),
            ["✕ Failed", "do_compile", "exit code 1"].as_slice(),
        ),
        (
            "daemon-reconnecting",
            reconnecting,
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/target-daemon-reconnecting-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/target-daemon-reconnecting-160x50.cells"
            )),
            ["Daemon: … Syncing", "Daemon synchronizing", "unavailable"].as_slice(),
        ),
    ];

    let update_goldens = std::env::var_os("YOCTUI_UPDATE_TARGET_GOLDENS").is_some();
    for (name, app, path, fixture, anchors) in scenes {
        let mut terminal =
            Terminal::new(TestBackend::new(TARGET_GOLDEN_WIDTH, TARGET_GOLDEN_HEIGHT)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        let actual = literal_cells(&terminal);
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        for anchor in anchors {
            assert!(
                output.contains(anchor),
                "{name} lost anchor {anchor}: {output}"
            );
        }
        if update_goldens {
            fs::write(path, serialize_target_golden(&actual)).unwrap();
        } else {
            assert_target_golden(name, &parse_target_golden(fixture), &actual);
        }
    }
}

#[test]
fn literal_shell_uses_reference_geometry_palette_and_command_rail() {
    let app = literal_reference_app();
    let mut terminal = Terminal::new(TestBackend::new(LITERAL_WIDTH, LITERAL_HEIGHT)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &app, literal_now()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let symbol = |x, y| buffer[(x, y)].symbol();

    assert_eq!(symbol(0, 0), "┌");
    assert_eq!(symbol(159, 0), "┐");
    assert_eq!(symbol(0, 2), "┌");
    assert_eq!(symbol(25, 2), "┐");
    assert_eq!(symbol(26, 2), "┌");
    assert_eq!(symbol(114, 2), "┐");
    assert_eq!(symbol(115, 2), "┌");
    assert_eq!(symbol(159, 2), "┐");
    assert_eq!(symbol(0, 47), "└");
    assert_eq!(symbol(159, 47), "┘");

    let row = |y| {
        (0..LITERAL_WIDTH)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>()
    };
    let rail = row(46);
    for label in [
        "↑/↓ select",
        "h/l groups",
        "Tab Focus",
        "Enter open",
        "Ctrl+B prefix",
        "F1 Help",
        "F3 History",
        "F10 Menu",
        "q Quit",
        "▶ Build running · 1 active",
    ] {
        assert!(rail.contains(label), "missing {label}: {rail}");
    }
    for false_label in ["F3 Jobs", "F4 Terminal", "F9 Search"] {
        assert!(
            !rail.contains(false_label),
            "dishonest {false_label}: {rail}"
        );
    }

    let palette = ThemePalette::for_app(&app);
    assert_eq!(palette.background, Color::Rgb(4, 12, 17));
    assert_eq!(palette.selection_background, Color::Rgb(13, 57, 132));
    assert_eq!(palette.warning, Color::Rgb(226, 170, 0));
    assert_eq!(palette.progress, Color::Rgb(139, 211, 0));
    assert_eq!(palette.informational, Color::Rgb(42, 178, 218));
    assert_eq!(palette.error, Color::Rgb(244, 67, 54));
}

#[test]
fn literal_navigator_projects_typed_project_state_and_full_row_selection() {
    let app = literal_reference_app();
    let mut terminal = Terminal::new(TestBackend::new(LITERAL_WIDTH, LITERAL_HEIGHT)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &app, literal_now()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let navigator = (0..LITERAL_HEIGHT)
        .flat_map(|y| (0..26).map(move |x| buffer[(x, y)].symbol()))
        .collect::<String>();
    for expected in [
        "Layers",
        "poky",
        "meta-yocto-bsp",
        "Recipes",
        "busybox",
        "Images",
        "core-image-minimal",
        "Tasks",
        "Devtool",
        "Targets",
        "qemux86-64",
    ] {
        assert!(
            navigator.contains(expected),
            "missing {expected}: {navigator}"
        );
    }
    let palette = ThemePalette::for_app(&app);
    let selected_row = (0..LITERAL_HEIGHT).find(|y| {
        (0..26).any(|x| buffer[(x, *y)].symbol() == "p")
            && (0..26).any(|x| buffer[(x, *y)].bg == palette.selection_background)
    });
    let selected_row = selected_row.expect("typed poky row must be selected");
    assert!(
        (1..25).all(|x| buffer[(x, selected_row)].bg == palette.selection_background),
        "selection must fill the Navigator content row"
    );
    assert!(navigator.contains("L: poky"), "{navigator}");
    assert!(navigator.contains("R: 858"), "{navigator}");
}

#[test]
fn literal_cockpit_uses_reference_tiers_and_typed_history() {
    let app = literal_reference_app();
    let mut terminal = Terminal::new(TestBackend::new(LITERAL_WIDTH, LITERAL_HEIGHT)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &app, literal_now()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let row = |y| {
        (0..LITERAL_WIDTH)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>()
    };

    assert!(row(2).contains("Tasks: Build"), "{}", row(2));
    assert_eq!(buffer[(26, 18)].symbol(), "└");
    assert!(row(19).contains("Log Viewer"), "{}", row(19));
    assert_eq!(buffer[(26, 32)].symbol(), "└");
    assert!(row(33).contains("Job History"), "{}", row(33));
    assert_eq!(buffer[(26, 41)].symbol(), "└");
    assert!(row(42).contains("Resources"), "{}", row(42));
    for metric in ["CPU", "RAM", "FS"] {
        assert!(row(43).contains(metric), "{}", row(43));
    }
    assert_eq!(buffer[(26, 45)].symbol(), "└");

    assert!(row(2).contains("Inspector: Task"), "{}", row(2));
    assert_eq!(buffer[(115, 12)].symbol(), "└");
    assert!(row(13).contains("Secondary facts"), "{}", row(13));
    assert_eq!(buffer[(115, 24)].symbol(), "└");
    assert!(row(25).contains("Recent Log (tail)"), "{}", row(25));
    assert_eq!(buffer[(115, 30)].symbol(), "└");
    assert!(row(31).contains("Actions"), "{}", row(31));
    assert_eq!(buffer[(115, 39)].symbol(), "└");
    assert!(row(40).contains("System Status"), "{}", row(40));
    assert_eq!(buffer[(115, 45)].symbol(), "└");

    let screen = (0..LITERAL_HEIGHT).map(row).collect::<String>();
    for expected in [
        "do_fetch",
        "do_compile",
        "72%",
        "✓ Succeeded",
        "core-image-minimal",
        "busybox",
        "✕ Failed",
        "/workspace/yocto/build/tmp/work",
        "le.85873",
    ] {
        assert!(screen.contains(expected), "missing {expected}: {screen}");
    }
}

#[test]
fn literal_ux_exposes_theme_choice_and_immediate_preview() {
    let mut app = literal_reference_app();
    app.command_palette_open = true;
    app.command_palette_query = "theme".into();
    let palette = rendered_text(&app, LITERAL_WIDTH, LITERAL_HEIGHT);
    assert!(palette.contains("Choose theme"), "{palette}");

    app.command_palette_open = false;
    let _ = update(&mut app, Action::OpenThemePicker);
    let _ = update(&mut app, Action::SelectTheme { delta: 1 });
    assert_eq!(app.theme, Theme::WhiteClassic);
    let mut terminal = Terminal::new(TestBackend::new(LITERAL_WIDTH, LITERAL_HEIGHT)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &app, literal_now()))
        .unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Theme — applies immediately"), "{output}");
    assert!(output.contains("Esc restore"), "{output}");
    assert_eq!(
        ThemePalette::for_app(&app).background,
        Color::Rgb(248, 248, 248)
    );
}

#[test]
fn worker_count_header_uses_active_pids_without_optional_labels() {
    use yoctui_model::{TaskId, TaskInfo};

    let mut app = App::new(64, 64 * 1024);
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    let _ = update(&mut app, Action::BuildStarted);
    for (recipe, pid) in [("rust-native", 1618606), ("tar", 42)] {
        let mut task = TaskInfo::active(
            TaskId(format!("{recipe}:do_compile")),
            recipe.into(),
            "do_compile".into(),
        );
        task.pid = Some(pid);
        let _ = update(&mut app, Action::TaskStarted(task));
    }
    for (width, height) in [(160, 50), (200, 50), (100, 30), (80, 24)] {
        let text = rendered_text(&app, width, height);
        if (width, height) == (160, 50) {
            assert!(text.contains("Workers: 2"), "{text}");
        }
        assert!(!text.contains("Workers: 0"), "{text}");
    }
}

#[test]
fn worker_count_header_keeps_partial_and_lost_identity_unavailable() {
    use yoctui_model::{ClientReplicaStatus, TaskId, TaskInfo};

    let mut app = App::new(64, 64 * 1024);
    app.daemon.status = ClientReplicaStatus::Current;
    let _ = update(&mut app, Action::BuildStarted);
    let task = TaskInfo::active(
        TaskId("unknown".into()),
        "unknown".into(),
        "do_compile".into(),
    );
    let _ = update(&mut app, Action::TaskStarted(task));
    for authority in [
        ClientReplicaStatus::Current,
        ClientReplicaStatus::Stale,
        ClientReplicaStatus::Disconnected,
        ClientReplicaStatus::Synchronizing,
    ] {
        app.daemon.status = authority;
        for (width, height) in [(160, 50), (100, 30), (80, 24)] {
            let text = rendered_text(&app, width, height);
            if (width, height) == (160, 50) {
                assert!(text.contains("Workers: unavailable"), "{text}");
            }
            assert!(!text.contains("Workers: 0"), "{text}");
        }
    }
    app.daemon.status = ClientReplicaStatus::Current;
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(rendered_text(&app, 160, 50).contains("Workers: 0"));
}

#[test]
fn task_identity_unresolved_statistics_render_without_invented_recipe_rows() {
    let mut app = App::new(64, 64 * 1024);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    let _ = update(&mut app, Action::BuildStarted);
    let _ = update(
        &mut app,
        Action::TaskStats(yoctui_model::TaskStats {
            completed: 2340,
            total: 6812,
            active: 1,
            failed: 0,
        }),
    );
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("2340/6812"), "{text}");
        assert!(!text.contains("unknown:do_compile"), "{text}");
    }
    assert!(app.tasks.is_empty());
    assert!(app.completed_tasks.is_empty());
}

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
