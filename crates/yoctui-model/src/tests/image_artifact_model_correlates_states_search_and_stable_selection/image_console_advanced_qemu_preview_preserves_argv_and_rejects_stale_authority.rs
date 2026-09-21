use super::*;

#[test]
fn image_console_advanced_qemu_preview_preserves_argv_and_rejects_stale_authority() {
    let mut app = qemu_model_app();
    update(&mut app, Action::BeginSelectedQemuLaunch);
    update(&mut app, Action::PreviewQemuLaunch);
    let Some(Dialog::QemuLaunchConfirmation(preview)) = app.active_dialog().cloned() else {
        panic!("missing preview");
    };
    let expected = preview.argv.clone();
    let mut stale = app.clone();
    stale.qemu_capability = QemuCapability::MissingTool;
    assert_eq!(
        update(&mut stale, Action::ConfirmQemuLaunchInTerminal),
        None
    );
    assert!(stale.active_dialog().is_some());
    let mut changed = app.clone();
    if let Some(Dialog::QemuLaunchConfirmation(value)) = changed.active_dialog_mut() {
        value.argv.push("unapproved".into());
    }
    assert_eq!(
        update(&mut changed, Action::ConfirmQemuLaunchInTerminal),
        None
    );
    let Some(Effect::Terminal(TerminalEffect::Create {
        kind,
        program,
        arguments,
        cwd,
        ..
    })) = update(&mut app, Action::ConfirmQemuLaunchInTerminal)
    else {
        panic!("missing terminal effect");
    };
    assert_eq!(kind, TerminalCreationKind::QemuConsole);
    assert_eq!(cwd, PathBuf::from("/build"));
    assert_eq!(program, expected[0]);
    assert_eq!(
        arguments,
        expected[1..]
            .iter()
            .map(|value| value.to_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(app.screen, Screen::TerminalSessions);
    assert!(app.qemu_sessions.is_empty());
}
