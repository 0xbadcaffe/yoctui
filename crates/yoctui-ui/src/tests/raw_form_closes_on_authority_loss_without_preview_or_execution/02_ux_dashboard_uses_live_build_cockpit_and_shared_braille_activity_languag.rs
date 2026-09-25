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
fn platform_menuconfig_terminal_renders_in_place_and_keeps_waiting_activity_visible() {
    let mut waiting = App::new(16, 4_096);
    waiting.screen = Screen::Kernel;
    waiting.kernel.menuconfig_terminal = yoctui_model::PlatformTerminalState {
        name: Some("kernel menuconfig".into()),
        ..yoctui_model::PlatformTerminalState::default()
    };
    let waiting_output = rendered_text(&waiting, 120, 35);
    assert!(
        waiting_output.contains("Starting Kernel menuconfig"),
        "{waiting_output}"
    );

    let mut app = ux_terminal_render_fixture();
    app.screen = Screen::Kernel;
    app.pane_layout =
        yoctui_model::PaneLayout::new(yoctui_model::PaneId(1)).expect("valid single pane");
    app.daemon.pty_sessions[0].name = "kernel menuconfig".into();
    app.daemon.pty_details[0].kind = yoctui_model::ClientDaemonPtyKind::Menuconfig;
    app.kernel.menuconfig_terminal = yoctui_model::PlatformTerminalState {
        name: Some("kernel menuconfig".into()),
        session_id: Some(app.daemon.pty_sessions[0].id),
        ..yoctui_model::PlatformTerminalState::default()
    };
    let output = rendered_text(&app, 160, 50);
    assert!(output.contains("Kernel menuconfig"), "{output}");
    assert!(output.contains("shell: bounded output"), "{output}");
    assert!(output.contains("Ctrl+B prefix"), "{output}");
    assert!(!output.contains("Inspector:"), "{output}");
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
