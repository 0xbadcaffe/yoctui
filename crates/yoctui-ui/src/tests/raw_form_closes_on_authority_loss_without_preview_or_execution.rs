//! Regression tests grouped around raw_form_closes_on_authority_loss_without_preview_or_execution.
use super::*;

#[test]
fn raw_form_closes_on_authority_loss_without_preview_or_execution() {
    for query in ["--show-versions", "grep '^PACKAGES='"] {
        let mut denied = raw_command_list_app();
        set_raw_command_query(&mut denied, query);
        let _ = update(
            &mut denied,
            Action::RawMode(yoctui_model::RawModeAction::OpenSelected),
        );
        assert_eq!(denied.raw_mode.view, yoctui_model::RawModeView::Browser);
        assert!(denied.raw_mode.form.is_none());
        assert!(denied.raw_mode.preview.is_none());
        assert!(denied.raw_mode.execution.is_none());
    }

    let mut app = raw_command_list_app();
    set_raw_command_query(&mut app, "--continue <target>");
    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::OpenSelected),
    );
    let mut replacement = app.workspace_compatibility.authority().unwrap().clone();
    replacement.snapshot.generation = 20;
    replacement.snapshot.environment.build_directory = yoctui_model::AuthoritativeValue::unknown();
    yoctui_model::install_workspace_compatibility(&mut app, replacement).unwrap();
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Browser);
    assert!(app.raw_mode.form.is_none());
    assert!(app.raw_mode.preview.is_none());
    assert!(app.raw_mode.execution.is_none());
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert!(
        app.raw_mode
            .notification
            .as_deref()
            .is_some_and(|message| message.contains("build-directory authority was lost"))
    );
}

#[test]
fn raw_command_list_renders_exact_template_favorite_and_five_states() {
    let mut app = raw_command_list_app();
    for (query, template, state, execution) in [
        ("--version", "bitbake --version", "AVAILABLE", "RUN"),
        (
            "--continue <target>",
            "bitbake --continue <target>",
            "LIMITED",
            "RUN",
        ),
        (
            "--show-versions",
            "bitbake --show-versions",
            "UNAVAILABLE",
            "RUN",
        ),
        (
            "--ui=<ui> <target>",
            "bitbake --ui=<ui> <target>",
            "UNKNOWN",
            "RUN",
        ),
        (
            "grep '^PACKAGES='",
            "grep '^PACKAGES='",
            "UNSUPPORTED",
            "REF",
        ),
    ] {
        set_raw_command_query(&mut app, query);
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains(template), "missing {template:?}: {output}");
        assert!(output.contains(state), "missing {state:?}: {output}");
        assert!(
            output.contains(execution),
            "missing {execution:?}: {output}"
        );
        assert!(output.contains("NOT-FAV"), "{output}");
    }

    set_raw_command_query(&mut app, "--version");
    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::ToggleFavorite),
    );
    let favorite = rendered_text(&app, 80, 24);
    assert!(favorite.contains("· FAV"), "{favorite}");

    app.color_enabled = false;
    let no_color = rendered_text(&app, 80, 24);
    assert!(no_color.contains("RUN · AVAILABLE · FAV"), "{no_color}");

    set_raw_command_query(&mut app, "grep '^PACKAGES='");
    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::OpenSelected),
    );
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Browser);
    assert!(app.raw_mode.form.is_none());
    assert!(
        app.raw_mode
            .notification
            .as_deref()
            .unwrap()
            .contains("reference-only")
    );
}

#[test]
fn raw_search_command_list_bounds_empty_stale_and_responsive_state() {
    let mut app = raw_command_list_app();
    set_raw_command_query(&mut app, "bitbake");
    let total = app
        .raw_mode
        .visible_commands(yoctui_model::builtin_raw_catalog())
        .len();
    assert!(total > 100);
    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::SelectCommand { delta: isize::MAX }),
    );
    let last = rendered_text(&app, 80, 24);
    assert!(last.contains('↑'), "{last}");
    assert!(last.contains(&format!("/{total}")), "{last}");
    assert!(last.contains('▶'), "{last}");

    let selected = app.raw_mode.command.clone();
    for (width, height) in [(160, 40), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(
            output.contains("Raw Mode / Commands"),
            "{width}x{height}: {output}"
        );
        assert_eq!(app.raw_mode.command, selected);
    }
    let below_minimum = rendered_text(&app, 79, 23);
    assert!(below_minimum.contains("Yoctui needs at least 80x24"));

    set_raw_command_query(&mut app, "definitely-no-such-raw-command");
    let empty = rendered_text(&app, 80, 24);
    assert!(empty.contains("0/0"), "{empty}");
    assert!(
        empty.contains("No Raw commands match this search"),
        "{empty}"
    );

    app.raw_mode.command = Some(yoctui_model::RawCommandId::new("stale.command.identity").unwrap());
    let stale = rendered_text(&app, 80, 24);
    assert!(
        stale.contains("No Raw commands match this search"),
        "{stale}"
    );
    assert!(!stale.contains('▶'), "{stale}");
}

#[test]
fn raw_command_help_follows_exact_selection_and_renders_every_field() {
    let mut app = raw_command_list_app();
    set_raw_command_query(&mut app, "--ui=<ui> <target>");
    let selected = app
        .raw_mode
        .selected_command(yoctui_model::builtin_raw_catalog())
        .unwrap();
    let output = rendered_text(&app, 200, 60);
    for expected in [
        format!("Description: {}", selected.reference.description),
        format!("Reference section: {}", selected.reference.heading),
        format!("Template: {}", raw_command_template(selected)),
        "Availability: UNKNOWN".into(),
        "Reason [bitbake.raw.ui]: UI support is unknown.".into(),
        "Implementation: none selected".into(),
        "Interaction: Interactive PTY".into(),
        "Safety: Build/mutating".into(),
        "Parameters:".into(),
        "- Ui <ui> · User interface · Required".into(),
        "- Target <target> · Target · Required".into(),
        "Favorite: No".into(),
    ] {
        assert!(output.contains(&expected), "missing {expected:?}: {output}");
    }

    let old_description = selected.reference.description.clone();
    set_raw_command_query(&mut app, "--version");
    let changed = rendered_text(&app, 200, 60);
    assert!(
        changed.contains("Description: Show the installed BitBake version."),
        "{changed}"
    );
    assert!(!changed.contains(&old_description), "{changed}");
    assert!(changed.contains("Parameters: none"), "{changed}");
    assert!(
        changed.contains("Implementation [bitbake.raw.cli]: bitbake.raw.argv"),
        "{changed}"
    );
    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::ToggleFavorite),
    );
    let favorite = rendered_text(&app, 200, 60);
    assert!(favorite.contains("Favorite: Yes"), "{favorite}");
}

#[test]
fn raw_command_help_explains_all_availability_states_and_reference_only_rows() {
    let mut app = raw_command_list_app();
    for (query, state, exact_detail) in [
        (
            "--version",
            "AVAILABLE",
            "Implementation [bitbake.raw.cli]: bitbake.raw.argv",
        ),
        (
            "--continue <target>",
            "LIMITED",
            "Reason [bitbake.raw.continue]: Continue is available",
        ),
        (
            "--show-versions",
            "UNAVAILABLE",
            "Reason [bitbake.raw.show_versions]: Show versions is unavailable.",
        ),
        (
            "--ui=<ui> <target>",
            "UNKNOWN",
            "Reason [bitbake.raw.ui]: UI support is unknown.",
        ),
        (
            "grep '^PACKAGES='",
            "UNSUPPORTED",
            "Interaction: Reference only (shell pipeline)",
        ),
    ] {
        set_raw_command_query(&mut app, query);
        let output = rendered_text(&app, 200, 60);
        assert!(
            output.contains(&format!("Availability: {state}")),
            "{output}"
        );
        assert!(output.contains(exact_detail), "{output}");
    }

    let reference = rendered_text(&app, 200, 60);
    assert!(reference.contains("Reason [reference]:"), "{reference}");
    assert!(
        reference.contains("Safety: Unsupported reference"),
        "{reference}"
    );
    assert!(
        reference.contains("Implementation: none selected"),
        "{reference}"
    );
}

#[test]
fn raw_accessibility_help_is_bounded_empty_no_color_and_responsive() {
    let mut app = raw_command_list_app();
    set_raw_command_query(&mut app, "--continue <target>");
    app.focus = FocusTarget::Workspace;
    app.color_enabled = false;
    let selection = app.raw_mode.command.clone();
    let output = rendered_text(&app, 160, 50);
    assert!(output.contains("Inspector: Raw command"), "{output}");
    assert!(output.contains("Availability: LIMITED"), "{output}");
    assert!(!output.contains('�'), "{output}");
    assert_eq!(app.raw_mode.command, selection);

    set_raw_command_query(&mut app, "definitely-no-such-raw-command");
    let empty = rendered_text(&app, 160, 30);
    assert!(empty.contains("No Raw command selected."), "{empty}");
    assert!(
        empty.contains("Select a command in the Workspace"),
        "{empty}"
    );

    app.raw_mode.command = Some(yoctui_model::RawCommandId::new("stale.command.identity").unwrap());
    let stale = rendered_text(&app, 160, 30);
    assert!(stale.contains("No Raw command selected."), "{stale}");
    let below_minimum = rendered_text(&app, 79, 23);
    assert!(below_minimum.contains("Yoctui needs at least 80x24"));
}

#[test]
fn raw_category_browser_renders_pinned_order_classification_and_bounds() {
    let mut app = App::new(16, 4096);
    let _ = update(&mut app, Action::Open(Screen::RawMode));
    app.focus = FocusTarget::Workspace;
    let wide = rendered_text(&app, 160, 50);
    for expected in [
        "[FAVORITES] Favorites",
        "[BITBAKE] Version and help",
        "[REFERENCE] Useful paths",
        "[CONCEPT] Quick conceptual",
        "[COMPANION] bitbake-setup",
        "1-32/32",
    ] {
        assert!(wide.contains(expected), "missing {expected:?}: {wide}");
    }
    assert!(
        wide.find("[FAVORITES]").unwrap() < wide.find("[BITBAKE]").unwrap(),
        "{wide}"
    );
    assert!(wide.contains('…'), "long category labels must clip: {wide}");

    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::SelectCategory { delta: isize::MAX }),
    );
    let narrow = rendered_text(&app, 80, 24);
    assert!(
        narrow.contains("One-screen emergency reference"),
        "{narrow}"
    );
    assert!(narrow.contains("↑"), "{narrow}");
    assert!(narrow.contains("/32"), "{narrow}");

    let selected = app.raw_mode.category.clone();
    app.raw_mode.search.query = "reference".into();
    for (width, height) in [(160, 40), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Categories"), "{width}x{height}: {output}");
        assert_eq!(app.raw_mode.category, selected);
        assert_eq!(app.raw_mode.search.query, "reference");
    }
    let below_minimum = rendered_text(&app, 79, 23);
    assert!(below_minimum.contains("Yoctui needs at least 80x24"));
}

#[test]
fn raw_category_browser_exposes_column_state_and_no_color_text() {
    let mut app = App::new(16, 4096);
    let _ = update(&mut app, Action::Open(Screen::RawMode));
    app.focus = FocusTarget::Workspace;
    app.color_enabled = false;
    let categories = rendered_text(&app, 80, 24);
    assert!(
        categories.contains("▶ [FAVORITES] Favorites"),
        "{categories}"
    );

    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::FocusCommands),
    );
    let commands = rendered_text(&app, 80, 24);
    for expected in [
        "Commands",
        "No favorite Raw commands",
        "f marks a selected command as a favorite",
        "Left/h returns to categories",
    ] {
        assert!(
            commands.contains(expected),
            "missing {expected:?}: {commands}"
        );
    }

    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::FocusCategories),
    );
    app.raw_mode.category = None;
    let unselected = rendered_text(&app, 80, 24);
    assert!(!unselected.contains('▶'), "{unselected}");
}

#[test]
fn raw_responsive_navigation_renders_with_exact_shell_help() {
    let mut app = App::new(16, 4096);
    let _ = update(&mut app, Action::Open(Screen::RawMode));
    app.focus = FocusTarget::Workspace;
    for (width, height) in [(160, 40), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Raw Mode"), "{width}x{height}: {output}");
        assert!(
            output.contains("Categories") && output.contains("[FAVORITES] Favorites"),
            "{width}x{height}: {output}"
        );
    }

    let below_minimum = rendered_text(&app, 79, 23);
    assert!(below_minimum.contains("Yoctui needs at least 80x24"));

    let _ = update(&mut app, Action::Open(Screen::Help));
    let help = rendered_text(&app, 160, 52);
    assert!(help.contains("Raw Mode: Left/Right browser pane"), "{help}");

    let _ = update(&mut app, Action::Open(Screen::RawMode));
    app.focus = FocusTarget::Workspace;
    let shortcuts = footer_shortcuts(&app);
    assert!(shortcuts.contains("f Favorite | H History"), "{shortcuts}");
    assert!(shortcuts.contains("F1 Help | F10 Menu"), "{shortcuts}");
}

#[test]
fn ux_scroll_production_renderer_exposes_bounded_position_at_every_breakpoint() {
    let mut app = App::new(16, 4_096);
    let _ = update(&mut app, Action::Open(Screen::RawMode));
    app.focus = FocusTarget::Workspace;
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("/32"), "{width}x{height}: {output}");
        assert!(output.contains('↓'), "{width}x{height}: {output}");
    }

    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::SelectCategory { delta: isize::MAX }),
    );
    app.color_enabled = false;
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("/32"), "{output}");
    assert!(output.contains('↑'), "{output}");
    assert!(!output.contains('�'), "{output}");
}

#[test]
fn ux_dashboard_uses_live_build_cockpit_and_shared_braille_activity_language() {
    let mut app = App::new(20, 2_000);
    app.screen = Screen::Tasks;
    app.host_telemetry.cpu_utilization_percent = Some(42);
    let task = yoctui_model::TaskInfo::active(
        yoctui_model::TaskId("bash:do_compile".into()),
        "bash".into(),
        "do_compile".into(),
    );
    app.tasks.insert(task.id.clone(), task);
    let output = rendered_text(&app, 160, 50);
    for anchor in [
        "Tasks:",
        "do_compile",
        "Log Viewer",
        "Job History",
        "Resource Telemetry",
        "Inspector: Task",
    ] {
        assert!(output.contains(anchor), "missing {anchor}: {output}");
    }
    assert_ne!(startup_activity_symbol(0), startup_activity_symbol(1));
    assert!(
        startup_activity_symbol(0)
            .chars()
            .all(|glyph| ('\u{2800}'..='\u{28ff}').contains(&glyph))
    );
}

#[test]
fn ux_progress_production_renderer_keeps_exact_and_unknown_progress_honest() {
    let mut app = App::new(16, 4_096);
    app.build.status = BuildStatus::Running;
    app.build.target = Some("core-image-minimal".into());
    app.build.completed = 3;
    app.build.total = Some(10);
    app.build.parse_current = Some(20);
    app.build.parse_total = Some(20);
    app.host_telemetry.cpu_utilization_percent = Some(72);
    let _ = update(&mut app, Action::Open(Screen::Tasks));

    let determinate = rendered_text(&app, 160, 50);
    assert!(determinate.contains("Overall  30%  3/10"), "{determinate}");
    assert!(determinate.contains("CPU Usage"), "{determinate}");
    assert!(determinate.contains("72%"), "{determinate}");
    assert!(!determinate.contains("Sstate 0%"), "{determinate}");

    app.build.total = None;
    app.reduced_motion = true;
    let unknown = rendered_text(&app, 100, 30);
    assert!(
        unknown.contains("Overall  progress unknown ⣿  3/—"),
        "{unknown}"
    );
    assert!(!unknown.contains("Overall  0%"), "{unknown}");

    let hierarchy = app.progress_hierarchy_at(SystemTime::now());
    assert_eq!(
        hierarchy.parse.state,
        yoctui_model::WidgetState::TerminalSuccess
    );
    assert_eq!(hierarchy.runqueue.state, yoctui_model::WidgetState::Active);
    assert_eq!(
        hierarchy.sstate.state,
        yoctui_model::WidgetState::Unavailable
    );
}

#[test]
fn ux_terminal_workbench_renders_writer_read_only_recovery_and_help_states() {
    let mut app = ux_terminal_render_fixture();
    let wide = rendered_text(&app, 160, 50);
    assert!(wide.contains("Terminal Sessions"), "{wide}");
    assert!(wide.contains("WRITER"), "{wide}");
    assert!(wide.contains("read-only"), "{wide}");
    assert!(wide.contains("dropped line-feeds ≥ 312"), "{wide}");

    let mut selected = ux_terminal_render_fixture();
    selected.pane_layout =
        yoctui_model::PaneLayout::new(yoctui_model::PaneId(1)).expect("valid single pane");
    selected.pty_selection = 1;
    let selected = rendered_text(&selected, 120, 35);
    assert!(
        selected.contains("devshell:busybox: bounded output"),
        "{selected}"
    );

    app.color_enabled = false;
    let narrow = rendered_text(&app, 80, 24);
    assert!(!narrow.contains('�'), "{narrow}");
    assert!(narrow.contains("CURRENT"), "{narrow}");

    app.daemon.status = yoctui_model::ClientReplicaStatus::Synchronizing;
    let reconnecting = rendered_text(&app, 100, 30);
    assert!(reconnecting.contains("RECONNECTING"), "{reconnecting}");
    assert!(
        reconnecting.contains("reconnect for control"),
        "{reconnecting}"
    );

    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::Help;
    let help = rendered_text(&app, 120, 35);
    assert!(help.contains("literal Ctrl+B"), "{help}");
    assert!(
        help.contains("close pane (process keeps running)"),
        "{help}"
    );

    app.daemon.pty_sessions.clear();
    app.daemon.pty_details.clear();
    app.daemon.pty_screens.clear();
    let empty_help = rendered_text(&app, 80, 24);
    assert!(
        empty_help.contains("Terminal keyboard help"),
        "{empty_help}"
    );
    assert!(empty_help.contains("literal Ctrl+B"), "{empty_help}");

    app = ux_terminal_render_fixture();
    app.daemon.pty_sessions[0].lifecycle = yoctui_model::ClientDaemonLifecycle::Exited;
    app.daemon.pty_sessions[1].lifecycle = yoctui_model::ClientDaemonLifecycle::Lost;
    let terminal_outcomes = rendered_text(&app, 120, 35);
    assert!(terminal_outcomes.contains("Exited"), "{terminal_outcomes}");
    assert!(terminal_outcomes.contains("Lost"), "{terminal_outcomes}");
}

#[test]
fn ux_preferences_render_real_settings_across_sizes_and_accessibility_modes() {
    let mut app = App::new(32, 4_096);
    app.screen = Screen::Settings;
    app.focus = FocusTarget::Workspace;
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Settings"), "{width}x{height}: {output}");
        assert!(
            output.contains("Active value"),
            "{width}x{height}: {output}"
        );
        assert!(!output.contains('�'), "{width}x{height}: {output}");
    }

    app.settings_selection = 12;
    let locked = rendered_text(&app, 80, 24);
    assert!(locked.contains("MetadataOnly"), "{locked}");
    assert!(locked.contains("Raster preview is unavailable"), "{locked}");

    app.settings_selection = 5;
    app.color_forced_off = true;
    app.color_enabled = false;
    let no_color = rendered_text(&app, 100, 30);
    assert!(no_color.contains("Disabled by --no-color"), "{no_color}");
    assert!(
        no_color.contains("stored choice is preserved"),
        "{no_color}"
    );

    app.color_forced_off = false;
    app.preferences.density = yoctui_model::UiDensity::Compact;
    app.preferences.symbols = SymbolPreference::Ascii;
    app.preferences.charts = yoctui_model::ChartPreference::AccessibleText;
    app.preferences.footer_shortcuts = false;
    app.reduced_motion = true;
    let accessible = rendered_text(&app, 100, 30);
    assert!(accessible.contains("shortcuts hidden"), "{accessible}");
    assert!(!accessible.contains('�'), "{accessible}");
}

#[test]
fn kernel_workspace_renders_configuration_and_device_tree_inventory() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::Kernel;
    app.kernel.inventory = PlatformInventoryState::Available(yoctui_model::PlatformInventory {
        component: yoctui_model::PlatformComponent::Kernel,
        target: "virtual/kernel".into(),
        provider: Some("/layers/linux-yocto.bb".into()),
        tasks: vec!["do_menuconfig".into()],
        roots: vec!["/work/kernel".into()],
        files: vec![yoctui_model::PlatformFile {
            path: "/work/kernel/.config".into(),
            root: "/work/kernel".into(),
            kind: yoctui_model::PlatformFileKind::DotConfig,
            size_bytes: 42,
        }],
        dtc: Some("/usr/bin/dtc".into()),
        limitations: vec![],
    });
    let output = rendered_text(&app, 120, 30);
    for expected in [
        "Kernel",
        "Configuration",
        "virtual/kernel",
        "linux-yocto.bb",
        ".config",
        "menuconfig available",
        "dtc available",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
}

#[test]
fn firmware_workspace_labels_detected_uboot_and_renders_device_trees() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::Firmware;
    app.firmware.view = yoctui_model::PlatformView::DeviceTrees;
    app.firmware.inventory = PlatformInventoryState::Available(yoctui_model::PlatformInventory {
        component: yoctui_model::PlatformComponent::UBoot,
        target: "u-boot-fslc".into(),
        provider: Some("/layers/u-boot-fslc.bb".into()),
        tasks: vec!["do_menuconfig".into()],
        roots: vec!["/work/u-boot".into()],
        files: vec![yoctui_model::PlatformFile {
            path: "/work/u-boot/board.dts".into(),
            root: "/work/u-boot".into(),
            kind: yoctui_model::PlatformFileKind::Dts,
            size_bytes: 84,
        }],
        dtc: Some("/usr/bin/dtc".into()),
        limitations: vec![],
    });
    let output = rendered_text(&app, 120, 30);
    for expected in ["U-Boot", "u-boot-fslc", "board.dts", "Device trees"] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
}

#[test]
fn overview_insights_render_all_eight_honest_responsive_states() {
    let mut app = App::new(32, 4_096);
    app.screen = Screen::Insights;
    app.focus = FocusTarget::Workspace;
    let expectations = [
        (
            yoctui_model::OverviewView::Timeline,
            "Build timeline / critical path",
        ),
        (
            yoctui_model::OverviewView::RebuildCauses,
            "Rebuild-cause graph",
        ),
        (
            yoctui_model::OverviewView::CacheAndDownloads,
            "Sstate & downloads",
        ),
        (yoctui_model::OverviewView::ImageSize, "Image-size treemap"),
        (
            yoctui_model::OverviewView::MetadataProvenance,
            "Metadata provenance graph",
        ),
        (
            yoctui_model::OverviewView::PackageTopology,
            "Runtime package dependency topology",
        ),
        (
            yoctui_model::OverviewView::SupplyChain,
            "CVE / license / SBOM overlay",
        ),
        (
            yoctui_model::OverviewView::DiskUsage,
            "Build disk-usage timeline",
        ),
    ];
    for (view, expected) in expectations {
        app.overview_view = view;
        for (width, height) in [(160, 50), (100, 30), (80, 24)] {
            let output = rendered_text_at(&app, width, height, UNIX_EPOCH);
            assert!(output.contains("Insights"), "{width}x{height}: {output}");
            assert!(
                output.contains(expected),
                "{view:?} {width}x{height}: {output}"
            );
            assert!(!output.contains('�'), "{view:?} {width}x{height}: {output}");
            for (index, tab) in yoctui_model::OverviewView::ALL.iter().enumerate() {
                assert!(
                    output.contains(&format!("{} {}", index + 1, tab.label())),
                    "{width}x{height}: {output}"
                );
            }
        }
    }

    app.overview_view = yoctui_model::OverviewView::CacheAndDownloads;
    app.workspace
        .variables
        .insert("SSTATE_DIR".into(), "/cache/sstate".into());
    app.workspace
        .variables
        .insert("DL_DIR".into(), "/cache/downloads".into());
    app.tasks.insert(
        yoctui_model::TaskId("setscene".into()),
        yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("setscene".into()),
            task: "do_package_setscene".into(),
            state: TaskState::Completed,
            ..yoctui_model::TaskInfo::default()
        },
    );
    let output = rendered_text_at(&app, 160, 50, UNIX_EPOCH);
    assert!(output.contains("/cache/sstate"), "{output}");
    assert!(output.contains("/cache/downloads"), "{output}");
    assert!(output.contains("Setscene outcomes"), "{output}");
    assert!(output.contains("offline readiness: unverified"), "{output}");
}
