use super::*;

#[test]
fn editor_preferences_form_shows_command_placement_and_validation_at_narrow_sizes() {
    let mut app = App::new(20, 1000);
    app.screen = Screen::Settings;
    update(
        &mut app,
        Action::EditorSettings(yoctui_model::EditorSettingsAction::Begin),
    );
    app.editor_settings.error = Some("Invalid command".into());
    assert!(footer_shortcuts(&app).contains("Tab placement"));
    assert!(!footer_shortcuts(&app).contains("q quit"));
    for (width, height) in [(80, 24), (100, 30), (160, 48)] {
        let rail = footer_rail_shortcuts(&app, width);
        assert!(rail.contains("Enter save") && rail.contains("Tab placement"));
        assert!(!rail.contains("Tab Focus") && !rail.contains("F12"));
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| settings_workspace(frame, &app, frame.area()))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        for expected in [
            "Editor command",
            "builtin",
            "Enter save",
            "Embedded Yoctui terminal",
            "Invalid command",
        ] {
            assert!(text.contains(expected), "{text}");
        }
    }
}
