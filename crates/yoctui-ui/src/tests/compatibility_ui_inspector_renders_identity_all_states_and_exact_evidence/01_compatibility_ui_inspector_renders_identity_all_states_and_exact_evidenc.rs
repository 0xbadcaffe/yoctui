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
        "F12 Menu",
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
