use super::*;

#[test]
fn qemu_workspace_maps_launch_edit_preview_and_cancellation_keys() {
    assert_eq!(
        images_workspace_action(false, Input::Char('Q')),
        Some(Action::BeginSelectedQemuLaunch)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('x')),
        Some(Action::BeginActiveImageRuntimeCancellation)
    );
    assert_eq!(
        qemu_launch_dialog_action(false, Input::Down),
        Some(Action::SelectQemuLaunchField { delta: 1 })
    );
    assert_eq!(
        qemu_launch_dialog_action(false, Input::Left),
        Some(Action::CycleQemuLaunchChoice { backwards: true })
    );
    assert_eq!(
        qemu_launch_dialog_action(false, Input::Enter),
        Some(Action::ActivateQemuLaunchField)
    );
    assert_eq!(
        qemu_launch_dialog_action(true, Input::Char('/')),
        Some(Action::AppendQemuLaunchField('/'))
    );
    assert_eq!(
        qemu_launch_dialog_action(true, Input::Backspace),
        Some(Action::BackspaceQemuLaunchField)
    );
    assert_eq!(
        qemu_launch_dialog_action(true, Input::Enter),
        Some(Action::FinishQemuLaunchFieldEdit)
    );
    assert_eq!(
        qemu_launch_dialog_action(false, Input::Char('p')),
        Some(Action::PreviewQemuLaunch)
    );
    assert_eq!(
        qemu_launch_dialog_action(true, Input::Esc),
        Some(Action::CancelQemuLaunch)
    );
    assert_eq!(
        qemu_launch_confirmation_action(Input::Enter),
        Some(Action::ConfirmQemuLaunchInTerminal)
    );
    assert_eq!(
        qemu_launch_confirmation_action(Input::Esc),
        Some(Action::CancelQemuLaunchPreview)
    );
    assert_eq!(
        qemu_cancellation_confirmation_action(Input::Enter),
        Some(Action::ConfirmQemuSessionCancellation)
    );
    assert_eq!(
        qemu_cancellation_confirmation_action(Input::Esc),
        Some(Action::CancelQemuSessionCancellation)
    );
}
