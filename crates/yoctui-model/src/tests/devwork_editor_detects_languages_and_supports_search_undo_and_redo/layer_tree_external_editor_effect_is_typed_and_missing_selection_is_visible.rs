use super::*;

#[test]
fn layer_tree_external_editor_effect_is_typed_and_missing_selection_is_visible() {
    let mut app = App::new(10, 1_000);
    let mut browser = LayerBrowser::new("meta-demo".into(), "/layers/meta-demo".into());
    browser.entries.push(LayerBrowserEntry {
        path: "/layers/meta-demo/recipes-demo/demo/demo.bb".into(),
        ..LayerBrowserEntry::default()
    });
    app.layer_browser = Some(browser);
    assert_eq!(
        update(&mut app, Action::EditSelectedLayerBrowserFile),
        Some(Effect::OpenLayerBrowserEditor {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            file: "recipes-demo/demo/demo.bb".into(),
        })
    );
    app.layer_browser.as_mut().unwrap().entries.clear();
    assert_eq!(update(&mut app, Action::EditSelectedLayerBrowserFile), None);
    assert_eq!(app.notification.as_deref(), Some("Select a file to edit."));
}
