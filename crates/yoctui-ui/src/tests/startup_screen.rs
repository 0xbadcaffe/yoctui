use super::*;

#[test]
fn startup_screen_renders_restored_destination_in_workspace_and_navigator() {
    for (screen, inspector) in [
        (Screen::Kernel, "Destination: Kernel"),
        (Screen::BuildEnvironment, "Destination: Build Environment"),
        (Screen::Dashboard, "Project Inspector"),
    ] {
        let mut app = App::new(16, 4096);
        app.restore_startup_screen(screen);
        for (width, height) in [(160, 50), (100, 30), (80, 24)] {
            let text = rendered_text_at(&app, width, height, literal_now());
            assert!(text.contains("Navigator"), "{width}x{height}: {text}");
            if width == 160 {
                assert!(text.contains(inspector), "{text}");
            }
            assert_eq!(app.screen, screen);
            assert_eq!(app.navigator_screen(), screen);
            assert_eq!(app.focus, yoctui_model::FocusTarget::Navigator);
        }
    }
}
