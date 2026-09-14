//! Regression tests grouped around next_generation_palette_retains_typed_facts_at_every_breakpoint.
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
    for (focus, expected) in [
        (
            FocusTarget::Navigator,
            ["Focus Navigator", "→ Workspace", "← Inspector"],
        ),
        (
            FocusTarget::Workspace,
            ["Focus Workspace", "→ Inspector", "← Navigator"],
        ),
        (
            FocusTarget::Inspector,
            ["Focus Inspector", "→ Navigator", "← Workspace"],
        ),
    ] {
        app.focus = focus;
        let footer = footer_shortcuts(&app);
        for label in expected {
            assert!(footer.contains(label), "{footer}");
        }
    }
}
#[test]
fn dialog_focus_is_trapped_then_visibly_restored_to_inspector() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Logs;
    app.focus = FocusTarget::Inspector;
    let _ = update(&mut app, Action::OpenBuildOptions);

    let dialog = rendered_text(&app, 100, 24);
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert!(dialog.contains("Image build options"));
    assert!(!dialog.contains("Panes:"));

    let _ = update(&mut app, Action::CloseBuildOptions);
    let restored = rendered_text(&app, 100, 24);
    assert_eq!(app.focus, FocusTarget::Inspector);
    assert!(restored.contains("Inspector"));
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
    for screen in [Screen::Dashboard, Screen::Tasks] {
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
    assert!(output.contains("Host CPU: 42%"));
    assert!(output.contains("Build disk free: 8.0 GiB"));
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
        "Tasks: Build",
        "Overall  40%  4/10",
        "do_compile",
        "Log Viewer",
        "ERROR: bash:do_compile failed with exit code 1",
        "Job History",
        "core-image-minimal",
        "Resource Telemetry",
        "Inspector: Task",
        "System Status",
    ] {
        assert!(wide.contains(anchor), "missing {anchor}: {wide}");
    }
}

#[test]
fn ux_dashboard_is_responsive_accessible_and_explicit_across_lifecycle_states() {
    let mut app = literal_reference_app();
    app.screen = Screen::Dashboard;
    app.focus = FocusTarget::Workspace;
    for (width, height) in [(160, 50), (130, 40), (100, 30), (80, 24)] {
        let output = rendered_text_at(&app, width, height, literal_now());
        assert!(output.contains("Tasks:"), "{width}x{height}: {output}");
        assert!(
            output.contains("Log Viewer") || output.contains("Overall"),
            "{width}x{height}: {output}"
        );
        assert!(output.contains("do_compile"), "{width}x{height}: {output}");
        assert!(!output.contains('�'), "{width}x{height}: {output}");
    }

    app.color_enabled = false;
    app.reduced_motion = true;
    app.build.status = BuildStatus::Failed;
    app.build.errors = 1;
    let failed = rendered_text_at(&app, 100, 30, literal_now());
    assert!(failed.contains("Overall"), "{failed}");
    assert!(failed.contains("Errors: 1"), "{failed}");

    app.build.status = BuildStatus::Completed;
    app.build.errors = 0;
    app.logs.clear_entries();
    let job = app
        .background_jobs
        .jobs
        .iter_mut()
        .find(|job| job.status == BackgroundJobStatus::Succeeded)
        .expect("literal fixture retains a successful job");
    job.result
        .as_mut()
        .expect("successful job has result")
        .artifacts
        .push("/deploy/completed.wic".into());
    let completed = rendered_text_at(&app, 130, 40, literal_now());
    assert!(completed.contains("Job History"), "{completed}");
    assert!(completed.contains("core-image-minimal"), "{completed}");

    let mut empty = App::new(16, 4_096);
    empty.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    empty.build_environment = BuildEnvironmentState::Unconfigured;
    empty.workspace.source_dir = None;
    empty.workspace.build_dir = None;
    let empty_output = rendered_text_at(&empty, 160, 50, literal_now());
    assert!(
        empty_output.contains("build not started · 0%"),
        "{empty_output}"
    );
    assert!(empty_output.contains("No task selected"), "{empty_output}");
    assert!(empty_output.contains("Job History"), "{empty_output}");
    assert!(
        empty_output.contains("Resource Telemetry"),
        "{empty_output}"
    );
    empty.build.status = BuildStatus::Failed;
    empty.build.errors = 1;
    let failed_without_rows = rendered_text_at(&empty, 160, 50, literal_now());
    assert!(
        failed_without_rows.contains("Errors: 1"),
        "{failed_without_rows}"
    );
}

#[test]
fn ux_command_center_unifies_bounded_source_contexts_without_bypassing_workspaces() {
    let mut app = literal_reference_app();
    app.screen = Screen::Dashboard;
    app.focus = FocusTarget::Workspace;
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
            id: yoctui_model::BackgroundJobId(99),
            kind: BackgroundJobKind::Build,
            title: "command-center-build".into(),
            context: yoctui_model::BackgroundJobContext {
                workspace: Some(Screen::Recipes),
                target: Some("core-image-minimal".into()),
                recipe: Some("busybox".into()),
                task: Some("do_compile".into()),
                image: None,
                path: None,
            },
            cancellation_supported: true,
            queued_at: literal_now(),
        }),
    );
    let command = yoctui_model::builtin_raw_catalog()
        .commands
        .iter()
        .find(|command| {
            command.parameters.is_empty()
                && matches!(
                    command.execution,
                    yoctui_model::RawExecutionPolicy::Executable { .. }
                )
        })
        .expect("the built-in catalog retains a parameterless executable command");
    app.raw_mode.favorites.push(
        yoctui_model::RawFavorite::new(
            command,
            "Env check",
            Default::default(),
            yoctui_model::RawAdditionalArguments::from_vec(Vec::new()).unwrap(),
            0,
        )
        .unwrap(),
    );
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 99,
            name: "sh".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 2,
        });
    app.pty_selection = app.daemon.pty_sessions.len() - 1;

    let wide = rendered_text_at(&app, 160, 50, literal_now());
    for anchor in [
        "Tasks: Build",
        "Job History",
        "System Status",
        "PTY 2",
        "1 queued",
    ] {
        assert!(wide.contains(anchor), "missing {anchor}: {wide}");
    }

    let compact = rendered_text_at(&app, 80, 24, literal_now());
    for anchor in ["Navigator", "Running", "1 queued"] {
        assert!(compact.contains(anchor), "missing {anchor}: {compact}");
    }
}
#[test]
fn next_generation_cpu_gauge_is_numeric_responsive_and_accessible() {
    let render_gauge = |app: &App, width| {
        let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
        terminal
            .draw(|frame| render_cpu_gauge(frame, app, frame.area()))
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
    app.host_telemetry.cpu_utilization_percent = Some(42);
    app.host_telemetry.logical_cpu_count = Some(16);
    let wide = render_gauge(&app, 36);
    assert!(wide.contains("CPU  42% · 16 cores"), "{wide}");
    let medium = render_gauge(&app, 18);
    assert!(medium.contains("CPU 42% · 16c"), "{medium}");
    let narrow = render_gauge(&app, 10);
    assert!(narrow.contains("CPU 42%"), "{narrow}");
    assert!(!narrow.contains("16c"), "{narrow}");

    app.host_telemetry.logical_cpu_count = None;
    let unknown_cores = render_gauge(&app, 36);
    assert!(unknown_cores.contains("CPU  42%"), "{unknown_cores}");
    assert!(!unknown_cores.contains("cores"), "{unknown_cores}");

    app.host_telemetry.cpu_utilization_percent = None;
    let unavailable = render_gauge(&app, 24);
    assert!(unavailable.contains("CPU ! unavailable"), "{unavailable}");
    assert!(!unavailable.contains("0%"), "{unavailable}");

    app.host_telemetry.cpu_utilization_percent = Some(87);
    app.host_telemetry.logical_cpu_count = Some(8);
    app.theme = Theme::HighContrast;
    app.reduced_motion = true;
    let reduced_motion = render_gauge(&app, 36);
    assert!(reduced_motion.contains("CPU  87% · 8 cores"));
    app.color_enabled = false;
    let no_color = render_gauge(&app, 18);
    assert!(no_color.contains("CPU 87% · 8c"), "{no_color}");
}
#[test]
fn next_generation_ram_gauge_is_honest_responsive_and_accessible() {
    let render_gauge = |app: &App, width| {
        let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
        terminal
            .draw(|frame| render_ram_gauge(frame, app, frame.area()))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };

    let gib = 1024_u64.pow(3);
    let mut app = App::new(10, 1_000);
    app.host_telemetry.memory_total_bytes = Some(16 * gib);
    app.host_telemetry.memory_available_bytes = Some(4 * gib);
    let wide = render_gauge(&app, 42);
    assert!(wide.contains("RAM  75% · 12.0 GiB / 16.0 GiB"), "{wide}");
    let medium = render_gauge(&app, 30);
    assert!(medium.contains("RAM 75% · 12.0/16.0 GiB"), "{medium}");
    let narrow = render_gauge(&app, 14);
    assert!(narrow.contains("RAM 75%"), "{narrow}");
    assert!(!narrow.contains("GiB"), "{narrow}");

    app.host_telemetry.memory_total_bytes = Some(u64::MAX);
    app.host_telemetry.memory_available_bytes = Some(u64::MAX / 2);
    let large = render_gauge(&app, 14);
    assert!(large.contains("RAM 50%"), "{large}");

    for (total, available) in [(None, None), (Some(0), Some(0)), (Some(10), Some(11))] {
        app.host_telemetry.memory_total_bytes = total;
        app.host_telemetry.memory_available_bytes = available;
        let unavailable = render_gauge(&app, 24);
        assert!(unavailable.contains("RAM ! unavailable"), "{unavailable}");
        assert!(!unavailable.contains("0%"), "{unavailable}");
    }

    app.host_telemetry.memory_total_bytes = Some(8 * gib);
    app.host_telemetry.memory_available_bytes = Some(2 * gib);
    app.theme = Theme::HighContrast;
    app.reduced_motion = true;
    assert!(render_gauge(&app, 30).contains("RAM 75% · 6.0/8.0 GiB"));
    app.color_enabled = false;
    assert!(render_gauge(&app, 14).contains("RAM 75%"));
}
#[test]
fn next_generation_disk_gauge_keeps_capacity_context_and_unavailable_honest() {
    let render_gauge = |app: &App, width| {
        let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
        terminal
            .draw(|frame| render_disk_gauge(frame, app, frame.area()))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };

    let gib = 1024_u64.pow(3);
    let mut app = App::new(10, 1_000);
    app.workspace.build_dir = Some("/work/build".into());
    app.host_telemetry.disk_total_bytes = Some(100 * gib);
    app.host_telemetry.disk_available_bytes = Some(40 * gib);
    let wide = render_gauge(&app, 80);
    assert!(
        wide.contains("BUILD FS  60% · 40.0/100.0 GiB free · /work/build"),
        "{wide}"
    );
    let medium = render_gauge(&app, 42);
    assert!(
        medium.contains("BUILD FS 60% · 40.0/100.0 GiB free"),
        "{medium}"
    );
    let narrow = render_gauge(&app, 20);
    assert!(narrow.contains("BUILD FS 60%"), "{narrow}");
    let minimum = render_gauge(&app, 10);
    assert!(minimum.contains("FS 60%"), "{minimum}");

    app.workspace.build_dir = None;
    let missing_context = render_gauge(&app, 28);
    assert!(
        missing_context.contains("BUILD FS ! unavailable"),
        "{missing_context}"
    );
    assert!(!missing_context.contains("0%"), "{missing_context}");

    app.workspace.build_dir = Some("/work/build".into());
    for (total, available) in [(None, None), (Some(0), Some(0)), (Some(10), Some(11))] {
        app.host_telemetry.disk_total_bytes = total;
        app.host_telemetry.disk_available_bytes = available;
        let unavailable = render_gauge(&app, 28);
        assert!(
            unavailable.contains("BUILD FS ! unavailable"),
            "{unavailable}"
        );
        assert!(!unavailable.contains("0%"), "{unavailable}");
    }

    app.host_telemetry.disk_total_bytes = Some(8 * gib);
    app.host_telemetry.disk_available_bytes = Some(2 * gib);
    app.theme = Theme::HighContrast;
    app.reduced_motion = true;
    assert!(render_gauge(&app, 42).contains("BUILD FS 75% · 2.0/8.0 GiB free"));
    app.color_enabled = false;
    assert!(render_gauge(&app, 20).contains("BUILD FS 75%"));
}
#[test]
fn next_generation_disk_io_sparklines_keep_current_and_history_distinct() {
    let render_io = |app: &App, width| {
        let mut terminal = Terminal::new(TestBackend::new(width, 2)).unwrap();
        terminal
            .draw(|frame| {
                let rows = Layout::vertical([Constraint::Length(1); 2]).split(frame.area());
                render_disk_io(frame, app, rows[0], rows[1]);
            })
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
    app.host_telemetry.disk_read_bytes_per_second = Some(2 * 1024);
    app.host_telemetry.disk_write_bytes_per_second = Some(4 * 1024);
    app.host_telemetry_history
        .disk_read_bytes_per_second
        .extend([512, 1024, 2 * 1024]);
    app.host_telemetry_history
        .disk_write_bytes_per_second
        .extend([4 * 1024, 1024, 3 * 1024, 4 * 1024]);
    let wide = render_io(&app, 42);
    assert!(wide.contains("Read 2.0 KiB/s"), "{wide}");
    assert!(wide.contains("Write 4.0 KiB/s"), "{wide}");
    assert!(wide.chars().any(|character| "▁▂▃▄▅▆▇█".contains(character)));

    let narrow = render_io(&app, 16);
    assert!(narrow.contains("R 2.0 KiB/s"), "{narrow}");
    assert!(narrow.contains("W 4.0 KiB/s"), "{narrow}");

    app.host_telemetry.disk_read_bytes_per_second = None;
    app.host_telemetry.disk_write_bytes_per_second = None;
    let unavailable = render_io(&app, 42);
    assert!(unavailable.contains("Read ! unavailable"), "{unavailable}");
    assert!(unavailable.contains("Write ! unavailable"), "{unavailable}");
    assert!(!unavailable.contains("0 B/s"), "{unavailable}");
    assert!(
        unavailable
            .chars()
            .any(|character| "▁▂▃▄▅▆▇█".contains(character)),
        "retained valid history should remain visible: {unavailable}"
    );

    app.host_telemetry.disk_read_bytes_per_second = Some(0);
    app.host_telemetry.disk_write_bytes_per_second = Some(0);
    app.theme = Theme::HighContrast;
    app.reduced_motion = true;
    assert!(render_io(&app, 42).contains("Read 0 B/s"));
    app.color_enabled = false;
    assert!(render_io(&app, 16).contains("R 0 B/s"));
}
#[test]
fn next_generation_network_io_sparklines_keep_current_and_history_distinct() {
    let render_io = |app: &App, width| {
        let mut terminal = Terminal::new(TestBackend::new(width, 2)).unwrap();
        terminal
            .draw(|frame| {
                let rows = Layout::vertical([Constraint::Length(1); 2]).split(frame.area());
                render_network_io(frame, app, rows[0], rows[1]);
            })
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
    app.host_telemetry.network_receive_bytes_per_second = Some(3 * 1024);
    app.host_telemetry.network_transmit_bytes_per_second = Some(6 * 1024);
    app.host_telemetry_history
        .network_receive_bytes_per_second
        .extend([512, 1024, 3 * 1024]);
    app.host_telemetry_history
        .network_transmit_bytes_per_second
        .extend([6 * 1024, 1024, 4 * 1024, 6 * 1024]);
    let wide = render_io(&app, 42);
    assert!(wide.contains("RX 3.0 KiB/s"), "{wide}");
    assert!(wide.contains("TX 6.0 KiB/s"), "{wide}");
    assert!(wide.chars().any(|character| "▁▂▃▄▅▆▇█".contains(character)));

    let narrow = render_io(&app, 16);
    assert!(narrow.contains("RX 3.0 KiB/s"), "{narrow}");
    assert!(narrow.contains("TX 6.0 KiB/s"), "{narrow}");

    app.host_telemetry.network_receive_bytes_per_second = None;
    app.host_telemetry.network_transmit_bytes_per_second = None;
    let unavailable = render_io(&app, 42);
    assert!(unavailable.contains("RX ! unavailable"), "{unavailable}");
    assert!(unavailable.contains("TX ! unavailable"), "{unavailable}");
    assert!(!unavailable.contains("0 B/s"), "{unavailable}");
    assert!(
        unavailable
            .chars()
            .any(|character| "▁▂▃▄▅▆▇█".contains(character)),
        "retained valid history should remain visible: {unavailable}"
    );

    app.host_telemetry.network_receive_bytes_per_second = Some(0);
    app.host_telemetry.network_transmit_bytes_per_second = Some(0);
    app.theme = Theme::HighContrast;
    app.reduced_motion = true;
    assert!(render_io(&app, 42).contains("RX 0 B/s"));
    app.color_enabled = false;
    assert!(render_io(&app, 16).contains("TX 0 B/s"));
}
