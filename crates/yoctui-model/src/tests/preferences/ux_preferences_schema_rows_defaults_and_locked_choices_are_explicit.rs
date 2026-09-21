use super::*;

#[test]
fn ux_preferences_schema_rows_defaults_and_locked_choices_are_explicit() {
    let app = App::new(8, 1024);
    let rows = app.preference_rows();
    assert_eq!(rows.len(), SETTINGS.len());
    assert_eq!(rows.len(), 15);
    assert!(rows.iter().any(|row| row.setting == Setting::Density));
    assert!(rows.iter().any(|row| row.setting == Setting::Symbols));
    assert!(rows.iter().any(|row| row.setting == Setting::Mouse));
    assert!(rows.iter().any(|row| row.setting == Setting::Charts));
    assert!(
        rows.iter()
            .find(|row| row.setting == Setting::ImagePreviews)
            .is_some_and(|row| !row.enabled() && row.disabled_reason.is_some())
    );
    assert!(
        rows.iter()
            .find(|row| row.setting == Setting::TerminalPrefix)
            .is_some_and(|row| !row.enabled() && row.value == "Ctrl+B")
    );
    app.effective_preferences().validate().unwrap();
}
