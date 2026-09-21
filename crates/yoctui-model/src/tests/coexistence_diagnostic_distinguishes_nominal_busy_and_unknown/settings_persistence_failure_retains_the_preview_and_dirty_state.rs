use super::*;

#[test]
fn settings_persistence_failure_retains_the_preview_and_dirty_state() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::ChangeSelectedSetting { backwards: true });
    assert_eq!(app.theme, Theme::HighContrast);

    let _ = update(
        &mut app,
        Action::SettingsPersistenceFailed("read-only filesystem".into()),
    );
    assert_eq!(app.theme, Theme::HighContrast);
    assert!(app.settings_dirty);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("read-only filesystem")
    );
    assert_eq!(
        update(&mut app, Action::RetrySettingsPersistence),
        Some(Effect::PersistSettings)
    );
}
