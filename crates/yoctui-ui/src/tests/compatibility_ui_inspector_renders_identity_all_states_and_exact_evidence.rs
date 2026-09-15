//! Regression tests grouped around compatibility_ui_inspector_renders_identity_all_states_and_exact_evidence.
use super::*;

#[test]
fn compatibility_ui_inspector_renders_identity_all_states_and_exact_evidence() {
    let app = compatibility_ui_inspector_app();
    let all = rendered_text(&app, 180, 44);
    for expected in [
        "Environment / Compatibility",
        "generation 7",
        "Degraded",
        "/work/poky/build",
        "wrynose 6.0",
        "2.18.0",
        "Available 1",
        "Limited 1",
        "Unavailable 1",
        "Unknown 1",
        "Unsupported 1",
        "bitbake.build",
        "bitbake.getvar",
        "devtool.upgrade",
        "resulttool",
        "git_archive",
        "F10 Menu",
    ] {
        assert!(all.contains(expected), "missing {expected}: {all}");
    }

    let mut unavailable = app.clone();
    let _ = update(
        &mut unavailable,
        Action::SetCompatibilityFilter(CompatibilityUiFilter::Unavailable),
    );
    unavailable.focus = FocusTarget::Inspector;
    let details = rendered_text(&unavailable, 180, 44);
    for expected in [
        "Capability: devtool.upgrade",
        "State: Unavailable",
        "probe.subcommand_absent",
        "Current Devtool does not expose the upgrade subcommand.",
        "Requirement: devtool upgrade",
        "DirectProbe / Negative",
        "argv: devtool --help",
    ] {
        assert!(details.contains(expected), "missing {expected}: {details}");
    }
}

#[test]
fn compatibility_ui_inspector_responsive_absent_themes_and_no_color_are_safe() {
    let app = compatibility_ui_inspector_app();
    for (width, focus) in [
        (180, FocusTarget::Workspace),
        (100, FocusTarget::Workspace),
        (80, FocusTarget::Workspace),
    ] {
        let mut responsive = app.clone();
        responsive.focus = focus;
        let output = rendered_text(&responsive, width, 30);
        assert!(
            output.contains("Compatibility") || output.contains("Capability:"),
            "{width}: {output}"
        );
    }
    assert!(rendered_text(&app, 79, 23).contains("needs at least 80x24"));

    for theme in [
        Theme::DarkPro,
        Theme::WhiteClassic,
        Theme::MatrixGreen,
        Theme::HighContrast,
    ] {
        let mut themed = app.clone();
        themed.theme = theme;
        themed.color_enabled = false;
        let output = rendered_text(&themed, 130, 30);
        assert!(output.contains("Available"), "{theme:?}: {output}");
    }

    let mut absent = App::new(32, 8192);
    absent.screen = Screen::Compatibility;
    absent.focus = FocusTarget::Workspace;
    absent.daemon.status = yoctui_model::ClientReplicaStatus::Stale;
    let output = rendered_text(&absent, 180, 34);
    assert!(output.contains("snapshot unavailable"), "{output}");
    assert!(output.contains("stale"), "{output}");
    assert!(
        output.contains("No probe is run by this client"),
        "{output}"
    );
}

#[test]
fn compatibility_ui_inspector_is_discoverable_without_changing_tasks_golden() {
    let mut app = App::new(32, 8192);
    let navigator = rendered_text(&app, 180, 40);
    assert!(navigator.contains("Compatibility"), "{navigator}");
    app.command_palette_open = true;
    app.command_palette_query = "compatibility".into();
    let palette = rendered_text(&app, 120, 30);
    assert!(palette.contains("Open Compatibility"), "{palette}");
    assert!(palette.contains("environment identity"), "{palette}");
}

#[test]
fn compatibility_ui_nav_actions_render_state_reason_and_fallback_from_one_snapshot() {
    let mut app = compatibility_ui_inspector_app();
    app.screen = Screen::Configuration;
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = 12;
    let navigator = rendered_text(&app, 180, 42);
    for expected in [
        "~ Configuration",
        "Inspector: Navigator",
        "Destination: Configuration",
        "Compatibility: Limited",
        "Native getvar is absent; environment dump fallback selected.",
        "bitbake.getvar.environment-fallback",
    ] {
        assert!(
            navigator.contains(expected),
            "missing {expected}: {navigator}"
        );
    }

    app.focus = FocusTarget::CommandPalette;
    app.command_palette_open = true;
    app.command_palette_query = "Open Configuration".into();
    let palette = rendered_text(&app, 120, 30);
    for expected in [
        "Open Configuration",
        "Compatibility: Limited",
        "Reason: Native getvar is absent",
        "Implementation: bitbake.getvar.environment-fallback",
    ] {
        assert!(palette.contains(expected), "missing {expected}: {palette}");
    }
}

#[test]
fn compatibility_ui_nav_actions_keep_navigation_local_and_gate_operations() {
    let mut app = App::new(32, 8192);
    app.screen = Screen::Logs;
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = 2;
    let navigator = rendered_text(&app, 180, 36);
    assert!(navigator.contains("Layers"), "{navigator}");
    assert!(!navigator.contains("? Layers"), "{navigator}");
    assert!(navigator.contains("Compatibility: Unknown"), "{navigator}");
    assert!(
        navigator.contains("No current environment capability snapshot"),
        "{navigator}"
    );

    app.focus = FocusTarget::CommandPalette;
    app.command_palette_open = true;
    app.command_palette_query = "Build image".into();
    let palette = rendered_text(&app, 120, 30);
    assert!(palette.contains("Compatibility: Unknown"), "{palette}");
    assert!(palette.contains("Cannot run:"), "{palette}");
    assert!(
        palette.contains("No current environment capability snapshot"),
        "{palette}"
    );

    app.command_palette_query = "Open Layers".into();
    let discoverable = rendered_text(&app, 120, 30);
    assert!(
        discoverable.contains("Compatibility: Unknown"),
        "{discoverable}"
    );
    assert!(!discoverable.contains("Cannot run:"), "{discoverable}");
}

#[test]
fn compatibility_ui_workspace_actions_render_exact_states_reasons_and_local_paths() {
    let mut app = compatibility_ui_inspector_app();
    app.screen = Screen::Configuration;
    app.focus = FocusTarget::Inspector;
    let configuration = rendered_text(&app, 180, 50);
    for expected in [
        "CONTEXTUAL ACTIONS",
        "Refresh effective variables",
        "[r] — Limited",
        "Limited",
        "Native getvar is absent; environment dump fallback selected.",
        "bitbake.getvar.environment-fallback",
        "Inspect/copy/source",
        "[Enter/C/U/o] — Local",
        "Local",
    ] {
        assert!(
            configuration.contains(expected),
            "missing {expected}: {configuration}"
        );
    }

    app.focus = FocusTarget::Navigator;
    app.navigator_selection = 19;
    let devtool = rendered_text(&app, 180, 58);
    for expected in [
        "Destination: Devtool",
        "Upgrade recipe",
        "[U] — Unavailable",
        "Unavailable",
        "Current Devtool does not expose the upgrade subcommand.",
    ] {
        assert!(devtool.contains(expected), "missing {expected}: {devtool}");
    }
}

#[test]
fn compatibility_dynamic_ui_workspace_actions_replace_without_stale_widget_state() {
    let mut app = compatibility_ui_inspector_app();
    app.screen = Screen::Configuration;
    app.focus = FocusTarget::Inspector;
    let limited = rendered_text(&app, 180, 46);
    assert!(limited.contains("Limited"), "{limited}");
    assert!(limited.contains("environment-fallback"), "{limited}");

    yoctui_model::invalidate_workspace_compatibility(&mut app);
    let unknown = rendered_text(&app, 180, 46);
    assert!(unknown.contains("[r] — Unknown"), "{unknown}");
    assert!(
        unknown.contains("No current environment capability snapshot"),
        "{unknown}"
    );
    assert!(!unknown.contains("environment-fallback"), "{unknown}");

    app.screen = Screen::Images;
    let images = rendered_text(&app, 180, 60);
    assert!(images.contains("Launch QEMU"), "{images}");
    assert!(images.contains("[Q] — Unknown"), "{images}");
    assert!(
        images.contains("Write selected local device") && images.contains("[D] — Local"),
        "{images}"
    );
    assert!(
        images.contains("Cancel owned image operation") && images.contains("[x/c] — Local"),
        "{images}"
    );
}

#[test]
fn compatibility_dynamic_ui_replaces_action_state_reason_and_preserves_selection() {
    let mut app = compatibility_ui_inspector_app();
    app.screen = Screen::Configuration;
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = 19;
    let unavailable = rendered_text(&app, 180, 56);
    assert!(unavailable.contains("Upgrade recipe"), "{unavailable}");
    assert!(unavailable.contains("[U] — Unavailable"), "{unavailable}");
    assert!(
        unavailable.contains("Current Devtool does not expose the upgrade subcommand."),
        "{unavailable}"
    );

    let mut authority = app.workspace_compatibility.authority().unwrap().clone();
    authority.snapshot.generation = 8;
    let record = authority
        .snapshot
        .capabilities
        .iter_mut()
        .find(|record| record.id == yoctui_model::CapabilityId::DevtoolUpgrade)
        .unwrap();
    record.state = yoctui_model::CapabilityState::Available;
    record.evidence[0].outcome = yoctui_model::CapabilityEvidenceOutcome::Positive;
    authority.implementations.insert(
        yoctui_model::CapabilityId::DevtoolUpgrade,
        yoctui_model::CapabilityImplementation {
            id: "devtool.upgrade.argv".into(),
            kind: yoctui_model::CapabilityImplementationKind::Command,
        },
    );
    yoctui_model::install_workspace_compatibility(&mut app, authority).unwrap();
    assert_eq!(app.navigator_selection, 19);
    let available = rendered_text(&app, 180, 56);
    assert!(available.contains("Upgrade recipe"), "{available}");
    assert!(available.contains("[U] — Available"), "{available}");
    assert!(available.contains("Available"), "{available}");
    assert!(available.contains("devtool.upgrade.argv"), "{available}");
    assert!(
        !available.contains("Current Devtool does not expose the upgrade subcommand."),
        "{available}"
    );

    let mut replacement = app.workspace_compatibility.authority().unwrap().clone();
    replacement.snapshot.generation = 9;
    let record = replacement
        .snapshot
        .capabilities
        .iter_mut()
        .find(|record| record.id == yoctui_model::CapabilityId::DevtoolUpgrade)
        .unwrap();
    record.state = yoctui_model::CapabilityState::Unavailable {
        reason: yoctui_model::CapabilityReason::new(
            "probe.subcommand_removed",
            "The reconnected Devtool omits upgrade.",
            Some("Required command: devtool upgrade".into()),
        )
        .unwrap(),
    };
    record.evidence[0].outcome = yoctui_model::CapabilityEvidenceOutcome::Negative;
    replacement
        .implementations
        .remove(&yoctui_model::CapabilityId::DevtoolUpgrade);
    yoctui_model::install_workspace_compatibility(&mut app, replacement).unwrap();
    assert_eq!(app.navigator_selection, 19);
    let replaced = rendered_text(&app, 180, 56);
    assert!(
        replaced.contains("The reconnected Devtool omits upgrade."),
        "{replaced}"
    );
    assert!(!replaced.contains("devtool.upgrade.argv"), "{replaced}");
}

#[test]
fn compatibility_dynamic_ui_dialog_actions_render_all_states_and_exact_authority() {
    let reason = |code: &str, message: &str| {
        yoctui_model::CapabilityReason::new(code, message, Some("bitbake <target>".into())).unwrap()
    };
    let cases = [
        (
            yoctui_model::CapabilityState::Available,
            yoctui_model::CapabilityEvidenceOutcome::Positive,
            true,
            "State: Available · Confirmation available",
        ),
        (
            yoctui_model::CapabilityState::AvailableWithLimitations {
                reason: reason(
                    "compatibility.fallback",
                    "Build uses the maintained command fallback.",
                ),
                limitations: vec!["Native event progress is unavailable.".into()],
            },
            yoctui_model::CapabilityEvidenceOutcome::Positive,
            true,
            "State: Limited · Confirmation available",
        ),
        (
            yoctui_model::CapabilityState::Unavailable {
                reason: reason("probe.command_absent", "BitBake build is unavailable."),
            },
            yoctui_model::CapabilityEvidenceOutcome::Negative,
            false,
            "State: Unavailable · Confirmation disabled",
        ),
        (
            yoctui_model::CapabilityState::Unknown {
                reason: reason("probe.timed_out", "BitBake build probe timed out."),
            },
            yoctui_model::CapabilityEvidenceOutcome::Inconclusive,
            false,
            "State: Unknown · Confirmation disabled",
        ),
        (
            yoctui_model::CapabilityState::Unsupported {
                reason: reason(
                    "yoctui.not_implemented",
                    "No maintained build adapter exists.",
                ),
            },
            yoctui_model::CapabilityEvidenceOutcome::Inconclusive,
            false,
            "State: Unsupported · Confirmation disabled",
        ),
    ];
    for (state, outcome, keep_implementation, expected) in cases {
        let mut app = compatibility_ui_inspector_app();
        let mut authority = app.workspace_compatibility.authority().unwrap().clone();
        authority.snapshot.generation = 8;
        let record = authority
            .snapshot
            .capabilities
            .iter_mut()
            .find(|record| record.id == yoctui_model::CapabilityId::BitBakeBuild)
            .unwrap();
        record.state = state;
        record.evidence[0].outcome = outcome;
        if matches!(
            record.state,
            yoctui_model::CapabilityState::Unsupported { .. }
        ) {
            record.evidence.clear();
        }
        if !keep_implementation {
            authority
                .implementations
                .remove(&yoctui_model::CapabilityId::BitBakeBuild);
        }
        yoctui_model::install_workspace_compatibility(&mut app, authority).unwrap();
        app.dialogs.push_front(Dialog::BuildOptions);
        app.focus = FocusTarget::Dialog;
        let output = rendered_text(&app, 120, 30);
        assert!(output.contains("Dialog compatibility"), "{output}");
        assert!(output.contains(expected), "missing {expected}: {output}");
        if expected.contains("Limited") {
            assert!(
                output.contains("Limitation: Native event progress is unavailable."),
                "{output}"
            );
        }
        if keep_implementation {
            assert!(output.contains("bitbake.build.command"), "{output}");
        }
    }

    let mut local = App::new(32, 8192);
    local.dialogs.push_front(Dialog::QuitConfirmation);
    local.focus = FocusTarget::Dialog;
    let output = rendered_text(&local, 80, 24);
    assert!(output.contains("Confirm exit"), "{output}");
    assert!(!output.contains("Confirmation disabled"), "{output}");
}

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
