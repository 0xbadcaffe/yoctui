use super::*;

#[test]
fn layer_tree_preview_bounds_large_text_and_rejects_binary() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-layer-tree-preview-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let text = directory.join("large.conf");
    fs::write(&text, vec![b'A'; 70 * 1024]).unwrap();
    let (content, kind, truncated) = read_layer_preview(&text).unwrap();
    assert_eq!(kind, PreviewKind::Text);
    assert_eq!(content.len(), 64 * 1024);
    assert!(truncated);

    let binary = directory.join("image.bin");
    fs::write(&binary, [0, 159, 146, 150]).unwrap();
    let (content, kind, truncated) = read_layer_preview(&binary).unwrap();
    assert_eq!(kind, PreviewKind::Binary);
    assert!(content.is_empty());
    assert!(!truncated);
    fs::remove_dir_all(directory).unwrap();
}
