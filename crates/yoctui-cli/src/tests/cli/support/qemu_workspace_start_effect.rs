use super::*;

pub(crate) fn qemu_workspace_start_effect(app: &mut App) -> (QemuSessionId, QemuLaunchRequest) {
    let _ = update(app, Action::BeginSelectedQemuLaunch);
    let _ = update(app, Action::PreviewQemuLaunch);
    let Some(Effect::StartQemuSession { id, request }) = update(app, Action::ConfirmQemuLaunch)
    else {
        panic!("expected QEMU start effect");
    };
    (id, request)
}
