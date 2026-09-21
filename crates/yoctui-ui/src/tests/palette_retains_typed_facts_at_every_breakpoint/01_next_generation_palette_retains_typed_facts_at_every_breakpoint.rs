use super::*;

#[test]
fn next_generation_palette_retains_typed_facts_at_every_breakpoint() {
    for (width, height, has_columns) in [(160, 50, true), (100, 30, true), (80, 24, false)] {
        let mut app = App::new(32, 8192);
        app.command_palette_open = true;
        app.focus = FocusTarget::CommandPalette;
        app.command_palette_query = "Open Dashboard".into();
        let output = rendered_text(&app, width, height);
        for expected in [
            "Command Palette · focus trapped",
            "[EDITING]",
            "Commands · 1 match",
            "Open Dashboard",
            "Esc",
            "✓ Ready",
            "Selected command",
            "Show build status",
            "Available: yes",
            "Esc close",
        ] {
            assert!(output.contains(expected), "{width}x{height}: {output}");
        }
        assert_eq!(output.matches('▶').count(), 1, "{width}x{height}: {output}");
        assert_eq!(output.contains("Shortcut"), has_columns, "{output}");
        assert_eq!(output.contains("Availability"), has_columns, "{output}");
    }
}

#[test]
fn global_search_loading_uses_the_shared_braille_activity_phase() {
    let mut app = App::new(32, 8192);
    let _ = yoctui_model::update(&mut app, Action::OpenGlobalSearch);
    let _ = yoctui_model::update(&mut app, Action::AppendCommandPaletteQuery('x'));
    let _ = yoctui_model::update(&mut app, Action::BeginGlobalContentSearch);
    app.animation_frame = 3;
    let activity = startup_activity_symbol(app.animation_frame as usize);
    let output = rendered_text(&app, 100, 30);
    assert!(
        output.contains(&format!("{activity} Searching build text files")),
        "{output}"
    );
}

#[test]
fn next_generation_palette_explains_local_disablement_without_false_ready_state() {
    let mut app = App::new(32, 8192);
    app.command_palette_open = true;
    app.focus = FocusTarget::CommandPalette;
    app.command_palette_query = "Build image".into();
    let output = rendered_text(&app, 160, 40);
    assert!(output.contains("– Unavailable"), "{output}");
    assert!(output.contains("Available: no"), "{output}");
    assert!(
        output.contains("Cannot run: Load a Yocto workspace first"),
        "{output}"
    );
    assert!(!output.contains("✓ Ready"), "{output}");
}

#[test]
fn next_generation_palette_bounds_scroll_and_clears_stale_empty_detail() {
    let mut app = App::new(32, 8192);
    app.command_palette_open = true;
    app.focus = FocusTarget::CommandPalette;
    let commands = app.command_palette_commands();
    app.command_palette_selection = commands
        .iter()
        .position(|command| command.action_id.as_str() == "help.open")
        .unwrap();
    let output = rendered_text(&app, 80, 24);
    assert!(
        output.contains(&format!("Commands · {0}/{0}", commands.len())),
        "{output}"
    );
    assert!(output.contains("Open Help"), "{output}");

    app.command_palette_query = "nothing matches this".repeat(20);
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("Commands · 0 matches"), "{output}");
    assert!(
        output.contains("No commands match this search."),
        "{output}"
    );
    assert!(output.contains("No command selected."), "{output}");
    assert!(!output.contains("Show all global"), "{output}");
}

#[test]
fn next_generation_palette_is_explicit_in_accessible_modes() {
    for (theme, color_enabled) in [
        (Theme::HighContrast, true),
        (Theme::Monochrome, true),
        (Theme::DarkPro, false),
    ] {
        let mut app = App::new(32, 8192);
        app.theme = theme;
        app.color_enabled = color_enabled;
        app.reduced_motion = true;
        app.command_palette_open = true;
        app.focus = FocusTarget::CommandPalette;
        app.command_palette_query = "Open Settings".into();
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("▶ Open Settings"), "{output}");
        assert!(output.contains("✓ Ready"), "{output}");
        assert!(output.contains("Available: yes"), "{output}");
        assert!(output.contains("focus trapped"), "{output}");
        assert_eq!(output.matches('▶').count(), 1, "{output}");
    }
}

#[test]
fn theme_command_palette_and_no_color_override_are_explicit() {
    let mut app = App::new(10, 1_000);
    app.command_palette_open = true;
    app.command_palette_query = "Choose theme".into();
    let palette = rendered_text(&app, 100, 25);
    assert!(palette.contains("Choose theme"), "{palette}");
    assert!(palette.contains("named workbench palette"), "{palette}");

    app.command_palette_open = false;
    app.color_enabled = false;
    app.color_forced_off = true;
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::OpenThemePicker);
    let picker = rendered_text(&app, 100, 25);
    assert!(picker.contains("locked by --no-color"), "{picker}");
}

#[test]
fn focus_command_rail_names_current_next_and_previous_panes() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    for (focus, expected) in [
        (
            FocusTarget::Navigator,
            ["Focus Navigator", "Tab Workspace", "Shift+Tab Workspace"],
        ),
        (
            FocusTarget::Workspace,
            ["Focus Workspace", "Tab Navigator", "Shift+Tab Navigator"],
        ),
    ] {
        app.focus = focus;
        let footer = footer_shortcuts(&app);
        for label in expected {
            assert!(footer.contains(label), "{footer}");
        }
    }

    app.screen = Screen::Dashboard;
    app.focus = FocusTarget::Navigator;
    let footer = footer_shortcuts(&app);
    assert!(footer.contains("no other actionable panes"), "{footer}");
    assert!(!footer.contains("Inspector"), "{footer}");
}
#[test]
fn dialog_focus_is_trapped_then_visibly_restored_to_actionable_workspace() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Logs;
    app.focus = FocusTarget::Workspace;
    let _ = update(&mut app, Action::OpenBuildOptions);

    let dialog = rendered_text(&app, 100, 24);
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert!(dialog.contains("Image build options"));
    assert!(!dialog.contains("Panes:"));

    let _ = update(&mut app, Action::CloseBuildOptions);
    let restored = rendered_text(&app, 100, 24);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert!(restored.contains("Log Viewer"));
    assert!(!restored.contains("Image build options"));
}
#[test]
fn formats_error_timestamp_without_panicking() {
    assert_eq!(timestamp_text(UNIX_EPOCH), "0s since Unix epoch");
}
#[test]
fn no_color_selection_uses_reverse_video() {
    let mut app = App::new(10, 1_000);
    app.color_enabled = false;
    assert!(
        selected_style(&app, true)
            .add_modifier
            .contains(Modifier::REVERSED)
    );
    assert_eq!(selected_style(&app, true).bg, None);
}

#[test]
fn renders_notification() {
    let mut app = App::new(1, 1);
    app.notification = Some("Select an image first with i.".into());
    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
    terminal.draw(|f| render(f, &app)).unwrap();
    let screen = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(screen.contains("Message"));
    assert!(screen.contains("Select an image first with i."));
    assert!(screen.contains("Esc dismiss"));
    assert!(screen.contains("? Help"));
}
#[test]
fn dashboard_renders_backend_and_build_metrics() {
    let mut terminal = Terminal::new(TestBackend::new(160, 32)).unwrap();
    let mut app = App::new(10, 1_000);
    app.backend = "bridge".into();
    app.build.completed = 3;
    app.build.total = Some(7);
    app.build.warnings = 2;
    app.build.errors = 1;
    app.workspace.release = Some("kirkstone".into());
    app.workspace.source_dir = Some("/src/poky".into());
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Backend: bridge"));
    assert!(output.contains("Tasks: 3/7"));
    assert!(output.contains("Warnings: 2  Errors: 1"));
    assert!(output.contains("Release: kirkstone"));
}
#[test]
fn bbmask_footer_shows_its_edit_shortcut() {
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Bbmask;
    app.focus = FocusTarget::Workspace;
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("e edit BBMASK"));
}
#[test]
fn compact_resource_meters_remain_visible_across_workspace_sizes() {
    let mut app = literal_reference_app();
    app.focus = FocusTarget::Workspace;
    app.host_telemetry.cpu_utilization_percent = Some(42);
    app.host_telemetry.memory_total_bytes = Some(100);
    app.host_telemetry.memory_available_bytes = Some(25);
    app.host_telemetry.disk_total_bytes = Some(100);
    app.host_telemetry.disk_available_bytes = Some(40);
    app.workspace.build_dir = Some("/work/build".into());
    let selected = app.task_progress_scroll;
    for screen in [Screen::Tasks] {
        app.screen = screen;
        for (width, height) in [(181, 43), (160, 48), (130, 40), (100, 30), (80, 24)] {
            let output = rendered_text_at(&app, width, height, literal_now());
            for expected in [
                "Resources",
                "CPU 42%",
                "RAM 75%",
                "FS 60%",
                "▪",
                "▫",
                "do_compile",
                "Log Viewer",
            ] {
                assert!(
                    output.contains(expected),
                    "{screen:?} {width}x{height}: missing {expected}: {output}"
                );
            }
            assert_eq!(app.task_progress_scroll, selected);
            assert_eq!(app.focus, FocusTarget::Workspace);
        }
    }
}

#[test]
fn compact_resource_meters_distinguish_unknown_zero_and_accessibility() {
    let render_strip = |app: &App, width, height| {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_compact_telemetry_strip(frame, app, frame.area()))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };
    let mut app = App::new(10, 1_000);
    for expected in ["CPU --", "RAM --", "FS --", "unavailable"] {
        assert!(render_strip(&app, 78, 4).contains(expected));
    }
    assert!(!render_strip(&app, 78, 4).contains("0%"));
    app.workspace.build_dir = Some("/work/build".into());
    app.host_telemetry.cpu_utilization_percent = Some(0);
    app.host_telemetry.memory_total_bytes = Some(100);
    app.host_telemetry.memory_available_bytes = Some(100);
    app.host_telemetry.disk_total_bytes = Some(100);
    app.host_telemetry.disk_available_bytes = Some(100);
    let zero = render_strip(&app, 78, 4);
    for expected in ["CPU 0%", "RAM 0%", "FS 0%", "▫"] {
        assert!(zero.contains(expected), "{zero}");
    }
    assert!(!zero.contains('▪'));
    app.host_telemetry.memory_total_bytes = Some(0);
    app.host_telemetry.disk_available_bytes = Some(101);
    let invalid = render_strip(&app, 78, 4);
    assert!(invalid.contains("RAM --") && invalid.contains("FS --"));
    app.host_telemetry.cpu_utilization_percent = Some(42);
    app.preferences.symbols = SymbolPreference::Ascii;
    app.color_enabled = false;
    app.reduced_motion = true;
    let accessible = render_strip(&app, 78, 4);
    for expected in ["CPU 42%", "#", ".", "unavailable"] {
        assert!(accessible.contains(expected), "{accessible}");
    }
    assert!(!accessible.contains('▪') && !accessible.contains('▫'));
    for width in 1..20 {
        for height in 1..4 {
            let _ = render_strip(&app, width, height);
        }
    }
    for (width, height) in [(160, 50), (200, 60)] {
        app.focus = FocusTarget::Workspace;
        let full = rendered_text_at(&app, width, height, literal_now());
        assert!(full.contains("Resource Telemetry"), "{full}");
    }
}

#[test]
fn dashboard_renders_host_cpu_and_build_disk_space() {
    let mut terminal = Terminal::new(TestBackend::new(300, 40)).unwrap();
    let mut app = App::new(10, 1_000);
    app.host_telemetry.cpu_utilization_percent = Some(42);
    app.host_telemetry.disk_available_bytes = Some(8 * 1024 * 1024 * 1024);
    app.host_telemetry.disk_total_bytes = Some(16 * 1024 * 1024 * 1024);
    app.workspace.build_dir = Some("/work/build".into());
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("CPU Usage") && output.contains("42%"));
    assert!(output.contains("8.00 / 16.00 GiB"));
}

#[test]
fn ux_dashboard_composes_priority_actions_attention_work_artifacts_and_health() {
    let mut app = literal_reference_app();
    app.screen = Screen::Dashboard;
    app.focus = FocusTarget::Workspace;
    app.build.errors = 1;
    let _ = update(
        &mut app,
        Action::Log(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Error,
            message: "ERROR: bash:do_compile failed with exit code 1".into(),
            recipe: Some("bash_5.2.21-2".into()),
            task: Some("do_compile".into()),
            path: Some("/work/build/tmp/log.do_compile".into()),
            timestamp: literal_now(),
            build: Some("core-image-minimal".into()),
            protected: true,
            diagnostic: None,
        }),
    );
    let completed = app
        .background_jobs
        .jobs
        .iter_mut()
        .rev()
        .find(|job| job.status == BackgroundJobStatus::Succeeded)
        .expect("literal fixture retains a successful job");
    completed
        .result
        .as_mut()
        .expect("successful literal job has a result")
        .artifacts
        .push("/deploy/core-image-minimal.wic".into());

    let wide = rendered_text_at(&app, 160, 50, literal_now());
    for anchor in [
        "Build Overview",
        "Overall  40%  4/10",
        "do_compile",
        "Quick Actions",
        "Attention",
        "Job History",
        "core-image-minimal",
        "Resource Telemetry",
        "Project Inspector",
        "Workspace",
    ] {
        assert!(wide.contains(anchor), "missing {anchor}: {wide}");
    }
}
