use super::*;

#[test]
fn ux_preferences_preview_and_reset_are_bounded_reversible_and_persisted() {
    let mut app = App::new(8, 1024);
    app.settings_selection = SETTINGS
        .iter()
        .position(|setting| *setting == Setting::Density)
        .unwrap();
    assert_eq!(
        crate::update(
            &mut app,
            crate::Action::ChangeSelectedSetting { backwards: false }
        ),
        Some(crate::Effect::PersistSettings)
    );
    assert_eq!(app.preferences.density, UiDensity::Compact);
    assert!(app.settings_dirty);

    assert_eq!(
        crate::update(&mut app, crate::Action::ResetPreferences),
        Some(crate::Effect::PersistSettings)
    );
    assert_eq!(app.effective_preferences(), WorkbenchPreferences::default());
    assert_eq!(app.pane_layout.pane_ids().len(), 1);
}
