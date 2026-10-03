use super::*;

#[test]
fn workspace_editor_discovers_more_than_the_old_visible_file_limit() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-workspace-editor-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("src")).unwrap();
    for index in 0..600 {
        fs::write(
            root.join("src").join(format!("file-{index:03}.c")),
            "int value;\n",
        )
        .unwrap();
    }
    let files = recipe_editor_files(&root).unwrap();
    assert_eq!(files.len(), 600);
    assert!(files.contains(&PathBuf::from("src/file-599.c")));
    fs::remove_dir_all(root).unwrap();
}
