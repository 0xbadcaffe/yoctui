use super::*;

fn search_app(selection: usize) -> App {
    let mut app = App::new(10, 1_000);
    app.command_palette_open = true;
    app.command_palette_mode = CommandPaletteMode::GlobalRegexSearch;
    app.command_palette_query = "match".into();
    app.command_palette_selection = selection;
    app.global_search_content = yoctui_model::GlobalSearchContentState::Ready {
        generation: 1,
        query: "match".into(),
        hits: (0..24)
            .map(|index| yoctui_model::GlobalSearchHit {
                kind: yoctui_model::GlobalSearchContentKind::BuildLog,
                path: format!("/build/log-{index:02}").into(),
                line: 1,
                column: 1,
                preview: format!("match-{index:02}"),
                image: None,
            })
            .collect(),
        truncated: false,
        searched_scopes: vec!["build=/build".into()],
    };
    app
}

fn selected_marker_row(app: &App) -> usize {
    let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
    terminal
        .draw(|frame| render_at(frame, app, SystemTime::UNIX_EPOCH))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .chunks(100)
        .position(|row| row.iter().any(|cell| cell.symbol() == "▶"))
        .expect("selected search result marker")
}

#[test]
fn global_search_selection_moves_within_pages() {
    let fifth = selected_marker_row(&search_app(5));
    let sixth = selected_marker_row(&search_app(6));
    assert_eq!(sixth, fifth + 1);

    let output = rendered_text(&search_app(6), 100, 25);
    assert!(output.contains("PgUp/PgDn page"), "{output}");
}
