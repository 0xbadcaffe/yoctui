use super::*;

#[test]
fn layer_tree_maps_lazy_navigation_hidden_refresh_and_inspector_modes() {
    assert_eq!(
        layer_tree_action(false, Input::Right),
        Some(Action::LayerBrowserExpand)
    );
    assert_eq!(
        layer_tree_action(false, Input::Left),
        Some(Action::LayerBrowserUp)
    );
    assert_eq!(
        layer_tree_action(false, Input::Char('.')),
        Some(Action::ToggleLayerBrowserHidden)
    );
    assert_eq!(
        layer_tree_action(false, Input::Char('i')),
        Some(Action::SetLayerInspectorMode(LayerInspectorMode::Metadata))
    );
    assert_eq!(
        layer_tree_action(false, Input::PageUp),
        Some(Action::SelectLayerBrowserEntry { delta: -10 })
    );
    assert_eq!(
        layer_tree_action(false, Input::PageDown),
        Some(Action::SelectLayerBrowserEntry { delta: 10 })
    );
    assert_eq!(
        layer_tree_action(true, Input::Char('b')),
        Some(Action::AppendMetadataQuery('b'))
    );
}
