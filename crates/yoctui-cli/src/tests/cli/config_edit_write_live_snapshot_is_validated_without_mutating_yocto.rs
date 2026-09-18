use super::*;

#[test]
#[ignore = "requires YOCTUI_LIVE_BUILD_DIR; validates a copy and never writes the live file"]
fn config_edit_write_live_snapshot_is_validated_without_mutating_yocto() {
    let live_build = PathBuf::from(
        std::env::var("YOCTUI_LIVE_BUILD_DIR")
            .expect("YOCTUI_LIVE_BUILD_DIR must identify an initialized Yocto build"),
    );
    let live_local_conf = live_build.join("conf/local.conf");
    let live_before = fs::read(&live_local_conf).unwrap();
    let snapshot_build =
        std::env::temp_dir().join(format!("yoctui-config-live-{}", std::process::id()));
    let _ = fs::remove_dir_all(&snapshot_build);
    fs::create_dir_all(snapshot_build.join("conf")).unwrap();
    fs::write(snapshot_build.join("conf/local.conf"), &live_before).unwrap();

    write_config_assignment_atomic(
        &snapshot_build,
        &config_edit_request(&snapshot_build, "MACHINE", "qemux86-64"),
    )
    .unwrap();
    let snapshot = fs::read_to_string(snapshot_build.join("conf/local.conf")).unwrap();
    assert!(snapshot.contains("MACHINE = \"qemux86-64\""));
    assert_eq!(fs::read(&live_local_conf).unwrap(), live_before);
    fs::remove_dir_all(snapshot_build).unwrap();
}
