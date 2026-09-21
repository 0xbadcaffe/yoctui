use super::*;

#[test]
fn test_workflow_launch_editor_rejects_changed_authoritative_context() {
    let mut app = test_workflow_app();
    let _ = update(&mut app, Action::BeginSelectedTestLaunch);
    let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog_mut() else {
        panic!("test launch TOML editor");
    };
    editor.text = editor.text.replace(
        "machine = \"qemux86-64\"",
        "machine = \"untrusted-machine\"",
    );

    let _ = update(&mut app, Action::PreviewTestLaunch);

    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestLaunchTomlEditor {
            validation_error: Some(message),
            ..
        }) if message.contains("must match the current Testing context")
    ));
}
