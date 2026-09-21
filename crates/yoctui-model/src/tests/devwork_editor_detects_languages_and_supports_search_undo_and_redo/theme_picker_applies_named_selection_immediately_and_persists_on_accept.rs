use super::*;

#[test]
fn theme_picker_applies_named_selection_immediately_and_persists_on_accept() {
    let mut app = App::new(10, 1_000);
    app.color_enabled = false;
    assert_eq!(update(&mut app, Action::OpenThemePicker), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ThemePicker { .. })
    ));
    let _ = update(&mut app, Action::SelectTheme { delta: 1 });
    assert_eq!(app.theme, Theme::WhiteClassic);
    assert!(app.color_enabled);
    assert!(app.settings_dirty);
    assert!(matches!(
        update(&mut app, Action::ApplySelectedTheme),
        Some(Effect::PersistSettings)
    ));
    assert!(app.active_dialog().is_none());
}
