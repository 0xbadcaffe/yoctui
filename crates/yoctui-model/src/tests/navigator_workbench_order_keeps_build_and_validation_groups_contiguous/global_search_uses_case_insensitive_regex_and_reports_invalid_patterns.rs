use super::*;

#[test]
fn global_search_uses_case_insensitive_regex_and_reports_invalid_patterns() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenGlobalSearch);
    assert_eq!(
        app.command_palette_mode,
        CommandPaletteMode::GlobalRegexSearch
    );
    assert!(app.filtered_command_palette_commands().is_empty());
    let _ = update(&mut app, Action::BeginGlobalContentSearch);
    assert_eq!(app.global_search_content, GlobalSearchContentState::Idle);
    for character in "^open (packages|sdk)$".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }
    let ids = app
        .filtered_command_palette_commands()
        .into_iter()
        .map(|command| command.id)
        .collect::<Vec<_>>();
    assert!(ids.is_empty());
    assert_eq!(app.command_palette_regex_error(), None);

    let _ = update(&mut app, Action::ClearCommandPaletteQuery);
    let _ = update(&mut app, Action::AppendCommandPaletteQuery('['));
    assert!(app.filtered_command_palette_commands().is_empty());
    assert!(app.command_palette_regex_error().is_some());

    let _ = update(&mut app, Action::CloseCommandPalette);
    let _ = update(&mut app, Action::OpenCommandPalette);
    assert_eq!(app.command_palette_mode, CommandPaletteMode::Commands);
    assert_eq!(app.command_palette_regex_error(), None);
}
