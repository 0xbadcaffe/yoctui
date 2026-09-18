use super::*;

#[cfg(unix)]
#[test]
fn security_daemon_rejects_untrusted_profile_and_shell_command() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "yoctui-security-daemon-profile-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".yoctui")).unwrap();

    let outside = root.join("outside.toml");
    fs::write(&outside, "schema_version = 1\n").unwrap();
    symlink(&outside, root.join(".yoctui/project.toml")).unwrap();
    assert!(load_project_profile(&root).is_err());

    fs::remove_file(root.join(".yoctui/project.toml")).unwrap();
    fs::write(
        root.join(".yoctui/project.toml"),
        "schema_version = 1\ncommand = 'false'\n",
    )
    .unwrap();
    assert!(load_project_profile(&root).is_err());
    fs::remove_dir_all(root).unwrap();
}
