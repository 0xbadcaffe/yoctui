#[test]
fn animation_is_absent_from_determinate_and_terminal_rows() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.screen = Screen::Tasks;
    app.tasks.insert(
        yoctui_model::TaskId("busybox:do_compile".into()),
        yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("busybox:do_compile".into()),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: Some(42),
            ..yoctui_model::TaskInfo::default()
        },
    );
    app.completed_tasks.push_back(yoctui_model::CompletedTask {
        task: yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("base-files:do_install".into()),
            recipe: "base-files".into(),
            task: "do_install".into(),
            progress: None,
            ..yoctui_model::TaskInfo::default()
        },
        success: true,
    });
    let output = rendered_text(&app, 300, 30);
    assert!(output.contains("busybox"));
    assert!(output.contains("do_compile"));
    assert!(output.contains("42%"));
    assert!(
        output.contains("base-files") && output.contains("✓ Succeeded"),
        "{output}"
    );
    assert!(!output.contains("base-files:do_install▸"));
    assert!(!output.contains("base-files:do_install active"));
}

#[test]
fn renders_small_terminal() {
    let mut terminal = Terminal::new(TestBackend::new(62, 18)).unwrap();
    terminal.draw(|f| render(f, &App::new(1, 1))).unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|c| c.symbol() == "Y")
    );
}
#[test]
fn persistent_shell_degrades_across_supported_terminal_widths() {
    for (width, height, expected) in [
        (140, 30, "Inspector"),
        (100, 24, "Navigator"),
        (80, 24, "Dashboard"),
    ] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render(frame, &App::new(10, 1_000)))
            .unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(
            output.contains(expected),
            "{width}x{height} should show {expected}"
        );
    }
}
#[test]
fn responsive_shell_uses_semantic_content_at_every_breakpoint() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;

    let wide = rendered_text(&app, 130, 24);
    assert!(wide.contains("Navigator"));
    assert!(wide.contains("Build"));
    assert!(wide.contains("Inspector"));

    let medium = rendered_text(&app, 129, 24);
    assert!(medium.contains("Navigator"));
    assert!(medium.contains("Tasks"));
    assert!(!medium.contains("┌Inspector"));

    app.focus = FocusTarget::Workspace;
    let narrow_workspace = rendered_text(&app, 99, 24);
    assert!(narrow_workspace.contains("Panes: Navigator  [Workspace]"));
    assert!(narrow_workspace.contains("Tasks"));

    app.focus = FocusTarget::Navigator;
    let narrow_navigator = rendered_text(&app, 80, 24);
    assert!(narrow_navigator.contains("Panes: [Navigator]  Workspace"));
    assert!(narrow_navigator.contains("Dashboard"));

    let too_small = rendered_text(&app, 79, 23);
    assert!(too_small.contains("Yoctui needs at least 80x24"));
    assert!(too_small.contains("Current terminal: 79x23"));
}
#[test]
fn responsive_resize_preserves_the_selected_pane() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    let mut terminal = Terminal::new(TestBackend::new(130, 24)).unwrap();
    terminal.draw(|frame| render(frame, &app)).unwrap();

    terminal.backend_mut().resize(100, 24);
    terminal.autoresize().unwrap();
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let medium = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(medium.contains("Tasks"));

    terminal.backend_mut().resize(80, 24);
    terminal.autoresize().unwrap();
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let narrow = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(narrow.contains("[Workspace]"));
    assert_eq!(app.focus, FocusTarget::Workspace);
}

#[test]
fn ux_focus_zoom_breadcrumb_and_subfocus_survive_responsive_accessible_rendering() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    app.workspace_subfocus = yoctui_model::WorkspaceSubfocus::Secondary;
    app.task_progress_scroll = 4;
    app.logs.scroll_offset = 3;
    app.color_enabled = false;
    app.reduced_motion = true;
    let retained = (app.task_progress_scroll, app.logs.scroll_offset);
    let _ = update(&mut app, Action::TogglePaneZoom);

    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(
            output.contains("ZOOM · Tasks · Workspace/Secondary"),
            "{output}"
        );
        assert!(output.contains("Esc restore"), "{output}");
        assert_eq!((app.task_progress_scroll, app.logs.scroll_offset), retained);
    }

    let _ = update(&mut app, Action::TogglePaneZoom);
    let restored = rendered_text(&app, 160, 50);
    assert!(!restored.contains("ZOOM · Tasks"), "{restored}");
    assert!(restored.contains("Navigator"), "{restored}");
    assert!(restored.contains("Inspector"), "{restored}");
    assert_eq!(
        app.workspace_subfocus,
        yoctui_model::WorkspaceSubfocus::Secondary
    );
    assert_eq!((app.task_progress_scroll, app.logs.scroll_offset), retained);
}

#[test]
fn ux_responsive_breakpoint_matrix_preserves_pane_priority_content_and_dialog_controls() {
    const SUPPORTED: [(u16, u16); 5] = [(200, 60), (160, 50), (130, 40), (100, 30), (80, 24)];
    let mut app = literal_reference_app();
    app.daemon.pty_sessions.clear();
    app.focus = FocusTarget::Workspace;
    app.recipe_selection = 2;
    app.layer_selection = 4;

    for (screen, expected) in [
        (Screen::Dashboard, "Build"),
        (Screen::Tasks, "do_compile"),
        (Screen::Logs, "Linking bash"),
        (Screen::Recipes, "core-image-minimal"),
        (Screen::Layers, "meta-oe"),
    ] {
        app.screen = screen;
        for (width, height) in SUPPORTED {
            let output = rendered_text_at(&app, width, height, literal_now());
            assert!(
                output.contains(expected),
                "{screen:?} at {width}x{height} lost {expected}: {output}"
            );
            assert!(
                !output.contains('\u{fffd}'),
                "{screen:?} at {width}x{height} emitted a replacement character"
            );
        }
    }

    app.screen = Screen::Tasks;
    for (width, height) in [(200, 60), (160, 50), (130, 40)] {
        let output = rendered_text_at(&app, width, height, literal_now());
        for expected in ["Navigator", "Tasks:", "Inspector: Task"] {
            assert!(
                output.contains(expected),
                "wide {width}x{height} lost {expected}: {output}"
            );
        }
    }

    let medium_workspace = rendered_text_at(&app, 100, 30, literal_now());
    assert!(medium_workspace.contains("Navigator"), "{medium_workspace}");
    assert!(
        medium_workspace.contains("do_compile"),
        "{medium_workspace}"
    );
    assert!(
        !medium_workspace.contains("Inspector: Task"),
        "the collapsed Inspector must not overlap medium Workspace: {medium_workspace}"
    );
    for (focus, switcher, content) in [
        (
            FocusTarget::Navigator,
            "Panes: [Navigator]  Workspace",
            "Dashboard",
        ),
        (
            FocusTarget::Workspace,
            "Panes: Navigator  [Workspace]",
            "do_compile",
        ),
    ] {
        app.focus = focus;
        let output = rendered_text_at(&app, 80, 24, literal_now());
        assert!(output.contains(switcher), "{output}");
        assert!(output.contains(content), "{output}");
    }

    app.focus = FocusTarget::Dialog;
    app.dialogs.push_back(Dialog::BuildOptions);
    for (width, height) in SUPPORTED {
        let output = rendered_text_at(&app, width, height, literal_now());
        for expected in [
            "modal · Image build options",
            "Machine:",
            "b  Build image",
            "Esc closes",
        ] {
            assert!(
                output.contains(expected),
                "dialog at {width}x{height} clipped {expected}: {output}"
            );
        }
    }
    app.dialogs.clear();

    let retained_focus = app.focus;
    let retained_recipe = app.recipe_selection;
    let retained_layer = app.layer_selection;
    for (width, height) in SUPPORTED.into_iter().rev() {
        let _ = rendered_text_at(&app, width, height, literal_now());
    }
    assert_eq!(app.focus, retained_focus);
    assert_eq!(app.recipe_selection, retained_recipe);
    assert_eq!(app.layer_selection, retained_layer);

    let too_small = rendered_text_at(&app, 79, 23, literal_now());
    for expected in [
        "Yoctui needs at least 80x24.",
        "Current terminal: 79x23.",
        "Resize the terminal or press Q to quit.",
    ] {
        assert!(too_small.contains(expected), "{too_small}");
    }
    for forbidden in ["Navigator", "Panes:", "Inspector:", "F1 Help"] {
        assert!(
            !too_small.contains(forbidden),
            "below minimum rendered shell content {forbidden}: {too_small}"
        );
    }
}

#[test]
fn ux_responsive_m21_surfaces_keep_identity_focus_and_recovery_at_every_required_size() {
    const SIZES: [(u16, u16); 5] = [(200, 60), (160, 50), (130, 40), (100, 30), (80, 24)];

    let dashboard = concept_idle_dashboard_app();
    let mut dependencies = App::new(10, 1_000);
    dependencies.screen = Screen::Dependencies;
    dependencies.focus = FocusTarget::Workspace;
    let mut rootfs = ux_rootfs_ui_app();
    rootfs.images_view = ImagesView::RootfsPackages;
    let terminal = concept_terminal_sessions_app();
    let editor = concept_editor_menu_app();
    let mut menu = App::new(10, 1_000);
    let _ = update(&mut menu, Action::OpenApplicationMenu);
    let mut onboarding = App::new_unconfigured(10, 1_000);
    let _ = update(&mut onboarding, Action::OpenOnboarding);
    let mut settings = App::new(10, 1_000);
    settings.screen = Screen::Settings;
    settings.focus = FocusTarget::Workspace;
    settings.settings_selection = 12;

    let cases = [
        (
            "dashboard/command center",
            dashboard,
            &["Dashboard", "Build Overview", "Job History"][..],
        ),
        (
            "dependency graph",
            dependencies,
            &["Dependency", "not loaded"][..],
        ),
        (
            "rootfs",
            rootfs,
            &["Rootfs composition", "Exact composition table"][..],
        ),
        ("terminal", terminal, &["Terminal Sessions", "shell"][..]),
        (
            "editor",
            editor,
            &["Recipe editor: bash", "bash_5.2.bb"][..],
        ),
        ("menu", menu, &["Application menu", "Workspace"][..]),
        (
            "onboarding",
            onboarding,
            &["Guided workflow", "Verify environment"][..],
        ),
        ("settings", settings, &["Settings", "MetadataOnly"][..]),
    ];

    for (name, app, anchors) in cases {
        let retained = (
            app.screen,
            app.focus,
            app.settings_selection,
            app.recipe_selection,
            app.layer_selection,
            app.pty_selection,
        );
        for (width, height) in SIZES {
            let output = rendered_text_at(&app, width, height, literal_now());
            assert!(
                !output.contains('\u{fffd}'),
                "{name} {width}x{height}: {output}"
            );
            assert!(
                anchors.iter().any(|anchor| output.contains(anchor)),
                "{name} {width}x{height} lost all semantic anchors: {output}"
            );
        }
        assert_eq!(
            (
                app.screen,
                app.focus,
                app.settings_selection,
                app.recipe_selection,
                app.layer_selection,
                app.pty_selection,
            ),
            retained,
            "rendering resized {name} state"
        );
        let too_small = rendered_text_at(&app, 79, 23, literal_now());
        assert!(
            too_small.contains("Yoctui needs at least 80x24"),
            "{name}: {too_small}"
        );
        assert!(!too_small.contains('\u{fffd}'), "{name}: {too_small}");
    }
}
