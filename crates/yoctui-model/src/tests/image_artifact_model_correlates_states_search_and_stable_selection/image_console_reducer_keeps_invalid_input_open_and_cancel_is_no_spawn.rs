use super::*;

#[test]
fn image_console_reducer_keeps_invalid_input_open_and_cancel_is_no_spawn() {
    let mut app = qemu_model_app();
    app.qemu_capability = QemuCapability::MissingTool;
    app.ssh_client_capability = SshClientCapability::Available {
        executable: "/usr/bin/ssh".into(),
    };
    let _ = update(&mut app, Action::BeginSelectedImageConsole);
    assert_eq!(update(&mut app, Action::ConfirmImageConsole), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ImageConsole(dialog)) if dialog.validation_error.as_deref().is_some_and(|message| message.contains("SSH host"))
    ));
    assert_eq!(update(&mut app, Action::CancelImageConsole), None);
    assert!(app.active_dialog().is_none());
}
