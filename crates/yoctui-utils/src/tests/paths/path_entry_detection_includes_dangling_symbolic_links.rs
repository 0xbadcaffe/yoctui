use super::*;

#[test]
fn path_entry_detection_includes_dangling_symbolic_links() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "yoctui-utils-dangling-entry-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let link = root.join("output");
    symlink(root.join("missing"), &link).unwrap();
    assert!(path_entry_exists(&link).unwrap());
    std::fs::remove_dir_all(root).unwrap();
}
