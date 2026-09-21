use super::*;

#[test]
fn global_content_search_rejects_stale_results_and_opens_selected_file() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenGlobalSearch);
    for character in "service token".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }
    let _ = update(&mut app, Action::BeginGlobalContentSearch);
    let generation = app.global_search_generation;
    let query = app.command_palette_query.clone();
    let hit = GlobalSearchHit {
        kind: GlobalSearchContentKind::ImageRootfs,
        path:
            "/build/tmp/work/machine/core-image-demo/1.0/rootfs/usr/lib/systemd/system/demo.service"
                .into(),
        line: 3,
        column: 13,
        preview: "Description=service token".into(),
        image: Some("core-image-demo".into()),
    };
    let _ = update(
        &mut app,
        Action::GlobalContentSearchLoaded {
            generation: generation.saturating_sub(1),
            query: query.clone(),
            hits: vec![hit.clone()],
            truncated: false,
            searched_scopes: vec!["stale".into()],
        },
    );
    assert!(app.global_search_content.hits().is_empty());
    let _ = update(
        &mut app,
        Action::GlobalContentSearchLoaded {
            generation,
            query,
            hits: vec![hit.clone()],
            truncated: false,
            searched_scopes: vec!["generated image".into()],
        },
    );
    assert_eq!(app.global_search_content.hits(), std::slice::from_ref(&hit));
    app.command_palette_selection = app.filtered_command_palette_commands().len();
    assert_eq!(
        update(&mut app, Action::ActivateCommandPalette),
        Some(Effect::OpenInEditor(hit.path))
    );
    assert!(!app.command_palette_open);
    let query = app.command_palette_query.clone();
    let selection = app.command_palette_selection;
    let results = app.global_search_content.clone();
    let _ = update(&mut app, Action::RestoreGlobalSearchResults);
    assert!(app.command_palette_open);
    assert_eq!(app.command_palette_query, query);
    assert_eq!(app.command_palette_selection, selection);
    assert_eq!(app.global_search_content, results);
}
