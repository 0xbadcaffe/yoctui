use super::*;

#[test]
fn config_edit_write_rejects_tampering_and_leaves_failed_destination_untouched() {
    let build_dir =
        std::env::temp_dir().join(format!("yoctui-config-reject-{}", std::process::id()));
    let _ = fs::remove_dir_all(&build_dir);
    fs::create_dir_all(build_dir.join("conf")).unwrap();
    let local_conf = build_dir.join("conf/local.conf");
    fs::write(&local_conf, "MACHINE = \"old\"\n").unwrap();
    let original = fs::read(&local_conf).unwrap();
    let mut request = config_edit_request(&build_dir, "MACHINE", "qemux86-64");
    request.assignment = "MACHINE = \"tampered\"".into();
    assert!(write_config_assignment_atomic(&build_dir, &request).is_err());
    assert_eq!(fs::read(&local_conf).unwrap(), original);

    let failed_build =
        std::env::temp_dir().join(format!("yoctui-config-failure-{}", std::process::id()));
    let _ = fs::remove_dir_all(&failed_build);
    let failed_destination = failed_build.join("conf/local.conf");
    fs::create_dir_all(&failed_destination).unwrap();
    let sentinel = failed_destination.join("sentinel");
    fs::write(&sentinel, "unchanged").unwrap();
    assert!(
        write_config_assignment_atomic(
            &failed_build,
            &config_edit_request(&failed_build, "MACHINE", "qemux86-64"),
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(sentinel).unwrap(), "unchanged");
    assert!(
        fs::read_dir(failed_build.join("conf"))
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".yoctui-"))
    );
    fs::remove_dir_all(build_dir).unwrap();
    fs::remove_dir_all(failed_build).unwrap();
}
