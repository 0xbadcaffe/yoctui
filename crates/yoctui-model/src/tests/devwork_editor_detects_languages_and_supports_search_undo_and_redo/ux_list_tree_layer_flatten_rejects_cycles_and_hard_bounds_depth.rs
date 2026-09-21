use super::*;

#[test]
fn ux_list_tree_layer_flatten_rejects_cycles_and_hard_bounds_depth() {
    let root = PathBuf::from("/layers/meta-cycle");
    let child = root.join("child");
    let mut browser = LayerBrowser::new("meta-cycle".into(), root.clone());
    browser.nodes.insert(
        root.clone(),
        vec![LayerBrowserEntry {
            path: child.clone(),
            is_dir: true,
            ..LayerBrowserEntry::default()
        }],
    );
    browser.nodes.insert(
        child.clone(),
        vec![LayerBrowserEntry {
            path: root.clone(),
            is_dir: true,
            ..LayerBrowserEntry::default()
        }],
    );
    browser.expanded.insert(child);
    browser.rebuild(None);
    assert_eq!(browser.cycle_entries, 1);
    assert!(browser.entries.len() <= LIST_TREE_MAX_ROWS);
    assert!(
        browser
            .entries
            .iter()
            .all(|entry| entry.depth <= LIST_TREE_MAX_DEPTH)
    );
}
