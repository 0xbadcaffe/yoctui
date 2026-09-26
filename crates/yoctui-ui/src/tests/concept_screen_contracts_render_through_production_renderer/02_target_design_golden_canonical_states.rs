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
                "Daemon health: ✓ Connected",
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
            ["D:… Syncing", "Daemon synchronizing", "unavailable"].as_slice(),
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
        "F12 Menu",
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
