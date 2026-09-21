use super::*;

#[test]
fn project_profile_paths_reject_absolute_escape_and_platform_syntax() {
    for invalid in ["", "/etc/passwd", "../secret", "a/../b", "a\\b", "C:/tmp"] {
        assert!(PortableProjectPath::new(invalid).is_err(), "{invalid}");
    }
    assert_eq!(
        PortableProjectPath::new("docs/release.toml")
            .unwrap()
            .as_str(),
        "docs/release.toml"
    );
}
