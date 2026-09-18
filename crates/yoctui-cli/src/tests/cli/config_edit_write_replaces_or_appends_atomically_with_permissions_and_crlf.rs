use super::*;

#[cfg(unix)]
#[test]
fn config_edit_write_replaces_or_appends_atomically_with_permissions_and_crlf() {
    use std::os::unix::fs::PermissionsExt;

    let build_dir = std::env::temp_dir().join(format!("yoctui-config-edit-{}", std::process::id()));
    let _ = fs::remove_dir_all(&build_dir);
    let conf_dir = build_dir.join("conf");
    fs::create_dir_all(&conf_dir).unwrap();
    let local_conf = conf_dir.join("local.conf");
    fs::write(
            &local_conf,
            "# MACHINE = \"commented\"\r\nMACHINE ??= \"old\"\r\nMACHINE = \"later\"\r\nMACHINE_EXTRA = \"keep\"\r\n",
        )
        .unwrap();
    fs::set_permissions(&local_conf, fs::Permissions::from_mode(0o640)).unwrap();

    write_config_assignment_atomic(
        &build_dir,
        &config_edit_request(&build_dir, "MACHINE", "qemux86-64"),
    )
    .unwrap();
    let replaced = fs::read_to_string(&local_conf).unwrap();
    assert!(replaced.contains("# MACHINE = \"commented\"\r\n"));
    assert!(replaced.contains("MACHINE_EXTRA = \"keep\"\r\n"));
    assert_eq!(replaced.matches("MACHINE = \"qemux86-64\"").count(), 1);
    assert!(!replaced.replace("\r\n", "").contains('\n'));
    assert_eq!(
        fs::metadata(&local_conf).unwrap().permissions().mode() & 0o777,
        0o640
    );

    write_config_assignment_atomic(
        &build_dir,
        &config_edit_request(&build_dir, "DISTRO", "poky"),
    )
    .unwrap();
    let appended = fs::read_to_string(&local_conf).unwrap();
    assert!(appended.ends_with("DISTRO = \"poky\"\r\n"));
    assert!(!appended.replace("\r\n", "").contains('\n'));
    fs::remove_dir_all(build_dir).unwrap();
}
