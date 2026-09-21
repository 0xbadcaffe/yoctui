use super::*;

#[test]
fn metadata_search_tracks_query_and_resets_metadata_selection() {
    let mut app = App::new(10, 1_000);
    app.recipe_selection = 3;
    app.layer_selection = 2;
    app.config_selection = 1;

    let _ = update(&mut app, Action::BeginMetadataSearch);
    let _ = update(&mut app, Action::AppendMetadataQuery('q'));
    let _ = update(&mut app, Action::AppendMetadataQuery('e'));

    assert!(app.metadata_searching);
    assert_eq!(app.metadata_query, "qe");
    assert_eq!(
        (
            app.recipe_selection,
            app.layer_selection,
            app.config_selection
        ),
        (0, 0, 0)
    );

    let _ = update(&mut app, Action::BackspaceMetadataQuery);
    let _ = update(&mut app, Action::FinishMetadataSearch);
    assert_eq!(app.metadata_query, "q");
    assert!(!app.metadata_searching);
}
