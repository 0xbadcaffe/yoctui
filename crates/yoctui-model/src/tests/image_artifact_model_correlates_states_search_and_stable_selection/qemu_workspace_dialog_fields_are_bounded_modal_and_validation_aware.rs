use super::*;

#[test]
fn qemu_workspace_dialog_fields_are_bounded_modal_and_validation_aware() {
    let mut app = qemu_model_app();
    let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QemuLaunch(QemuLaunchDialog {
            selected_field: QemuLaunchField::Machine,
            editing: false,
            ..
        }))
    ));
    let _ = update(&mut app, Action::ActivateQemuLaunchField);
    assert_eq!(
        app.notification.as_deref(),
        Some("Image and machine identity are read-only.")
    );
    let _ = update(&mut app, Action::SelectQemuLaunchField { delta: 2 });
    let _ = update(&mut app, Action::ActivateQemuLaunchField);
    for _ in 0..(MAX_QEMU_PATH_INPUT_BYTES + 10) {
        let _ = update(&mut app, Action::AppendQemuLaunchField('x'));
    }
    let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog() else {
        panic!("launch dialog");
    };
    assert_eq!(dialog.draft.kernel.len(), MAX_QEMU_PATH_INPUT_BYTES);
    assert!(dialog.editing);
    let _ = update(&mut app, Action::FinishQemuLaunchFieldEdit);
    let _ = update(&mut app, Action::SelectQemuLaunchField { delta: 2 });
    let _ = update(&mut app, Action::CycleQemuLaunchChoice { backwards: false });
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QemuLaunch(QemuLaunchDialog {
            draft: QemuLaunchDraft {
                networking: QemuNetworkingMode::Tap,
                ..
            },
            ..
        }))
    ));
    let _ = update(&mut app, Action::PreviewQemuLaunch);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QemuLaunch(QemuLaunchDialog {
            validation_error: Some(message),
            ..
        })) if message.contains("paths must be normalized")
    ));
    let _ = update(&mut app, Action::CancelQemuLaunch);
    assert!(app.active_dialog().is_none());
    assert_eq!(app.focus, FocusTarget::Navigator);
}
