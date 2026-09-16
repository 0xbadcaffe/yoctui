//! Regression tests grouped around next_generation_transient_status_reserves_shortcuts_and_degrades_responsively.
use super::*;

#[test]
fn next_generation_transient_status_reserves_shortcuts_and_degrades_responsively() {
    let mut app = App::new(32, 8192);
    app.focus = FocusTarget::Workspace;
    app.notification =
        Some("Profile saved with a deliberately long status that must remain on one line".into());
    let render_footer = |app: &App, width| {
        let mut terminal = Terminal::new(TestBackend::new(width, 2)).unwrap();
        terminal
            .draw(|frame| workbench_footer(frame, app, frame.area(), UNIX_EPOCH))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };

    for width in [200_u16, 160, 130, 100] {
        let footer = render_footer(&app, width);
        assert!(footer.contains("i Profile saved"), "{width}: {footer}");
        assert!(footer.contains('…'), "{width}: {footer}");
        if width >= 160 {
            assert!(footer.contains("F1 Help"), "{width}: {footer}");
            assert!(footer.contains("F10 Menu"), "{width}: {footer}");
        } else {
            assert!(footer.contains("? Help"), "{width}: {footer}");
            assert!(footer.contains("Ctrl+P Menu"), "{width}: {footer}");
        }
        assert!(footer.contains("q Quit"), "{width}: {footer}");
        assert!(footer.contains("00:00:00"), "{width}: {footer}");
    }
    let narrow = render_footer(&app, 80);
    assert!(narrow.contains("i Profile saved"), "{narrow}");
    assert!(narrow.contains("? Help"), "{narrow}");
    assert!(narrow.contains("Ctrl+P Menu"), "{narrow}");
    assert!(narrow.contains("q Quit"), "{narrow}");
    assert!(!narrow.contains("00:00:00"), "{narrow}");

    app.notification = None;
    let idle = render_footer(&app, 160);
    assert!(!idle.contains("Profile saved"), "{idle}");
    assert!(idle.contains("F3 History"), "{idle}");
}

#[test]
fn next_generation_transient_status_maps_typed_priority_and_accessible_markers() {
    assert_eq!(
        transient_status_tone(TransientStatusKind::Error),
        StatusTone::Error
    );
    assert_eq!(
        transient_status_tone(TransientStatusKind::Confirmation),
        StatusTone::Warning
    );
    assert_eq!(
        transient_status_tone(TransientStatusKind::Success),
        StatusTone::Success
    );
    assert_eq!(
        transient_status_tone(TransientStatusKind::Notification),
        StatusTone::Info
    );
    assert_eq!(
        transient_status_tone(TransientStatusKind::Reconnecting),
        StatusTone::Pending
    );
    assert_eq!(
        transient_status_tone(TransientStatusKind::Activity),
        StatusTone::Running
    );

    let mut app = App::new(32, 8192);
    app.notification = Some("ordinary notice".into());
    app.dialogs.push_front(Dialog::QuitConfirmation);
    assert!(rendered_text(&app, 160, 30).contains("! Confirmation pending"));

    app.dialogs.clear();
    let _ = update(
        &mut app,
        Action::Failure(yoctui_model::AppError::new(
            "backend",
            "connection lost",
            "retry",
        )),
    );
    app.dialogs.push_front(Dialog::QuitConfirmation);
    let failure = rendered_text(&app, 160, 30);
    assert!(
        failure.contains("✕ backend: connection lost. retry"),
        "{failure}"
    );
    assert!(!failure.contains("Notice"), "{failure}");

    app.dialogs.clear();
    app.notification = None;
    app.build.status = BuildStatus::Idle;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Synchronizing;
    for (theme, color_enabled) in [
        (Theme::HighContrast, true),
        (Theme::Monochrome, true),
        (Theme::DarkPro, false),
    ] {
        app.theme = theme;
        app.color_enabled = color_enabled;
        app.reduced_motion = true;
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("… Daemon synchronizing"), "{output}");
        assert!(output.contains("? Help"), "{output}");
    }
}

#[test]
fn next_generation_search_line_is_bounded_counted_and_focus_explicit() {
    let mut app = App::new(32, 8192);
    let render_line = |app: &App, query: &str, editing, width| {
        search_line(
            app,
            query,
            editing,
            Some(1),
            3,
            SearchNavigation::Matches,
            SearchExit::Done,
            width,
        )
    };

    let wide = render_line(
        &app,
        "a deliberately long needle that must be bounded",
        true,
        160,
    );
    let wide_text = wide
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    assert!(wide_text.contains("/ Search [EDITING]"), "{wide_text}");
    assert!(wide_text.contains("Results: 2/3"), "{wide_text}");
    assert!(wide_text.contains("n/N next/previous"), "{wide_text}");
    assert!(wide_text.contains("Ctrl+U clear"), "{wide_text}");
    assert!(wide_text.contains("Enter/Esc done"), "{wide_text}");
    assert!(wide_text.contains('▏'), "{wide_text}");
    assert!(wide.width() <= 160);

    let medium = render_line(&app, "needle", false, 80);
    let medium_text = medium
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    assert!(medium_text.contains("[FILTERED]"), "{medium_text}");
    assert!(medium_text.contains("2/3"), "{medium_text}");
    assert!(medium_text.contains("Ctrl+U clear"), "{medium_text}");
    assert!(!medium_text.contains('▏'), "{medium_text}");
    assert!(medium.width() <= 80);

    let narrow = render_line(&app, "wide界query", true, 32);
    let narrow_text = narrow
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    assert!(narrow_text.contains("[EDITING]"), "{narrow_text}");
    assert!(narrow_text.contains("2/3"), "{narrow_text}");
    assert!(narrow_text.contains('▏'), "{narrow_text}");
    assert!(narrow.width() <= 32);

    app.theme = Theme::HighContrast;
    app.color_enabled = false;
    app.reduced_motion = true;
    let idle = search_line(
        &app,
        "",
        false,
        None,
        0,
        SearchNavigation::Results,
        SearchExit::Done,
        80,
    );
    let idle_text = idle
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    assert!(idle_text.contains("[IDLE]"), "{idle_text}");
    assert!(idle_text.contains("<empty>"), "{idle_text}");
    assert!(idle_text.contains("0/0"), "{idle_text}");
    assert!(!idle_text.contains("Ctrl+U clear"), "{idle_text}");
}

#[test]
fn next_generation_search_is_shared_by_every_typed_workspace() {
    let mut states = Vec::new();
    for screen in [Screen::Recipes, Screen::Layers, Screen::Configuration] {
        let mut app = App::new(32, 8192);
        app.screen = screen;
        app.focus = FocusTarget::Workspace;
        app.metadata_searching = true;
        app.metadata_query = "needle".into();
        states.push(app);
    }
    for screen in [
        Screen::Logs,
        Screen::Packages,
        Screen::Images,
        Screen::Sdk,
        Screen::Testing,
        Screen::Security,
        Screen::Qa,
        Screen::Compatibility,
    ] {
        let mut app = App::new(32, 8192);
        app.screen = screen;
        app.focus = FocusTarget::Workspace;
        match screen {
            Screen::Logs => {
                app.logs.searching = true;
                app.logs.query = "needle".into();
            }
            Screen::Packages => {
                app.package_searching = true;
                app.package_query = "needle".into();
            }
            Screen::Images => {
                app.image_artifact_searching = true;
                app.image_artifact_query = "needle".into();
            }
            Screen::Sdk => {
                app.sdk_artifact_searching = true;
                app.sdk_artifact_query = "needle".into();
            }
            Screen::Testing => {
                app.test_view = TestWorkspaceView::Results;
                app.test_result_searching = true;
                app.test_result_query = "needle".into();
            }
            Screen::Security => {
                app.security.searching = true;
                app.security.query = "needle".into();
            }
            Screen::Qa => {
                app.qa.searching = true;
                app.qa.query = "needle".into();
            }
            Screen::Compatibility => {
                app.compatibility_ui.searching = true;
                app.compatibility_ui.query = "needle".into();
            }
            _ => unreachable!(),
        }
        states.push(app);
    }
    let mut palette = App::new(32, 8192);
    palette.command_palette_open = true;
    palette.focus = FocusTarget::CommandPalette;
    palette.command_palette_query = "needle".into();
    states.push(palette);

    for mut app in states {
        app.theme = Theme::HighContrast;
        app.color_enabled = false;
        app.reduced_motion = true;
        let output = rendered_text(&app, 160, 40);
        assert!(output.contains("[EDITING]"), "{:?}: {output}", app.screen);
        assert!(output.contains("needle"), "{:?}: {output}", app.screen);
        assert!(output.contains("0/0"), "{:?}: {output}", app.screen);
        assert!(
            output.contains("Ctrl+U clear"),
            "{:?}: {output}",
            app.screen
        );
    }
}

#[test]
fn workbench_shell_clock_is_a_fixed_width_terminal_clock() {
    assert_eq!(clock_text(UNIX_EPOCH), "00:00:00");
    assert_eq!(
        clock_text(UNIX_EPOCH + Duration::from_secs(3_661)),
        "01:01:01"
    );
    assert_eq!(
        clock_text(UNIX_EPOCH + Duration::from_secs(90_061)),
        "01:01:01"
    );
}

#[test]
fn workbench_navigator_renders_grouped_hierarchy_and_full_row_selection() {
    let mut app = App::new(32, 8192);
    app.focus = FocusTarget::Navigator;
    let mut terminal = Terminal::new(TestBackend::new(180, 40)).unwrap();
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    for expected in ["OVERVIEW", "CONTENT", "BUILD", "VALIDATE", "TOOLS"] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
    let palette = ThemePalette::for_app(&app);
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .filter(|cell| cell.bg == palette.selection_background)
            .count()
            >= 18
    );
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| cell.fg == palette.warning)
    );
}

#[test]
fn workbench_navigator_scrolls_the_last_destination_into_view() {
    let mut app = App::new(32, 8192);
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = 24;
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("TOOLS"), "{output}");
    assert!(output.contains("Settings"), "{output}");
}

#[test]
fn next_generation_navigator_renders_authoritative_badges_and_collapsed_groups() {
    let mut app = App::new(32, 8192);
    app.focus = FocusTarget::Navigator;
    app.build.errors = 3;
    app.tasks.insert(
        yoctui_model::TaskId("busybox:do_compile".into()),
        yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("busybox:do_compile".into()),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            state: TaskState::Active,
            ..Default::default()
        },
    );
    let expanded = rendered_text(&app, 180, 40);
    assert!(expanded.contains("Tasks"), "{expanded}");
    assert!(expanded.contains("Tasks          1"), "{expanded}");
    assert!(expanded.contains("Errors         3"), "{expanded}");
    assert!(expanded.contains("Logs        LIVE"), "{expanded}");

    app.navigator_selection = 9;
    app.navigator_groups_expanded[2] = false;
    let collapsed = rendered_text(&app, 180, 40);
    assert!(collapsed.contains("▸ BUILD"), "{collapsed}");
    assert!(!collapsed.contains("Tasks          1"), "{collapsed}");
}

#[test]
fn next_generation_navigator_reports_bounded_scroll_position() {
    let mut app = App::new(32, 8192);
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = 24;
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("Navigator · 30/30 ↑"), "{output}");
    assert!(output.contains("Settings"), "{output}");
}

#[test]
fn security_workflow_renders_cve_identity_capability_partial_and_themes_responsively() {
    let mut app = security_workflow_ui_app();
    for (width, height, theme, color) in [
        (80, 24, Theme::Monochrome, false),
        (100, 30, Theme::WhiteClassic, true),
        (130, 30, Theme::MatrixGreen, true),
        (160, 40, Theme::HighContrast, true),
        (160, 40, Theme::DarkPro, true),
    ] {
        app.theme = theme;
        app.color_enabled = color;
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Security"), "{output}");
        assert!(output.contains("CVEs"), "{output}");
        assert!(output.contains("CVE-2026-0001"), "{output}");
        assert!(output.contains("vulnerable"), "{output}");
        assert!(output.contains("one malformed report"), "{output}");
    }

    app.focus = FocusTarget::Inspector;
    let inspector = rendered_text(&app, 160, 40);
    assert!(inspector.contains("Exact report"), "{inspector}");
    assert!(inspector.contains("cvefingerprint"), "{inspector}");
    assert!(
        inspector.contains("upstream-product=busybox"),
        "{inspector}"
    );
    assert!(inspector.contains("CVSS:3.1/AV:N"), "{inspector}");

    app.focus = FocusTarget::Workspace;
    app.color_enabled = false;
    let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
    terminal.draw(|frame| render(frame, &app)).unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| { cell.symbol() == "▶" && cell.modifier.contains(Modifier::REVERSED) }),
        "no-color selected finding must use reverse video"
    );
}

#[test]
fn security_workflow_renders_sbom_document_component_drill_and_limitations() {
    let mut app = security_workflow_ui_app();
    let spdx_identity = app
        .security
        .inventory
        .reports()
        .unwrap()
        .iter()
        .find_map(|report| match report {
            SecurityReport::Spdx(document) => Some(document.identity.clone()),
            _ => None,
        })
        .unwrap();
    app.security.view = SecurityView::Sbom;
    app.security.report_selection = Some(spdx_identity);
    for (width, height) in [(80, 24), (100, 30), (130, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("SBOM"), "{output}");
        assert!(output.contains("SPDX-2.3"), "{output}");
        assert!(output.contains("core-image-minimal"), "{output}");
    }

    app.security.drilled = true;
    app.security.component_selection = Some("SPDXRef-Package-busybox".into());
    let drilled = rendered_text(&app, 80, 24);
    assert!(drilled.contains("SPDXRef-Package-busybox"), "{drilled}");
    assert!(drilled.contains("GPL-2.0-only"), "{drilled}");

    app.focus = FocusTarget::Inspector;
    let inspector = rendered_text(&app, 160, 40);
    assert!(inspector.contains("https://example.invalid/spdx/image"));
    assert!(inspector.contains("SHA256=abcd1234"));
    assert!(inspector.contains("external references unavailable"));
    assert!(inspector.contains("Organization: Yocto"));
}

#[test]
fn security_workflow_renders_every_inventory_and_capability_state() {
    let mut app = security_workflow_ui_app();
    let request = app.security.inventory.request().unwrap().clone();
    for (state, expected) in [
        (SecurityInventoryState::NotLoaded, "Reports are not loaded"),
        (
            SecurityInventoryState::Loading {
                request: request.clone(),
            },
            "Loading report generation",
        ),
        (
            SecurityInventoryState::AvailableEmpty {
                request: request.clone(),
            },
            "available-empty",
        ),
        (
            SecurityInventoryState::Failed {
                request: request.clone(),
                message: "permission denied".into(),
            },
            "acquisition failed",
        ),
        (
            SecurityInventoryState::Cancelled {
                request: request.clone(),
            },
            "acquisition cancelled",
        ),
        (
            SecurityInventoryState::TimedOut {
                request: request.clone(),
            },
            "acquisition timed out",
        ),
        (
            SecurityInventoryState::Lost {
                request,
                message: "worker channel closed".into(),
            },
            "worker lost",
        ),
    ] {
        app.security.inventory = state;
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains(expected), "{expected}: {output}");
    }

    app.security.inventory = SecurityInventoryState::NotLoaded;
    for (capability, expected) in [
        (SecurityCapability::NotInspected, "not inspected"),
        (SecurityCapability::Inspecting, "inspection in progress"),
        (
            SecurityCapability::Failed("missing class".into()),
            "inspection failed",
        ),
    ] {
        app.security.capability = capability;
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains(expected), "{expected}: {output}");
    }
}

#[test]
fn security_workflow_renders_mapper_session_terminal_outcomes_and_bounded_output() {
    let mut app = security_workflow_ui_app();
    let request = app.security.inventory.request().unwrap().clone();
    app.security.inventory = SecurityInventoryState::AvailableEmpty { request };
    for status in [
        SecuritySessionStatus::Starting,
        SecuritySessionStatus::Running,
        SecuritySessionStatus::Cancelling,
        SecuritySessionStatus::Succeeded,
        SecuritySessionStatus::Failed,
        SecuritySessionStatus::Cancelled,
        SecuritySessionStatus::TimedOut,
        SecuritySessionStatus::Lost,
    ] {
        app.security.sessions = vec![security_session(status)];
        let output = rendered_text(&app, 100, 30);
        assert!(
            output.contains(security_session_status_label(status)),
            "{status:?}: {output}"
        );
        assert!(output.contains("mapped busybox"), "{output}");
        assert!(output.contains("[truncated]"), "{output}");
    }
}

#[test]
fn security_workflow_dialogs_render_exact_previews_at_all_breakpoints() {
    let mut app = security_workflow_ui_app();
    app.focus = FocusTarget::Dialog;
    let session = security_session(SecuritySessionStatus::Starting);
    app.dialogs
        .push_front(Dialog::Security(SecurityDialog::Operation(session.preview)));
    for (width, height) in [(80, 24), (100, 30), (130, 30), (160, 40)] {
        let output = rendered_text(&app, width, height);
        assert!(output.contains("Confirm CVE package mapping"), "{output}");
        assert!(output.contains("Exact indexed shell-free"), "{output}");
        assert!(output.contains("1: /build/tmp/log/cve"), "{output}");
    }

    app.dialogs.clear();
    let mut editor = yoctui_model::PopupEditor::new(format!(
        "# exact canonical non-symlink path\nroot = \"/reports/{}\"\n",
        "long-path-".repeat(80)
    ));
    editor.select_toml_value("root").unwrap();
    app.dialogs
        .push_front(Dialog::Security(SecurityDialog::Import {
            editor,
            validation_error: None,
        }));
    let import = rendered_text(&app, 80, 24);
    assert!(import.contains("Security import.toml"), "{import}");
    assert!(import.contains("Home/End line"), "{import}");
    assert!(import.contains("canonical non-symlink"), "{import}");

    app.dialogs.clear();
    app.dialogs
        .push_front(Dialog::Security(SecurityDialog::Cancellation(
            yoctui_model::SecuritySessionId(9),
        )));
    let cancellation = rendered_text(&app, 80, 24);
    assert!(
        cancellation.contains("Confirm Security cancellation"),
        "{cancellation}"
    );
    assert!(cancellation.contains("session 9 only"), "{cancellation}");
}

#[test]
fn semantic_theme_exposes_complete_role_catalog_for_every_theme() {
    for theme in [
        Theme::DarkPro,
        Theme::WhiteClassic,
        Theme::MatrixGreen,
        Theme::VscodeDark,
        Theme::VscodeLight,
        Theme::AccessibleDark,
        Theme::SoftLight,
        Theme::HighContrast,
        Theme::Monochrome,
    ] {
        let mut app = App::new(10, 1_000);
        app.theme = theme;
        let palette = ThemePalette::for_app(&app);
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|cell| cell.bg == palette.background)
        );
        if theme == Theme::Monochrome {
            assert!(palette.attribute_only);
            assert!(palette.focus().add_modifier.contains(Modifier::BOLD));
            assert!(palette.selected().add_modifier.contains(Modifier::REVERSED));
        } else {
            assert!(!palette.attribute_only);
            assert_ne!(palette.focused_border, palette.inactive_border);
            assert_ne!(palette.selection_background, palette.background);
            assert_ne!(palette.error, palette.success);
            assert_ne!(palette.warning, palette.informational);
            assert_eq!(palette.running, palette.progress);
            assert_eq!(palette.pending, palette.warning);
            assert_eq!(palette.graph_cpu, palette.graph_disk_read);
            assert_eq!(palette.graph_memory, palette.graph_disk_write);
            assert_eq!(palette.graph_network_rx, palette.success);
            assert_eq!(palette.graph_network_tx, palette.warning);
        }
    }
}

#[test]
fn semantic_theme_no_color_resets_colors_and_preserves_attributes() {
    let mut app = App::new(10, 1_000);
    app.theme = Theme::WhiteClassic;
    app.color_enabled = false;
    let palette = ThemePalette::for_app(&app);

    assert!(palette.attribute_only);
    for color in [
        palette.background,
        palette.primary_foreground,
        palette.secondary_foreground,
        palette.focused_border,
        palette.inactive_border,
        palette.selection_foreground,
        palette.selection_background,
        palette.success,
        palette.warning,
        palette.error,
        palette.running,
        palette.pending,
        palette.accent,
        palette.muted,
        palette.progress,
        palette.graph_cpu,
        palette.graph_memory,
        palette.graph_disk_read,
        palette.graph_disk_write,
        palette.graph_network_rx,
        palette.graph_network_tx,
        palette.disabled,
        palette.informational,
        palette.heading,
        palette.table_header,
    ] {
        assert_eq!(color, Color::Reset);
    }
    assert!(palette.focus().add_modifier.contains(Modifier::BOLD));
    assert!(
        selected_style(&app, true)
            .add_modifier
            .contains(Modifier::REVERSED)
    );
    assert!(
        severity_style(&app, Severity::Error)
            .add_modifier
            .contains(Modifier::UNDERLINED)
    );
    assert_ne!(
        severity_style(&app, Severity::Warning).add_modifier,
        severity_style(&app, Severity::Trace).add_modifier
    );

    app.focus = FocusTarget::Navigator;
    let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
    terminal.draw(|frame| render(frame, &app)).unwrap();
    assert!(terminal.backend().buffer().content.iter().any(|cell| {
        cell.modifier.contains(Modifier::REVERSED) || cell.modifier.contains(Modifier::BOLD)
    }));
}

#[test]
fn theme_light_shell_and_dialog_apply_the_semantic_background() {
    let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    app.theme = Theme::WhiteClassic;
    app.dialogs.push_back(Dialog::BuildOptions);
    terminal.draw(|frame| render(frame, &app)).unwrap();

    assert_eq!(
        ThemePalette::for_app(&app).background,
        Color::Rgb(248, 248, 248)
    );
    assert_eq!(
        ThemePalette::for_app(&app).informational,
        Color::Rgb(0, 107, 107)
    );
}

#[test]
fn theme_picker_renders_named_choices_and_immediate_apply_hint() {
    let mut app = App::new(10, 1_000);
    app.dialogs.push_back(Dialog::ThemePicker {
        selection: 1,
        original_theme: Theme::DarkPro,
        original_color_enabled: true,
        original_settings_dirty: false,
    });
    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("Theme — applies immediately"));
    assert!(output.contains("White"));
    assert!(output.contains("Dark gray"));
    assert!(!output.to_ascii_lowercase().contains("vscode"));
    assert!(output.contains("Enter apply"));
}

#[test]
fn theme_progress_and_log_severity_use_semantic_roles() {
    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.theme = Theme::HighContrast;
    app.tasks.insert(
        yoctui_model::TaskId("busybox:do_compile".into()),
        yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("busybox:do_compile".into()),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: Some(50),
            ..yoctui_model::TaskInfo::default()
        },
    );
    terminal.draw(|frame| render(frame, &app)).unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| cell.fg == Color::Rgb(50, 255, 100))
    );

    app.screen = Screen::Logs;
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "compile failed".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: None,
        protected: false,
        diagnostic: None,
    });
    terminal.draw(|frame| render(frame, &app)).unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| cell.fg == Color::Rgb(255, 70, 70))
    );
}

#[test]
fn settings_workspace_renders_typed_rows_and_controls_on_narrow_terminals() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Settings;
    app.settings_selection = 5;
    app.settings_dirty = true;
    app.theme = Theme::MatrixGreen;
    app.animation_speed = yoctui_model::AnimationSpeed::Slow;
    app.reduced_motion = true;
    app.logs.follow = false;

    let output = rendered_text(&app, 100, 30);
    assert!(output.contains("Settings (not saved)"));
    assert!(output.contains("Theme"));
    assert!(output.contains("Green"));
    assert!(!output.to_ascii_lowercase().contains("vscode"));
    assert!(output.contains("Animation speed"));
    assert!(output.contains("Reduced motion"));
    assert!(output.contains("Log wrap"));
    assert!(output.contains("Log follow"));
    assert!(output.contains("Keybindings"));
    assert!(output.contains("select"));
    assert!(output.contains("change"));
}

#[test]
fn clone_progress_is_visible_across_terminal_widths() {
    let mut app = App::new(32, 8192);
    update(
        &mut app,
        Action::SetBackgroundActivity {
            activity: yoctui_model::BackgroundActivity::Cloning,
            active: true,
        },
    );
    for width in [80, 100, 160] {
        let mut terminal = Terminal::new(TestBackend::new(width, 2)).unwrap();
        terminal
            .draw(|frame| workbench_footer(frame, &app, frame.area(), UNIX_EPOCH))
            .unwrap();
        let text = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(text.contains("Cloning…"), "{width}: {text}");
        assert!(
            text.chars().any(|c| ('\u{2800}'..='\u{28ff}').contains(&c)),
            "{text}"
        );
    }
}
