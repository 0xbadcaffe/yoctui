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
    assert!(output.contains("PgUp/Dn page"), "{output}");
}

#[test]
fn streaming_search_renders_matches_while_scanning_and_explicit_file_names_mode() {
    let mut app = search_app(1);
    let hits = app.global_search_content.hits()[..3].to_vec();
    app.global_search_content = yoctui_model::GlobalSearchContentState::Streaming {
        generation: 1,
        query: "match".into(),
        hits,
    };
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("scanning"), "{text}");
        assert!(text.contains("log-01"), "partial result missing: {text}");
        assert!(text.contains("Alt+n names"), "{text}");
    }
    app.global_search_target = yoctui_model::GlobalSearchTarget::FileNames;
    let text = rendered_text(&app, 160, 50);
    assert!(text.contains("File Names Regex Search"));
    assert!(text.contains("Alt+n contents"));
}

fn loading_search(workspace: bool) -> App {
    let mut app = search_app(0);
    app.global_search_content = yoctui_model::GlobalSearchContentState::Loading {
        generation: 1,
        query: "match".into(),
    };
    app.global_search_root = workspace.then(|| "/workspace/recipe".into());
    app
}

fn searching_row(app: &App, width: u16, height: u16) -> String {
    rendered_region_rows(width, height, |frame, area| {
        super::super::palette_render::command_palette(frame, app, area)
    })
    .into_iter()
    .find(|row| row.contains("Searching"))
    .expect("visible search loading status")
}

#[test]
fn global_search_loading_has_one_animated_marker_in_both_scopes() {
    for workspace in [false, true] {
        let mut app = loading_search(workspace);
        for (width, height) in [(160, 50), (100, 25)] {
            let mut previous = None;
            for phase in [0, 1, 7] {
                app.animation_frame = phase;
                let row = searching_row(&app, width, height);
                let marker = startup_activity_symbol(phase as usize);
                assert!(row.contains(&format!("{marker} Searching")), "{row}");
                assert!(
                    !row.contains(&format!("… {marker}")),
                    "duplicate static marker: {row}"
                );
                assert_eq!(
                    row.chars()
                        .filter(|ch| ('\u{2800}'..='\u{28ff}').contains(ch))
                        .count(),
                    marker.chars().count(),
                    "{row}"
                );
                if let Some(previous) = previous {
                    assert_ne!(row, previous, "animation did not advance");
                }
                previous = Some(row);
            }
            let output = rendered_text(&app, width, height);
            assert!(
                output.contains(if workspace {
                    "Workspace Regex Search"
                } else {
                    "Global Regex Search"
                }),
                "{output}"
            );
            assert!(output.contains("Keep typing"), "{output}");
        }
    }
}

#[test]
fn global_search_loading_reduced_motion_ascii_and_other_states_remain_safe() {
    let mut app = loading_search(false);
    app.reduced_motion = true;
    let first = searching_row(&app, 160, 50);
    app.animation_frame = 3;
    assert_eq!(first, searching_row(&app, 160, 50));
    assert!(
        first.contains("⣿ Searching") && !first.contains("… ⣿"),
        "{first}"
    );
    app.preferences.symbols = SymbolPreference::Ascii;
    let ascii = searching_row(&app, 160, 50);
    assert!(
        ascii.contains("* Searching") && !ascii.contains("… *"),
        "{ascii}"
    );
    assert!(
        !ascii
            .chars()
            .any(|ch| ('\u{2800}'..='\u{28ff}').contains(&ch)),
        "{ascii}"
    );
    for (width, height) in [(40, 15), (20, 8), (1, 1)] {
        let _ = rendered_text(&app, width, height);
    }
    app.global_search_content = yoctui_model::GlobalSearchContentState::Failed {
        generation: 1,
        query: "match".into(),
        message: "fixture failure".into(),
    };
    let failed = rendered_text(&app, 160, 50);
    assert!(
        failed.contains("Content search failed.") && failed.contains("fixture failure"),
        "{failed}"
    );
    assert!(!failed.contains("Searching"), "{failed}");
    let ready = rendered_text(&search_app(0), 160, 50);
    assert!(
        ready.contains("match-00") && !ready.contains("Searching"),
        "{ready}"
    );
}
