use super::*;

#[test]
fn layer_tree_scanner_is_shallow_sorted_and_keeps_hidden_metadata() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-layer-tree-scan-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(directory.join("recipes-demo")).unwrap();
    fs::write(directory.join("demo.bb"), "SUMMARY = \"demo\"").unwrap();
    fs::write(directory.join(".hidden"), "hidden").unwrap();

    let entries = scan_layer_directory(&directory, true).unwrap();
    assert_eq!(
        entries[0].path.file_name().unwrap().to_string_lossy(),
        "recipes-demo"
    );
    assert!(entries.iter().any(|entry| entry.is_hidden));
    assert!(entries.iter().all(|entry| entry.depth == 0));
    assert!(!entries.iter().any(|entry| entry.path.ends_with(".git")));
    fs::remove_dir_all(directory).unwrap();
}
