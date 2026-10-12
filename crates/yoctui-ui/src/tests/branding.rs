use super::*;

#[test]
fn portable_branding_startup_is_responsive_and_does_not_change_app_state() {
    for (width, height) in [
        (240, 80),
        (160, 50),
        (100, 30),
        (80, 24),
        (40, 12),
        (24, 8),
        (1, 1),
        (0, 0),
    ] {
        let app = App::new(16, 4096);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render_startup(frame, &app)).unwrap();
        let text = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        if width >= 24 && height >= 8 {
            assert!(text.contains("yoctui v"), "{width}x{height}: {text}");
            assert!(
                text.contains("Connecting to daemon..."),
                "{width}x{height}: {text}"
            );
        }
        if width >= 42 && height >= 12 {
            assert!(text.contains('█'), "{text}");
        } else {
            assert!(!text.contains('█'), "{text}");
        }
        assert_eq!(app.screen, Screen::Dashboard);
        assert_eq!(app.focus, FocusTarget::Navigator);
    }
}

#[test]
fn portable_branding_respects_ascii_monochrome_and_light_themes() {
    for theme in [Theme::DarkPro, Theme::WhiteClassic, Theme::MatrixGreen] {
        for color in [true, false] {
            let mut app = App::new(16, 4096);
            app.theme = theme;
            app.color_enabled = color;
            app.preferences.symbols = SymbolPreference::Ascii;
            let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
            terminal.draw(|frame| render_startup(frame, &app)).unwrap();
            let cells = &terminal.backend().buffer().content;
            assert!(cells.iter().any(|cell| cell.symbol() == "#"));
            assert!(!cells.iter().any(|cell| cell.symbol() == "█"));
            if !color {
                assert!(
                    cells
                        .iter()
                        .all(|cell| cell.fg == Color::Reset && cell.bg == Color::Reset)
                );
            }
        }
    }
}

#[test]
fn portable_branding_help_keeps_guide_and_about_visible() {
    let mut app = App::new(16, 4096);
    app.screen = Screen::Help;
    app.focus = FocusTarget::Workspace;
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let text = rendered_text_at(&app, width, height, literal_now());
        assert!(text.contains("Operator guide"), "{text}");
        assert!(text.contains("About Yoctui"), "{text}");
        assert_eq!(text.contains('█'), width == 160, "{text}");
    }
}
