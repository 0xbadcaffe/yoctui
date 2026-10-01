use super::*;

#[test]
fn daemon_start_explicit_directory_overrides_inherited_environment() {
    assert!(!use_inherited_daemon_environment(
        Some(Path::new("/chosen/build")),
        true
    ));
    assert!(use_inherited_daemon_environment(None, true));
    assert!(!use_inherited_daemon_environment(None, false));
}

#[test]
fn daemon_start_owns_an_independent_session() {
    let mut command = ProcessCommand::new("sh");
    command.args(["-c", "printf '%s ' \"$$\"; ps -o sid= -p $$"]);
    detach_daemon_session(&mut command);
    let output = command.output().unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let ids: Vec<i32> = text
        .split_whitespace()
        .map(|id| id.parse().unwrap())
        .collect();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], ids[1]);
    assert_ne!(ids[1], unsafe { libc::getsid(0) });
}

#[test]
fn daemon_start_diagnostics_are_private_and_reject_symlinks() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!(
        "yoctui-daemon-log-{}-{}",
        std::process::id(),
        yoctui_utils::unix_ms()
    ));
    fs::create_dir(&root).unwrap();
    let log = root.join("daemon.log");
    open_daemon_log(&log).unwrap();
    assert_eq!(
        fs::metadata(&log).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let link = root.join("link");
    symlink(&log, &link).unwrap();
    assert!(open_daemon_log(&link).is_err());
    assert!(open_daemon_log(&root).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn daemon_start_infers_initialized_build_directory_and_canonical_script() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!(
        "yoctui-daemon-cwd-{}-{}",
        std::process::id(),
        yoctui_utils::unix_ms()
    ));
    let build = root.join("build/romulus");
    let upstream = root.join("upstream");
    std::fs::create_dir_all(build.join("conf")).unwrap();
    std::fs::create_dir_all(&upstream).unwrap();
    std::fs::write(build.join("conf/local.conf"), "MACHINE = \"romulus\"\n").unwrap();
    std::fs::write(build.join("conf/bblayers.conf"), "BBLAYERS = \"\"\n").unwrap();
    std::fs::write(upstream.join("oe-init-build-env"), "#!/bin/sh\n").unwrap();
    symlink("upstream/oe-init-build-env", root.join("oe-init-build-env")).unwrap();

    let profile = inferred_build_environment_profile(&build).unwrap();
    assert_eq!(profile.source_dir, root.canonicalize().unwrap());
    assert_eq!(profile.build_dir, build.canonicalize().unwrap());
    assert_eq!(
        profile.init_script,
        upstream.join("oe-init-build-env").canonicalize().unwrap()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn daemon_build_directory_initializes_explicit_profile() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!(
        "yoctui-daemon-build-directory-{}-{}",
        std::process::id(),
        yoctui_utils::unix_ms()
    ));
    let build = root.join("build/romulus");
    std::fs::create_dir_all(build.join("conf")).unwrap();
    std::fs::write(build.join("conf/local.conf"), "MACHINE = \"romulus\"\n").unwrap();
    std::fs::write(build.join("conf/bblayers.conf"), "BBLAYERS = \"\"\n").unwrap();
    let script = root.join("oe-init-build-env");
    std::fs::write(
        &script,
        "#!/bin/sh\nexport BUILDDIR=\"$1\"\nexport YOCTUI_DAEMON_PROFILE_TEST=ready\n",
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&script, permissions).unwrap();

    let environment = initialize_daemon_build_directory(&build).await.unwrap();

    assert_eq!(
        environment.get("BUILDDIR"),
        Some(&build.display().to_string())
    );
    assert_eq!(
        environment.get("YOCTUI_DAEMON_PROFILE_TEST"),
        Some(&"ready".to_owned())
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn daemon_build_directory_rejects_a_non_yocto_profile() {
    let missing = std::env::temp_dir().join(format!(
        "yoctui-daemon-build-directory-missing-{}-{}",
        std::process::id(),
        yoctui_utils::unix_ms()
    ));
    let error = initialize_daemon_build_directory(&missing)
        .await
        .expect_err("an explicit invalid build directory must fail");
    assert!(
        error
            .to_string()
            .contains("cannot locate oe-init-build-env")
    );
}
