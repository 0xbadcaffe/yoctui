use super::*;

#[test]
fn path_entry_detection_distinguishes_missing_and_present_entries() {
    let root = std::env::temp_dir().join(format!("yoctui-utils-entry-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("output");
    assert!(!path_entry_exists(&path).unwrap());
    std::fs::write(&path, b"present").unwrap();
    assert!(path_entry_exists(&path).unwrap());
    std::fs::remove_dir_all(root).unwrap();
}
