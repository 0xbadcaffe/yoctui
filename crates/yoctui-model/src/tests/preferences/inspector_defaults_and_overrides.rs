use super::*;

#[test]
fn inspector_preference_is_off_by_default_and_session_toggle_preserves_startup_choice() {
    let mut app = App::new(8, 1024);
    crate::update(&mut app, crate::Action::Open(crate::Screen::Settings));
    assert_eq!(app.focus, crate::FocusTarget::Workspace);
    assert!(!app.inspector_visible);
    assert!(!app.preferences.inspector_visible);
    crate::update(&mut app, crate::Action::ToggleInspector);
    assert!(app.inspector_visible);
    assert!(!app.effective_preferences().inspector_visible);
    app.settings_selection = SETTINGS
        .iter()
        .position(|setting| *setting == Setting::Inspector)
        .unwrap();
    assert!(matches!(
        crate::update(
            &mut app,
            crate::Action::ChangeSelectedSetting { backwards: false }
        ),
        Some(crate::Effect::PersistSettings)
    ));
    assert!(app.preferences.inspector_visible);
    crate::update(&mut app, crate::Action::ToggleInspector);
    assert!(!app.inspector_visible);
    let mut restarted = App::new(8, 1024);
    restarted
        .install_preferences(app.effective_preferences())
        .unwrap();
    assert!(restarted.inspector_visible);
    app.preferences.mouse_enabled = false;
    crate::update(&mut app, crate::Action::ResetSelectedPreference);
    assert!(!app.inspector_visible);
    assert!(!app.preferences.inspector_visible);
    assert!(!app.preferences.mouse_enabled);
}

#[test]
fn preference_rows_distinguish_defaults_custom_values_and_launch_overrides() {
    let mut app = App::new(8, 1024);
    assert!(
        app.preference_rows()
            .iter()
            .all(|row| !row.is_modified && row.value == row.default_value)
    );
    app.color_forced_off = true;
    app.color_enabled = false;
    app.preferences.mouse_enabled = false;
    app.logs.insert(crate::tests::log("first"));
    app.logs.follow = false;
    app.logs.paused_len = Some(1);
    app.logs.insert(crate::tests::log("second"));
    let rows = app.preference_rows();
    let color = rows
        .iter()
        .find(|row| row.setting == Setting::Color)
        .unwrap();
    assert!(!color.is_modified);
    assert_eq!(color.default_value, "true");
    let mouse = rows
        .iter()
        .find(|row| row.setting == Setting::Mouse)
        .unwrap();
    assert!(mouse.is_modified);
    assert_eq!(mouse.default_value, "true");
    crate::update(&mut app, crate::Action::ToggleInspector);
    app.settings_selection = SETTINGS
        .iter()
        .position(|setting| *setting == Setting::Mouse)
        .unwrap();
    crate::update(&mut app, crate::Action::ResetSelectedPreference);
    assert!(app.preferences.mouse_enabled);
    assert_eq!(app.logs.paused_len, Some(1));
    assert!(!app.logs.follow);
    assert!(
        app.inspector_visible,
        "resetting another preference preserves the session toggle"
    );
}
