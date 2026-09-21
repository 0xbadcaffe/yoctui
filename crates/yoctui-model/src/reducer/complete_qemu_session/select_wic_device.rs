use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::SelectWicDevice { delta } => {
            let Some(Dialog::WicDevicePicker(dialog)) = app.active_dialog() else {
                return None;
            };
            let request_matches = matches!(
                &app.wic_devices,
                WicDeviceInventoryState::Available { request, .. }
                    | WicDeviceInventoryState::Partial { request, .. }
                    if request == &dialog.request
            );
            if !request_matches {
                return None;
            }
            let rows = app.wic_device_rows();
            if rows.is_empty() {
                app.wic_device_selection = None;
                return None;
            }
            let current = app
                .wic_device_selection
                .as_ref()
                .and_then(|selected| rows.iter().position(|row| &row.identity == selected))
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current.saturating_add(delta as usize).min(rows.len() - 1)
            };
            app.wic_device_selection = Some(rows[next].identity.clone());
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
