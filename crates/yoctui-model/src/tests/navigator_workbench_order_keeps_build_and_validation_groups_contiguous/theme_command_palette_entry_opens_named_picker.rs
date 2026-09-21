use super::*;

#[test]
fn theme_command_palette_entry_opens_named_picker() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Navigator;
    let _ = update(&mut app, Action::OpenCommandPalette);
    for character in "Choose theme".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }

    assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ThemePicker { .. })
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(app.focus_return, Some(FocusTarget::Navigator));
}
