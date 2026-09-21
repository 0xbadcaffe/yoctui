use super::*;

#[test]
fn global_search_query_is_bounded() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenGlobalSearch);
    for _ in 0..MAX_COMMAND_PALETTE_QUERY_CHARS + 20 {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery('x'));
    }
    assert_eq!(
        app.command_palette_query.chars().count(),
        MAX_COMMAND_PALETTE_QUERY_CHARS
    );
}
