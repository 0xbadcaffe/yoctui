use super::*;

#[test]
fn daemon_manager_renders_health_logs_review_and_configuration_at_boundary_sizes() {
    let mut app = App::new(100, 100_000);
    app.screen = Screen::Daemons;
    app.focus = FocusTarget::Workspace;
    app.daemon_manager.details = vec![
        "Host: demo-host".into(),
        "Local IP addresses: 192.0.2.1".into(),
    ];
    app.daemon_manager.build_directory = "/build".into();
    app.daemon_manager.source_directory = "/source".into();
    app.daemon_manager.message = Some("Stop completed".into());
    app.daemon_manager.logs = vec!["daemon discovery: build identity ready".into()];
    for (width, height) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("Host: demo-host"), "{width}x{height}: {text}");
        assert!(text.contains("Stop completed"), "{width}x{height}: {text}");
        assert!(text.contains("1 Health / activity"));
        app.daemon_manager.logs_visible = true;
        assert!(rendered_text(&app, width, height).contains("daemon discovery:"));
        app.daemon_manager.logs_visible = false;
        app.daemon_manager.editing = true;
        assert!(rendered_text(&app, width, height).contains("Configure local daemon"));
        app.daemon_manager.editing = false;
        app.daemon_manager.review = Some(yoctui_model::DaemonControl::Stop);
        let text = rendered_text(&app, width, height);
        assert!(text.contains("Daemon control review"));
        assert!(text.contains("Esc cancel"));
        app.daemon_manager.review = None;
    }
}

#[test]
fn daemon_lines_distinguish_activity_recovery_and_journal_fields_without_changing_text() {
    use crate::daemon_manager_render::styled_daemon_line;
    for theme in [
        Theme::DarkPro,
        Theme::WhiteClassic,
        Theme::HighContrast,
        Theme::Monochrome,
    ] {
        let palette = ThemePalette::for_theme(theme, true);
        for (text, color) in [
            ("Recovery: old terminal classified Lost", palette.warning),
            (
                "Recent activity: workspace refreshed",
                palette.informational,
            ),
        ] {
            let line = styled_daemon_line(text, false, &palette);
            assert_eq!(line.to_string(), text);
            assert_eq!(line.spans[0].style, palette.role(color, Modifier::BOLD));
        }
        let text = "2026-10-10T08:06:40+02:00 demo-host yoctui[123]: WARNING: retrying λ";
        let line = styled_daemon_line(text, true, &palette);
        assert_eq!(line.to_string(), text);
        assert_eq!(line.spans.len(), 7);
        for (index, color, modifier) in [
            (0, palette.secondary_foreground, Modifier::empty()),
            (2, palette.informational, Modifier::BOLD),
            (4, palette.accent, Modifier::BOLD),
            (6, palette.warning, Modifier::empty()),
        ] {
            assert_eq!(line.spans[index].style, palette.role(color, modifier));
        }
        for (message, color) in [
            ("ERROR: failed", palette.error),
            ("warning: retry", palette.warning),
            ("DEBUG: probe", palette.secondary_foreground),
            ("workspace ready", palette.success),
            ("ordinary output", palette.primary_foreground),
        ] {
            let text = format!("2026-10-10T08:06:40+02:00 host systemd[1]: {message}");
            let line = styled_daemon_line(&text, true, &palette);
            assert_eq!(line.to_string(), text);
            assert_eq!(line.spans[6].style, palette.role(color, Modifier::empty()));
        }
        for text in [
            "",
            "  continuation λ",
            "-- No entries --",
            "2026-10-10T08:06:40+02:00 malformed",
        ] {
            assert_eq!(styled_daemon_line(text, true, &palette).to_string(), text);
        }
    }
    let palette = ThemePalette::for_theme(Theme::DarkPro, false);
    let line = styled_daemon_line("Recovery: preserved work", false, &palette);
    assert_eq!(line.to_string(), "Recovery: preserved work");
    assert_eq!(line.spans[0].style.fg, Some(Color::Reset));
    assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn daemon_line_colors_survive_rendering_and_vertical_scrolling() {
    let mut app = App::new(100, 100_000);
    app.screen = Screen::Daemons;
    app.focus = FocusTarget::Workspace;
    app.daemon_manager.logs_visible = true;
    let timestamp = "2026-10-10T08:06:40+02:00";
    app.daemon_manager.logs = (0..100)
        .map(|i| format!("{timestamp} demo-host yoctui[123]: WARNING: line {i}"))
        .collect();
    let palette = ThemePalette::for_app(&app);
    for scroll in [0, 1, 100] {
        app.daemon_manager.scroll = scroll;
        let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let buffer = terminal.backend().buffer();
        let row = (0..48)
            .find(|y| {
                (0..160)
                    .map(|x| buffer[(x, *y)].symbol())
                    .collect::<String>()
                    .contains(timestamp)
            })
            .unwrap();
        let text = (0..160)
            .map(|x| buffer[(x, row)].symbol())
            .collect::<String>();
        for (part, color) in [
            (timestamp, palette.secondary_foreground),
            ("demo-host", palette.informational),
            ("yoctui[123]", palette.accent),
            ("WARNING", palette.warning),
        ] {
            let x = text[..text.find(part).unwrap()].chars().count() as u16;
            assert_eq!(buffer[(x, row)].fg, color, "{part} at scroll {scroll}");
        }
        if scroll == 100 {
            assert!(rendered_text(&app, 160, 48).contains("line 99"));
        }
    }
}
