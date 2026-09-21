use super::*;

#[test]
fn settings_selection_and_changes_are_typed_and_persisted() {
    let mut app = App::new(10, 1_000);
    assert_eq!(SETTINGS[app.settings_selection], Setting::Theme);
    assert_eq!(
        update(&mut app, Action::ChangeSelectedSetting { backwards: false }),
        Some(Effect::PersistSettings)
    );
    assert_eq!(app.theme, Theme::WhiteClassic);
    assert!(app.settings_dirty);

    let log_follow_index = SETTINGS
        .iter()
        .position(|setting| *setting == Setting::LogFollow)
        .unwrap();
    let _ = update(
        &mut app,
        Action::SelectSetting {
            delta: log_follow_index as isize,
        },
    );
    assert_eq!(SETTINGS[app.settings_selection], Setting::LogFollow);
    assert_eq!(
        update(&mut app, Action::ChangeSelectedSetting { backwards: true }),
        Some(Effect::PersistSettings)
    );
    assert!(!app.logs.follow);
    assert_eq!(app.logs.paused_len, Some(0));

    let _ = update(&mut app, Action::SettingsPersisted);
    assert!(!app.settings_dirty);
    assert!(app.notification.is_none());
}
