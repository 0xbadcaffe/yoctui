use super::*;

#[test]
fn devwork_terminal_detached_plan_preserves_argv_without_shell_evaluation() {
    let directory = DevworkTempDir::new();
    let launcher = DetachedTerminalLauncher {
        program: "/usr/bin/xterm".into(),
        command_separator: "-e",
    };
    let request = yoctui_model::TerminalLaunchRequest {
        name: "devtool edit-recipe busybox".into(),
        kind: yoctui_model::TerminalCreationKind::Utility,
        cwd: directory.0.clone(),
        program: "/usr/bin/env".into(),
        arguments: vec![
            "devtool".into(),
            "edit-recipe".into(),
            "busybox;touch /tmp/not-executed".into(),
        ],
    };
    let command = detached_terminal_command(&launcher, &request).unwrap();
    assert_eq!(
        command.get_program(),
        std::ffi::OsStr::new("/usr/bin/xterm")
    );
    let arguments = command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(arguments[0], "-e");
    assert!(arguments[1].ends_with("/env"));
    assert_eq!(
        &arguments[2..],
        ["devtool", "edit-recipe", "busybox;touch /tmp/not-executed"]
    );
    assert!(!arguments.iter().any(|argument| argument == "-c"));
}
