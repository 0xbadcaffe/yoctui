use super::*;

#[test]
fn environment_setup_directory_scan_missing_build_hidden_and_split_source() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join(".hidden")).unwrap();
    fs::create_dir_all(root.path().join("layers/openembedded-core")).unwrap();
    fs::write(
        root.path()
            .join("layers/openembedded-core/oe-init-build-env"),
        "not executed",
    )
    .unwrap();
    fs::write(root.path().join("regular-file"), "ignored").unwrap();
    let result =
        read_environment_directory(&root.path().join("missing/build"), true, root.path()).unwrap();
    assert_eq!(result.path, root.path().canonicalize().unwrap());
    assert_eq!(result.children.len(), 2);
    assert!(result.children[0].ends_with(".hidden"));
    assert!(
        result
            .init_script
            .unwrap()
            .ends_with("layers/openembedded-core/oe-init-build-env")
    );
    assert!(!root.path().join("missing").exists());
    assert!(read_environment_directory(&root.path().join("missing"), false, root.path()).is_err());
    assert!(read_environment_directory(Path::new("relative"), true, root.path()).is_err());
    assert!(
        read_environment_directory(&root.path().join("regular-file"), false, root.path()).is_err()
    );
}
