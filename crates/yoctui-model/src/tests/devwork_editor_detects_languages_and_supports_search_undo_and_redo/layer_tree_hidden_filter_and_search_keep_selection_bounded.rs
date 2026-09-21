use super::*;

#[test]
fn layer_tree_hidden_filter_and_search_keep_selection_bounded() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo".into(),
            entries: vec![
                LayerBrowserEntry {
                    path: "/layers/meta-demo/.hidden".into(),
                    is_hidden: true,
                    ..LayerBrowserEntry::default()
                },
                LayerBrowserEntry {
                    path: "/layers/meta-demo/visible.bb".into(),
                    ..LayerBrowserEntry::default()
                },
            ],
        },
    );
    assert_eq!(app.layer_browser.as_ref().unwrap().entries.len(), 1);
    let _ = update(&mut app, Action::ToggleLayerBrowserHidden);
    assert_eq!(app.layer_browser.as_ref().unwrap().entries.len(), 2);
    let _ = update(&mut app, Action::BeginMetadataSearch);
    let _ = update(&mut app, Action::AppendMetadataQuery('v'));
    let _ = update(
        &mut app,
        Action::SelectLayerBrowserEntry { delta: isize::MAX },
    );
    assert_eq!(
        app.layer_browser
            .as_ref()
            .unwrap()
            .selected_entry()
            .unwrap()
            .path,
        PathBuf::from("/layers/meta-demo/visible.bb")
    );
}
