#[test]
fn compatibility_dynamic_ui_parent_gate_uses_one_projection_across_surfaces() {
    let mut app = compatibility_ui_inspector_app();
    app.screen = Screen::Configuration;
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = 12;
    let navigator = rendered_text(&app, 180, 42);
    assert!(navigator.contains("Compatibility: Limited"), "{navigator}");
    assert!(
        navigator.contains("bitbake.getvar.environment-fallback"),
        "{navigator}"
    );

    app.command_palette_open = true;
    app.focus = FocusTarget::CommandPalette;
    app.command_palette_query = "Open Configuration".into();
    let palette = rendered_text(&app, 140, 32);
    assert!(palette.contains("Compatibility: Limited"), "{palette}");
    assert!(
        palette.contains("bitbake.getvar.environment-fallback"),
        "{palette}"
    );

    app.command_palette_open = false;
    app.focus = FocusTarget::Inspector;
    let workspace = rendered_text(&app, 180, 50);
    assert!(
        workspace.contains("Refresh effective variables") && workspace.contains("[r] — Limited"),
        "{workspace}"
    );
    assert!(
        workspace.contains("bitbake.getvar.environment-fallback"),
        "{workspace}"
    );

    app.dialogs.push_front(Dialog::BuildOptions);
    app.focus = FocusTarget::Dialog;
    let dialog = rendered_text(&app, 160, 36);
    assert!(
        dialog.contains("State: Available · Confirmation available"),
        "{dialog}"
    );
    assert!(dialog.contains("bitbake.build.command"), "{dialog}");

    yoctui_model::invalidate_workspace_compatibility(&mut app);
    app.dialogs.push_front(Dialog::BuildOptions);
    app.focus = FocusTarget::Dialog;
    let invalidated = rendered_text(&app, 160, 36);
    assert!(
        invalidated.contains("State: Unknown · Confirmation disabled"),
        "{invalidated}"
    );
    assert!(
        invalidated.contains("No current environment capability snapshot"),
        "{invalidated}"
    );
    assert!(
        !invalidated.contains("bitbake.build.command"),
        "{invalidated}"
    );
}

#[test]
fn client_replica_status_renders_without_replacing_local_presentation() {
    let mut app = App::new(32, 8192);
    app.screen = Screen::Layers;
    app.focus = FocusTarget::Navigator;
    app.theme = Theme::MatrixGreen;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Running;
    app.daemon.instance_identity = Some("04040404".into());
    app.daemon.jobs.push(yoctui_model::ClientDaemonJobSummary {
        id: 1,
        kind: yoctui_model::ClientDaemonJobKind::BitBakeBuild,
        label: "core-image-minimal".into(),
        lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
        progress_current: None,
        progress_total: None,
        exit_code: None,
    });
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 2,
            name: "devshell".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    let output = rendered_text(&app, 160, 40);
    assert!(output.contains("Daemon: ✓ Connected"), "{output}");
    assert!(output.contains("BitBake: ✓ Running"), "{output}");
    assert!(output.contains("Layers"), "{output}");
}

#[test]
fn client_runtime_daemon_health_remains_visible_during_navigation() {
    let mut app = App::new(32, 8192);
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Connecting;
    for screen in [Screen::Dashboard, Screen::Tasks, Screen::Recipes] {
        app.screen = screen;
        let output = rendered_text(&app, 160, 40);
        assert!(
            output.contains("Daemon: ✓ Connected"),
            "{screen:?}: {output}"
        );
        assert!(
            output.contains("BitBake: … Connecting"),
            "{screen:?}: {output}"
        );
    }
}

#[test]
fn workbench_shell_keeps_daemon_health_compact() {
    let mut app = App::new(32, 8192);
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.telemetry = Some(yoctui_model::ClientDaemonTelemetry {
        uptime_seconds: 17,
        active_jobs: 2,
        pty_sessions: 1,
        queue_depth: 3,
        pressure: yoctui_model::ClientDaemonPressureCounters::default(),
        memory_bytes: Some(8 * 1024 * 1024),
        recovery: yoctui_model::DaemonRecoveryState::Recovered,
    });
    let output = rendered_text(&app, 160, 40);
    assert!(output.contains("Daemon: ✓ Connected"), "{output}");
    assert!(output.contains("BitBake: – Disconnected"), "{output}");
    assert!(!output.contains("Telemetry --"), "{output}");
}

#[test]
fn workbench_shell_renders_project_context_and_reference_command_rail() {
    let mut app = App::new(32, 8192);
    app.workspace.source_dir = Some("/work/poky".into());
    app.build.target = Some("core-image-minimal".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Running;

    let output = rendered_text(&app, 180, 36);
    for expected in [
        "yoctui",
        "Project: poky",
        "– Idle",
        "Target: core-image-minimal",
        "Machine: qemux86-64",
        "Distro: poky",
        "Daemon: ✓ Connected",
        "BitBake: ✓ Running",
        "F1 Help",
        "F2 Tasks",
        "F10 Menu",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
    assert!(!output.contains("Telemetry --"), "{output}");
}

#[test]
fn next_generation_header_projects_authoritative_context_by_width() {
    let mut app = App::new(32, 8192);
    app.workspace.source_dir = Some("/work/poky".into());
    app.workspace.release = Some("scarthgap".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    app.build.target = Some("core-image-minimal".into());
    app.build.status = BuildStatus::Running;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Running;

    let render = |width| {
        let mut terminal = Terminal::new(TestBackend::new(width, 2)).unwrap();
        terminal
            .draw(|frame| workbench_header(frame, &app, frame.area(), literal_now()))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };

    let full = render(180);
    let versioned_brand = concat!("yoctui v", env!("CARGO_PKG_VERSION"));
    assert!(
        full.contains(versioned_brand),
        "missing {versioned_brand}: {full}"
    );
    for expected in [
        "Project: poky",
        "▶ Running",
        "Target: core-image-minimal",
        "Machine: qemux86-64",
        "Distro: poky (scarthgap)",
        "Daemon: ✓ Connected",
        "BitBake: ✓ Running",
    ] {
        assert!(full.contains(expected), "missing {expected}: {full}");
    }

    let wide = render(160);
    assert!(
        wide.contains(versioned_brand),
        "missing {versioned_brand}: {wide}"
    );
    for expected in [
        "Project: poky",
        "▶ Running",
        "Target: core-image-minimal",
        "Machine: qemux86-64",
        "Daemon: ✓ Connected",
        "BitBake: ✓ Running",
    ] {
        assert!(wide.contains(expected), "missing {expected}: {wide}");
    }
    assert!(!wide.contains("Distro:"), "{wide}");
    assert!(!wide.contains("scarthgap"), "{wide}");

    let compact_wide = render(130);
    assert!(
        compact_wide.contains(versioned_brand),
        "missing {versioned_brand}: {compact_wide}"
    );
    for expected in [
        "Project: poky",
        "T:core-image-minimal",
        "Machine: qemux86-64",
        "D:✓ Connected",
        "BB:✓ Running",
    ] {
        assert!(
            compact_wide.contains(expected),
            "missing {expected}: {compact_wide}"
        );
    }

    let medium = render(110);
    assert!(
        medium.contains(versioned_brand),
        "missing {versioned_brand}: {medium}"
    );
    for expected in [
        "Project: poky",
        "▶ Running",
        "T:core-image-minimal",
        "D:✓ Connected",
        "BB:✓ Running",
    ] {
        assert!(medium.contains(expected), "missing {expected}: {medium}");
    }
    assert!(!medium.contains("Machine:"), "{medium}");
    assert!(!medium.contains("Distro:"), "{medium}");

    let narrow = render(90);
    assert!(
        narrow.contains(versioned_brand),
        "missing {versioned_brand}: {narrow}"
    );
    for expected in [
        "yoctui",
        "▶ Running",
        "T:core-image-minimal",
        "D:✓ Connected",
    ] {
        assert!(narrow.contains(expected), "missing {expected}: {narrow}");
    }
    for omitted in ["Project:", "Machine:", "Distro:", "BB:"] {
        assert!(!narrow.contains(omitted), "unexpected {omitted}: {narrow}");
    }
}

#[test]
fn next_generation_header_handles_missing_stale_and_accessible_states() {
    assert_eq!(header_mode(179), HeaderMode::Wide);
    assert_eq!(header_mode(180), HeaderMode::Full);
    assert_eq!(header_mode(129), HeaderMode::Medium);
    assert_eq!(header_mode(130), HeaderMode::Wide);
    assert_eq!(header_mode(99), HeaderMode::Narrow);
    assert_eq!(header_mode(100), HeaderMode::Medium);

    let mut app = App::new(32, 8192);
    app.daemon.status = yoctui_model::ClientReplicaStatus::Stale;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Running;
    app.theme = Theme::HighContrast;
    app.color_enabled = false;
    app.reduced_motion = true;

    let render = |width| {
        let mut terminal = Terminal::new(TestBackend::new(width, 2)).unwrap();
        terminal
            .draw(|frame| workbench_header(frame, &app, frame.area(), literal_now()))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };
    let wide = render(160);
    assert!(wide.contains("Project: unavailable"), "{wide}");
    assert!(wide.contains("Target: not selected"), "{wide}");
    assert!(wide.contains("Daemon: ! Stale"), "{wide}");
    assert!(wide.contains("BitBake: – Unavailable"), "{wide}");
    assert!(!wide.contains("BitBake: ✓ Running"), "{wide}");
    assert!(!wide.contains("Machine:"), "{wide}");
    assert!(!wide.contains("Distro:"), "{wide}");

    let minimum = render(80);
    assert!(minimum.contains("yoctui"), "{minimum}");
    assert!(minimum.contains("– Idle"), "{minimum}");
    assert!(minimum.contains("T:not selected"), "{minimum}");
    assert!(minimum.contains("D:! Stale"), "{minimum}");
}

#[test]
fn next_generation_footer_is_contextual_bounded_and_keymap_truthful() {
    let dashboard = App::new(32, 8192);
    for width in [130_u16, 160, 180, 200] {
        let rail = footer_rail_shortcuts(&dashboard, width.saturating_sub(10));
        assert!(rail.contains("↑/↓ select"), "{width}: {rail}");
        assert!(rail.contains("Enter open"), "{width}: {rail}");
        assert!(rail.contains("Ctrl+B prefix"), "{width}: {rail}");
        assert!(rail.contains("F1 Help"), "{width}: {rail}");
        assert!(rail.contains("F10 Menu"), "{width}: {rail}");
        assert!(rail.contains("q Quit"), "{width}: {rail}");
        assert!(footer_item_width(&rail) <= usize::from(width - 10));
    }

    let mut tasks = App::new(32, 8192);
    tasks.screen = Screen::Tasks;
    tasks.focus = FocusTarget::Workspace;
    let task_rail = footer_rail_shortcuts(&tasks, 148);
    for label in [
        "↑/↓ select",
        "f state",
        "F field",
        "/ edit filter",
        "c cancel",
        "Tab Focus",
        "F1 Help",
        "F10 Menu",
        "q Quit",
    ] {
        assert!(task_rail.contains(label), "missing {label}: {task_rail}");
    }
    assert!(!task_rail.contains("F2 Tasks"), "{task_rail}");
    for false_label in ["F3 Jobs", "F4 Terminal", "F9 Search"] {
        assert!(!task_rail.contains(false_label), "{task_rail}");
    }

    let compact = footer_rail_shortcuts(&dashboard, 80);
    assert!(compact.contains("h/l groups"), "{compact}");
    assert!(compact.contains("? Help"), "{compact}");
    assert!(compact.contains("Ctrl+P Menu"), "{compact}");
    assert!(compact.contains("q Quit"), "{compact}");
    assert!(footer_item_width(&compact) <= 80, "{compact}");
}

#[test]
fn next_generation_footer_projects_modal_and_accessible_states() {
    let mut app = App::new(32, 8192);
    app.command_palette_open = true;
    app.focus = FocusTarget::CommandPalette;
    let palette = footer_rail_shortcuts(&app, 100);
    for label in ["Type search", "↑/↓ select", "Enter run", "Esc close"] {
        assert!(palette.contains(label), "missing {label}: {palette}");
    }

    app.command_palette_open = false;
    app.focus = FocusTarget::Dialog;
    app.dialogs.push_front(Dialog::QuitConfirmation);
    let dialog = footer_rail_shortcuts(&app, 100);
    assert!(dialog.contains("Enter select/confirm"), "{dialog}");
    assert!(dialog.contains("Esc cancel"), "{dialog}");

    for (theme, color) in [
        (Theme::HighContrast, true),
        (Theme::Monochrome, true),
        (Theme::DarkPro, false),
    ] {
        app.dialogs.clear();
        app.focus = FocusTarget::Workspace;
        app.theme = theme;
        app.color_enabled = color;
        app.reduced_motion = true;
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("? Help"), "{output}");
        assert!(output.contains("q Quit"), "{output}");
    }

    app.screen = Screen::Help;
    let help = rendered_text(&app, 200, 30);
    for shortcut in FUNCTION_SHORTCUTS {
        let label = format!("{} {}", shortcut.key_label, shortcut.action_label);
        assert!(help.contains(&label), "missing {label}: {help}");
    }
}
