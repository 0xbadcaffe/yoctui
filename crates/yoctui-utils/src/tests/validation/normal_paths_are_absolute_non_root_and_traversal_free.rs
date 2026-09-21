use super::*;

#[test]
fn normal_paths_are_absolute_non_root_and_traversal_free() {
    let separator = std::path::MAIN_SEPARATOR;
    let valid = std::path::PathBuf::from(format!("{separator}tmp{separator}image.wic"));
    assert!(is_absolute_normal_path(&valid));
    assert!(is_absolute_normal_path_within(
        &valid,
        valid.as_os_str().len()
    ));
    assert!(!is_absolute_normal_path(Path::new("relative/image.wic")));
    assert!(!is_absolute_normal_path(Path::new(
        std::path::MAIN_SEPARATOR_STR
    )));
    assert!(!is_absolute_normal_path(Path::new(&format!(
        "{separator}tmp{separator}..{separator}image.wic"
    ))));
    assert!(!is_absolute_normal_path_within(&valid, 1));
}
