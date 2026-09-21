use super::*;

#[test]
fn command_palette_available_entry_dispatches_existing_typed_action() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenCommandPalette);
    for character in "Open Settings".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }

    assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
    assert_eq!(app.screen, Screen::Settings);
    assert!(!app.command_palette_open);
    assert_eq!(app.focus, FocusTarget::Navigator);
}
