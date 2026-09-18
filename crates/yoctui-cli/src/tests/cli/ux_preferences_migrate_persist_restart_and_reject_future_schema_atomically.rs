use super::*;

#[test]
fn ux_preferences_migrate_persist_restart_and_reject_future_schema_atomically() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-ux-preferences-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("session.toml");
    fs::write(
            &path,
            "theme = 'matrix-green'\nanimation_speed = 'slow'\nreduced_motion = true\ncolor_enabled = false\nlog_wrap = true\nlog_follow = false\n",
        )
        .unwrap();

    let mut legacy = read_session(Some(&path)).unwrap();
    let migrated = session_preferences(&legacy).unwrap();
    assert_eq!(migrated.theme, Theme::MatrixGreen);
    assert_eq!(migrated.animation_speed, AnimationSpeed::Slow);
    assert!(migrated.reduced_motion);
    assert!(!migrated.color_enabled);
    assert!(migrated.log_wrap);
    assert!(!migrated.log_follow);

    let mut app = App::new(10, 1_000);
    app.install_preferences(migrated).unwrap();
    app.preferences.density = yoctui_model::UiDensity::Compact;
    app.preferences.symbols = yoctui_model::SymbolPreference::Ascii;
    app.preferences.mouse_enabled = false;
    app.preferences.footer_shortcuts = false;
    app.preferences.charts = yoctui_model::ChartPreference::AccessibleText;
    app.preferences.remember_pane_sizes = false;
    persist_settings(Some(&path), &mut legacy, &app, true).unwrap();

    let bytes = fs::read(&path).unwrap();
    let restored = read_session(Some(&path)).unwrap();
    let preferences = restored.preferences.clone().unwrap();
    assert_eq!(preferences.density, yoctui_model::UiDensity::Compact);
    assert_eq!(preferences.symbols, yoctui_model::SymbolPreference::Ascii);
    assert!(!preferences.mouse_enabled);
    assert!(!preferences.footer_shortcuts);
    assert_eq!(
        preferences.charts,
        yoctui_model::ChartPreference::AccessibleText
    );
    assert_eq!(restored.pane_layout, None);
    assert_eq!(restored.theme, None);
    let mut restarted = App::new(10, 1_000);
    restarted.install_preferences(preferences).unwrap();
    assert_eq!(restarted.preferences, app.preferences);

    let mut invalid = restored;
    invalid.preferences.as_mut().unwrap().schema_version += 1;
    assert!(write_session(Some(&path), &invalid).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);

    fs::remove_file(path).unwrap();
    fs::remove_dir(directory).unwrap();
}
