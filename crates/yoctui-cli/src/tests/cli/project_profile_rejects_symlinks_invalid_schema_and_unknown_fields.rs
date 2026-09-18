use super::*;

#[cfg(unix)]
#[test]
fn project_profile_rejects_symlinks_invalid_schema_and_unknown_fields() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "yoctui-project-profile-reject-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).unwrap();
    let outside = root.join("outside.toml");
    fs::write(&outside, "schema_version = 1\n").unwrap();
    fs::create_dir(root.join(".yoctui")).unwrap();
    symlink(&outside, root.join(".yoctui/project.toml")).unwrap();
    assert!(load_project_profile(&root).is_err());
    fs::remove_file(root.join(".yoctui/project.toml")).unwrap();
    fs::write(root.join(".yoctui/project.toml"), "schema_version = 2\n").unwrap();
    assert!(load_project_profile(&root).is_err());
    fs::write(
        root.join(".yoctui/project.toml"),
        "schema_version = 1\ncommand = 'false'\n",
    )
    .unwrap();
    assert!(load_project_profile(&root).is_err());
    fs::remove_dir_all(root).unwrap();
}
