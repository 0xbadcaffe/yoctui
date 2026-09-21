use super::*;

#[test]
fn layer_tree_loads_children_lazily_and_collapses_without_losing_parent() {
    let mut app = App::new(10, 1_000);
    app.workspace.layers.push(Layer {
        name: "meta-demo".into(),
        path: "/layers/meta-demo".into(),
        priority: Some(5),
    });
    assert_eq!(
        update(&mut app, Action::BeginSelectedLayerBrowser),
        Some(Effect::LoadLayerBrowserDirectory {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo".into(),
        })
    );
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo".into(),
            entries: vec![LayerBrowserEntry {
                path: "/layers/meta-demo/recipes-core".into(),
                is_dir: true,
                ..LayerBrowserEntry::default()
            }],
        },
    );
    assert_eq!(
        update(&mut app, Action::LayerBrowserEnter),
        Some(Effect::LoadLayerBrowserDirectory {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo/recipes-core".into(),
        })
    );
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo/recipes-core".into(),
            entries: vec![LayerBrowserEntry {
                path: "/layers/meta-demo/recipes-core/demo.bb".into(),
                ..LayerBrowserEntry::default()
            }],
        },
    );
    let browser = app.layer_browser.as_ref().unwrap();
    assert_eq!(browser.entries.len(), 2);
    assert_eq!(browser.entries[0].depth, 0);
    assert_eq!(browser.entries[1].depth, 1);
    assert!(
        browser
            .nodes
            .contains_key(&PathBuf::from("/layers/meta-demo/recipes-core"))
    );
    let _ = update(&mut app, Action::LayerBrowserUp);
    assert_eq!(app.layer_browser.as_ref().unwrap().entries.len(), 1);
}
