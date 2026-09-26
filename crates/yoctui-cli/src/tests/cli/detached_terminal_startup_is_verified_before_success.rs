use super::*;

#[test]
fn detached_terminal_profiles_preserve_exact_argv_and_wait_semantics() {
    let directory = DevworkTempDir::new();
    let launcher = DetachedTerminalLauncher {
        program: "/usr/bin/gnome-terminal".into(),
        prefix_arguments: vec!["--wait".into(), "--".into()],
    };
    let request = yoctui_model::TerminalLaunchRequest {
        name: "kernel menuconfig".into(),
        kind: yoctui_model::TerminalCreationKind::Utility,
        cwd: directory.0.clone(),
        program: "/usr/bin/env".into(),
        arguments: vec!["bitbake".into(), "virtual/kernel".into()],
        completion: None,
    };

    let command = detached_terminal_command(&launcher, &request).unwrap();
    let arguments = command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(arguments[0..2], ["--wait", "--"]);
    assert!(arguments[2].ends_with("/env"));
    assert_eq!(&arguments[3..], ["bitbake", "virtual/kernel"]);
}

#[test]
fn detached_terminal_early_exit_is_a_launch_failure() {
    let command = ProcessCommand::new("/usr/bin/false");
    let error = launch_and_probe_detached_terminal(
        command,
        Path::new("/usr/bin/false"),
        Duration::from_millis(20),
    )
    .unwrap_err();
    assert!(error.to_string().contains("exited during startup"));
}

#[tokio::test]
async fn detached_terminal_operation_reports_success_only_after_probe() {
    let directory = DevworkTempDir::new();
    let launcher = directory.0.join("terminal-fixture");
    std::fs::write(&launcher, "#!/bin/sh\nsleep 0.2\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let command = ProcessCommand::new(&launcher);
    launch_and_probe_detached_terminal(command, &launcher, Duration::from_millis(20)).unwrap();
}
