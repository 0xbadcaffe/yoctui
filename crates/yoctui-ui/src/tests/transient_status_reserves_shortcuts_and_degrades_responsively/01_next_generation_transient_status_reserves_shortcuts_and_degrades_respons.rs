use super::*;

#[test]
fn next_generation_transient_status_reserves_shortcuts_and_degrades_responsively() {
    let mut app = App::new(32, 8192);
    app.focus = FocusTarget::Workspace;
    let notice = "Profile saved with a deliberately long status that must remain on one line";
    app.notification = Some(notice.into());
    let render_footer = |app: &App, width| {
        let mut terminal = Terminal::new(TestBackend::new(width, 3)).unwrap();
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
        assert!(footer.contains(&format!("i {notice}")), "{width}: {footer}");
        if width >= 130 {
            assert!(footer.contains("F1 Help"), "{width}: {footer}");
            assert!(footer.contains("F12 Menu"), "{width}: {footer}");
        } else {
            assert!(footer.contains("? Help"), "{width}: {footer}");
            assert!(footer.contains("Ctrl+P Menu"), "{width}: {footer}");
        }
        assert!(footer.contains("q Quit"), "{width}: {footer}");
        assert!(footer.contains("UTC 00:00:00"), "{width}: {footer}");
    }
    let narrow = render_footer(&app, 80);
    assert!(narrow.contains("i Profile saved"), "{narrow}");
    assert!(narrow.contains("? Help"), "{narrow}");
    assert!(narrow.contains("Ctrl+P Menu"), "{narrow}");
    assert!(narrow.contains("q Quit"), "{narrow}");
    assert!(!narrow.contains("UTC 00:00:00"), "{narrow}");

    app.notification = None;
    let idle = render_footer(&app, 160);
    assert!(!idle.contains("Profile saved"), "{idle}");
    assert!(idle.contains("F3 History"), "{idle}");
}

#[test]
fn transient_status_uses_the_row_above_shortcuts_and_daemon_waiting_is_braille() {
    let mut app = App::new(32, 8192);
    app.focus = FocusTarget::Workspace;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Connecting;

    let mut terminal = Terminal::new(TestBackend::new(120, 3)).unwrap();
    terminal
        .draw(|frame| workbench_footer(frame, &app, frame.area(), UNIX_EPOCH))
        .unwrap();
    let row = |y: u16| {
        let start = usize::from(y) * 120;
        terminal
            .backend()
            .buffer()
            .content
            [start..start + 120]
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };
    let status = row(0);
    let shortcuts = row(1);
    assert!(
        throbber_widgets_tui::BRAILLE_EIGHT_DOUBLE
            .symbols
            .iter()
            .any(|symbol| status.starts_with(symbol)),
        "{status}"
    );
    assert!(status.contains("BitBake connecting"), "{status}");
    assert!(!shortcuts.contains("BitBake connecting"), "{shortcuts}");
    assert!(shortcuts.contains("q Quit"), "{shortcuts}");
    assert!(shortcuts.contains("UTC 00:00:00"), "{shortcuts}");
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
    assert_eq!(clock_label(UNIX_EPOCH), "UTC 00:00:00");
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
