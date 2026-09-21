use super::*;

#[test]
fn environment_setup_directory_scan_is_bounded_and_empty_is_valid() {
    let root = tempfile::tempdir().unwrap();
    assert!(
        read_environment_directory(root.path(), false, root.path())
            .unwrap()
            .children
            .is_empty()
    );
    for i in 0..ENVIRONMENT_DIRECTORY_LIMIT + 2 {
        fs::create_dir(root.path().join(format!("d{i}"))).unwrap();
    }
    let result = read_environment_directory(root.path(), false, root.path()).unwrap();
    assert_eq!(result.children.len(), ENVIRONMENT_DIRECTORY_LIMIT);
    assert!(result.notice.is_some());
}
