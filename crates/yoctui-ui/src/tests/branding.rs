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
fn portable_branding_help_and_about_are_distinct_at_all_demo_sizes() {
    let mut app = App::new(16, 4096);
    app.screen = Screen::Help;
    app.focus = FocusTarget::Workspace;
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let text = rendered_text_at(&app, width, height, literal_now());
        assert!(text.contains("Operator guide"), "{text}");
        assert!(text.contains("About Yoctui"), "{text}");
        assert!(!text.contains('█'), "{text}");
        assert!(!text.contains("Build SHA:"), "{text}");
        update(&mut app, Action::OpenAbout);
        let about = rendered_text_at(&app, width, height, literal_now());
        assert!(about.contains("About Yoctui"), "{about}");
        assert!(about.contains("yoctui v"), "{about}");
        assert!(about.contains("Yocto development workbench"), "{about}");
        assert!(about.contains("Build SHA:"), "{about}");
        assert!(about.contains('█'), "{about}");
        assert!(!about.contains("Keyboard and action reference"), "{about}");
        assert!(!about.contains("Connecting to daemon..."), "{about}");
        update(&mut app, Action::Open(Screen::Help));
    }
}

#[test]
fn operator_guide_shows_current_steps_and_real_picker_inventory_responsively() {
    for (width, height) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let mut app = App::new(16, 4096);
        app.workspace.recipes = vec![yoctui_model::Recipe {
            name: "core-image-minimal".into(),
            ..Default::default()
        }];
        update(&mut app, Action::OpenOnboarding);
        update(&mut app, Action::AdvanceOnboarding);
        let guide = rendered_text_at(&app, width, height, literal_now());
        assert!(guide.contains("1/6 completed"), "{guide}");
        assert!(guide.contains("Select a target"), "{guide}");
        assert!(guide.contains("n complete current"), "{guide}");
        update(&mut app, Action::ActivateOnboardingStep);
        let picker = rendered_text_at(&app, width, height, literal_now());
        assert!(picker.contains("Available image targets"), "{picker}");
        assert!(picker.contains("core-image-minimal"), "{picker}");
        assert!(!picker.contains("No image recipes"), "{picker}");
        update(&mut app, Action::CancelImagePicker);
        assert!(
            rendered_text_at(&app, width, height, literal_now())
                .contains("Six-step workbench path")
        );
    }
}
