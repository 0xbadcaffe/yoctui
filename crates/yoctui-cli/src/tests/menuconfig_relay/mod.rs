use super::*;

mod bitbake_configuration;

#[test]
fn relay_configuration_overrides_cached_terminal_selection_and_cleans_up() {
    let runtime = tempfile::tempdir().unwrap();
    let socket = runtime.path().join("relay.sock");
    let mut environment = std::collections::BTreeMap::new();
    configure_environment(
        &mut environment,
        Path::new("/opt/Roy's tools/yoctui"),
        &socket,
    );
    let (directory, config) = configuration::create(&environment, &socket).unwrap();
    let contents = fs::read_to_string(&config).unwrap();
    assert!(contents.starts_with("OE_TERMINAL = \"custom\"\n"));
    assert!(contents.contains("\nOE_TERMINAL_CUSTOMCMD = \"${@bytes.fromhex('"));
    assert!(contents.lines().all(|line| line.contains(" = \"")));
    assert!(!contents.contains("__anonymous"));
    assert!(contents.contains("PATH:prepend:task-devshell"));
    assert!(!contents.contains("HOSTTOOLS_DIR"));
    let path = directory.path().to_owned();
    drop(directory);
    assert!(!path.exists());
    assert_eq!(
        configuration::task_label(&["iw".into(), "-c".into(), "devshell".into()]),
        "devshell"
    );
    assert_eq!(
        configuration::task_label(&["virtual/kernel".into(), "-c".into(), "menuconfig".into()]),
        "menuconfig"
    );
}

#[test]
fn relay_command_preserves_bitbake_arguments_without_a_shell() {
    let (program, arguments) = command(
        Path::new("/opt/bitbake/bin/bitbake"),
        &["virtual/kernel".into(), "-c".into(), "menuconfig".into()],
    )
    .unwrap();
    assert!(program.is_absolute());
    assert_eq!(
        arguments,
        [
            "__menuconfig-relay",
            "--bitbake",
            "/opt/bitbake/bin/bitbake",
            "--",
            "virtual/kernel",
            "-c",
            "menuconfig",
        ]
    );
}

#[test]
fn command_validation_rejects_paths_outside_the_build() {
    let build = std::env::temp_dir().join(format!(
        "yoctui-menuconfig-validation-{}",
        std::process::id()
    ));
    fs::create_dir_all(&build).unwrap();
    assert!(
        validate_command(
            &build,
            &[
                build.clone(),
                PathBuf::from("/bin/true"),
                std::env::temp_dir().join("pidfile"),
                PathBuf::from("/bin/false"),
            ],
        )
        .is_err()
    );
    fs::remove_dir_all(build).unwrap();
}

#[test]
fn relay_environment_appends_the_stable_terminal_variables() {
    let mut environment = std::collections::BTreeMap::from([(
        "BB_ENV_PASSTHROUGH_ADDITIONS".into(),
        "MACHINE DISTRO".into(),
    )]);
    configure_environment(
        &mut environment,
        Path::new("/opt/bin/yoctui"),
        Path::new("/run/user/1000/yoctui/menuconfig.sock"),
    );
    assert_eq!(environment["OE_TERMINAL"], "custom");
    assert_eq!(
        environment["BB_ENV_PASSTHROUGH_ADDITIONS"],
        "MACHINE DISTRO OE_TERMINAL OE_TERMINAL_CUSTOMCMD"
    );
    assert!(environment["OE_TERMINAL_CUSTOMCMD"].contains("__menuconfig-handoff"));
    assert!(environment["OE_TERMINAL_CUSTOMCMD"].contains("-- {command}"));
}

#[test]
fn concurrent_relays_use_distinct_private_sockets() {
    let runtime_directory = Path::new("/run/user/1000/yoctui");

    let first = socket_path_in(runtime_directory, 1201);
    let second = socket_path_in(runtime_directory, 1202);

    assert_eq!(
        first,
        Path::new("/run/user/1000/yoctui/menuconfig-1201.sock")
    );
    assert_eq!(
        second,
        Path::new("/run/user/1000/yoctui/menuconfig-1202.sock")
    );
    assert_ne!(first, second);
}

#[cfg(target_os = "linux")]
#[test]
fn outer_bitbake_client_cannot_share_the_menuconfig_pty_streams() {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "sleep 30"]);
    isolate_outer_bitbake_console(&mut command);
    let mut child = command.spawn().unwrap();

    for descriptor in 0..=2 {
        let target = fs::read_link(format!("/proc/{}/fd/{descriptor}", child.id())).unwrap();
        if descriptor == 0 {
            assert_eq!(target, Path::new("/dev/null"));
        } else {
            assert!(target.to_string_lossy().starts_with("pipe:"));
        }
    }

    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn failed_bitbake_diagnostics_are_bounded_and_keep_the_error() {
    let mut command = Command::new("python3");
    command.args(["-c", "import sys; sys.stdout.write('x' * 131072); sys.stdout.flush(); sys.stderr.write('ERROR: missing ncurses dependency\\n'); sys.exit(1)"]);
    isolate_outer_bitbake_console(&mut command);
    let mut child = command.spawn().unwrap();
    let mut diagnostics = diagnostics::Diagnostics::start(&mut child).unwrap();
    assert!(!child.wait().unwrap().success());
    let tail = diagnostics.finish();
    assert!(tail.len() <= diagnostics::LIMIT);
    assert!(tail.len() >= diagnostics::LIMIT / 2);
    assert!(String::from_utf8_lossy(&tail).contains("ERROR: missing ncurses dependency"));
}

#[test]
fn diagnostics_stop_even_when_a_descendant_keeps_the_pipe_open() {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "sleep 1 & echo 'ERROR: failed handoff'; exit 1"]);
    isolate_outer_bitbake_console(&mut command);
    let mut child = command.spawn().unwrap();
    let mut diagnostics = diagnostics::Diagnostics::start(&mut child).unwrap();
    child.wait().unwrap();
    let started = std::time::Instant::now();
    let tail = diagnostics.finish();
    assert!(started.elapsed() < Duration::from_millis(500));
    assert!(String::from_utf8_lossy(&tail).contains("ERROR: failed handoff"));
}
