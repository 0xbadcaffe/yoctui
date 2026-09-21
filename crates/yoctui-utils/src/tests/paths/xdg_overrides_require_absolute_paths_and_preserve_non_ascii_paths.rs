use super::*;

#[test]
fn xdg_overrides_require_absolute_paths_and_preserve_non_ascii_paths() {
    let home = std::env::temp_dir().join("home with spaces-λ");
    let custom = std::env::temp_dir().join("custom-λ");
    assert_eq!(
        xdg_directory(Some(custom.clone()), Some(home.clone()), ".config"),
        Some(custom)
    );
    for invalid in [PathBuf::new(), PathBuf::from("relative")] {
        assert_eq!(
            xdg_directory(Some(invalid), Some(home.clone()), ".config"),
            Some(home.join(".config"))
        );
    }
    assert_eq!(
        xdg_directory(None, Some("relative".into()), ".config"),
        None
    );
    assert_eq!(xdg_directory(None, None, ".config"), None);
}
