use super::*;

#[test]
fn ux_list_tree_keyboard_routes_expansion_pages_edges_and_search_without_widget_state() {
    assert_eq!(
        layer_tree_action(false, Input::Right),
        Some(Action::LayerBrowserExpand)
    );
    assert_eq!(
        layer_tree_action(false, Input::PageDown),
        Some(Action::SelectLayerBrowserEntry {
            delta: DEFAULT_COLLECTION_PAGE_ROWS
        })
    );
    assert_eq!(
        layer_tree_action(false, Input::End),
        Some(Action::SelectLayerBrowserEntry { delta: isize::MAX })
    );
    assert_eq!(
        layer_tree_action(false, Input::Char('/')),
        Some(Action::BeginMetadataSearch)
    );
}
