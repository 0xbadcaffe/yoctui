use super::*;

#[test]
fn command_palette_empty_and_disabled_activation_are_inert() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenCommandPalette);
    let original = app.clone();
    assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
    assert_eq!(app, original, "disabled Build image must remain open");

    for character in "no such command".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }
    let no_results = app.clone();
    assert!(app.filtered_command_palette_commands().is_empty());
    assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
    assert_eq!(app, no_results);
}
