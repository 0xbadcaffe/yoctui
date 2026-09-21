use super::*;

#[test]
fn image_console_reducer_launches_qemu_or_ssh_through_typed_terminal_effects() {
    let mut qemu = qemu_model_app();
    let _ = update(&mut qemu, Action::BeginSelectedImageConsole);
    assert!(matches!(
        qemu.active_dialog(),
        Some(Dialog::ImageConsole(dialog)) if dialog.draft.mode == ImageConsoleMode::Qemu
    ));
    let Some(Effect::Terminal(TerminalEffect::Create {
        kind,
        program,
        arguments,
        ..
    })) = update(&mut qemu, Action::ConfirmImageConsole)
    else {
        panic!("expected QEMU terminal creation");
    };
    assert_eq!(kind, TerminalCreationKind::QemuConsole);
    assert_eq!(program, PathBuf::from("/opt/poky/scripts/runqemu"));
    assert!(arguments.iter().any(|argument| argument == "nographic"));
    assert!(arguments.iter().any(|argument| argument == "serialstdio"));
    assert_eq!(qemu.screen, Screen::TerminalSessions);

    let mut ssh = qemu_model_app();
    ssh.qemu_capability = QemuCapability::MissingTool;
    ssh.ssh_client_capability = SshClientCapability::Available {
        executable: "/usr/bin/ssh".into(),
    };
    let _ = update(&mut ssh, Action::BeginSelectedImageConsole);
    let Some(Dialog::ImageConsole(dialog)) = ssh.active_dialog_mut() else {
        panic!("expected Image Console dialog");
    };
    assert_eq!(dialog.draft.mode, ImageConsoleMode::Ssh);
    dialog.draft.host = "target.example".into();
    dialog.draft.user = "root".into();
    let Some(Effect::Terminal(TerminalEffect::Create {
        kind,
        program,
        arguments,
        ..
    })) = update(&mut ssh, Action::ConfirmImageConsole)
    else {
        panic!("expected SSH terminal creation");
    };
    assert_eq!(kind, TerminalCreationKind::SshConsole);
    assert_eq!(program, PathBuf::from("/usr/bin/ssh"));
    assert_eq!(
        arguments.last().map(String::as_str),
        Some("root@target.example")
    );
}
