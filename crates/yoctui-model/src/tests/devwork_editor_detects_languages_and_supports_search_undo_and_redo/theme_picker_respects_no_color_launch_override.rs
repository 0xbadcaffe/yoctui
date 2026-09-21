use super::*;

#[test]
fn theme_picker_respects_no_color_launch_override() {
    let mut app = App::new(10, 1_000);
    app.color_enabled = false;
    app.color_forced_off = true;
    let _ = update(&mut app, Action::OpenThemePicker);
    let _ = update(&mut app, Action::SelectTheme { delta: 1 });
    assert_eq!(app.theme, Theme::WhiteClassic);
    assert!(!app.color_enabled);

    app.settings_selection = SETTINGS
        .iter()
        .position(|setting| *setting == Setting::Color)
        .unwrap();
    assert_eq!(
        update(&mut app, Action::ChangeSelectedSetting { backwards: false }),
        None
    );
    assert_eq!(
        app.notification.as_deref(),
        Some("Disabled by --no-color for this launch; the stored choice is preserved.")
    );
}
