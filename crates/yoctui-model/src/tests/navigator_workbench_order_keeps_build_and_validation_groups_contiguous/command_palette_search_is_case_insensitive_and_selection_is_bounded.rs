use super::*;

#[test]
fn command_palette_search_is_case_insensitive_and_selection_is_bounded() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenCommandPalette);
    for character in "PROVENANCE".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }
    let commands = app.filtered_command_palette_commands();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, CommandId::OpenConfiguration);

    let _ = update(&mut app, Action::SelectCommandPalette { delta: 99 });
    assert_eq!(app.command_palette_selection, 0);
    for _ in 0.."PROVENANCE".len() {
        let _ = update(&mut app, Action::BackspaceCommandPaletteQuery);
    }
    assert!(app.command_palette_query.is_empty());
    assert!(app.filtered_command_palette_commands().len() > 6);
}
