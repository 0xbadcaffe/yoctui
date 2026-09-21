use super::*;

#[test]
fn ux_preferences_reject_future_schema_and_invalid_keymap_without_partial_install() {
    let mut app = App::new(8, 1024);
    let original = app.effective_preferences();
    let mut future = original.clone();
    future.schema_version += 1;
    assert!(future.validate().is_err());
    assert!(app.install_preferences(future).is_err());
    assert_eq!(app.effective_preferences(), original);

    let mut invalid = original.clone();
    invalid.keymap.schema_version += 1;
    assert!(invalid.validate().is_err());
    assert_eq!(app.effective_preferences(), original);

    let mut changed = original.clone();
    changed.density = UiDensity::Compact;
    changed.symbols = SymbolPreference::Ascii;
    changed.mouse_enabled = false;
    app.install_preferences(changed.clone()).unwrap();
    assert_eq!(app.effective_preferences(), changed);
}
