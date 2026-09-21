use super::*;

#[test]
fn theme_picker_restores_original_theme_when_closed_with_escape() {
    let mut app = App::new(10, 1_000);
    app.color_enabled = false;
    let _ = update(&mut app, Action::OpenThemePicker);
    let _ = update(&mut app, Action::SelectTheme { delta: 2 });
    assert_eq!(app.theme, Theme::MatrixGreen);
    assert!(app.color_enabled);
    let _ = update(&mut app, Action::CloseThemePicker);
    assert_eq!(app.theme, Theme::DarkPro);
    assert!(!app.color_enabled);
    assert!(!app.settings_dirty);
    assert!(app.active_dialog().is_none());
}
