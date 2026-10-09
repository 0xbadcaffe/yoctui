use super::*;

fn hit(name: &str) -> GlobalSearchHit {
    GlobalSearchHit {
        kind: GlobalSearchContentKind::GeneratedMetadata,
        path: format!("/build/{name}.txt").into(),
        line: 1,
        column: 1,
        preview: "needle".into(),
        image: None,
    }
}

#[test]
fn streaming_search_appends_without_reordering_selection_and_rejects_stale_batches() {
    let mut app = App::new(10, 1000);
    update(&mut app, Action::OpenGlobalSearch);
    app.command_palette_query = "needle".into();
    update(&mut app, Action::BeginGlobalContentSearch);
    let generation = app.global_search_generation;
    let publish = |app: &mut App, generation, hits| {
        update(
            app,
            Action::GlobalContentSearchProgress {
                generation,
                query: "needle".into(),
                hits,
            },
        );
    };
    publish(&mut app, generation, vec![hit("z"), hit("y")]);
    assert!(app.global_search_content.loading());
    app.command_palette_selection = 1;
    publish(&mut app, generation, vec![hit("a"), hit("z")]);
    assert_eq!(
        app.global_search_content.hits(),
        &[hit("z"), hit("y"), hit("a")]
    );
    assert_eq!(app.command_palette_selection, 1);
    publish(&mut app, generation.wrapping_sub(1), vec![hit("stale")]);
    assert_eq!(app.global_search_content.hits().len(), 3);
    update(&mut app, Action::CloseCommandPalette);
    assert!(!app.global_search_content.loading());
    assert!(matches!(
        app.global_search_content,
        GlobalSearchContentState::Ready {
            truncated: true,
            ..
        }
    ));
    publish(&mut app, generation, vec![hit("late")]);
    assert_eq!(app.global_search_content.hits().len(), 3);
    update(&mut app, Action::RestoreGlobalSearchResults);
    update(&mut app, Action::ToggleGlobalSearchTarget);
    assert_eq!(app.global_search_target, GlobalSearchTarget::FileNames);
    assert!(app.global_search_content.hits().is_empty());
    assert_eq!(app.command_palette_query, "needle");
    update(&mut app, Action::BeginGlobalContentSearch);
    publish(&mut app, generation, vec![hit("old-mode")]);
    assert!(app.global_search_content.hits().is_empty());
}

#[test]
fn streaming_search_bounds_and_validates_untrusted_results() {
    let mut app = App::new(10, 1000);
    update(&mut app, Action::OpenGlobalSearch);
    app.command_palette_query = "needle".into();
    update(&mut app, Action::BeginGlobalContentSearch);
    let mut invalid = hit("bad");
    invalid.path = "relative".into();
    let mut hits = vec![invalid];
    hits.extend((0..600).map(|i| hit(&i.to_string())));
    let generation = app.global_search_generation;
    update(
        &mut app,
        Action::GlobalContentSearchProgress {
            generation,
            query: "needle".into(),
            hits,
        },
    );
    assert_eq!(
        app.global_search_content.hits().len(),
        MAX_GLOBAL_SEARCH_HITS
    );
}
