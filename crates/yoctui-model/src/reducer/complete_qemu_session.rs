//! State transitions beginning with CompleteQemuSession.
use super::*;

mod backspace_wic_create_field_to_wic_device_inventory_failed;
mod complete_qemu_session_to_append_wic_create_field;
mod select_wic_device;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::CompleteQemuSession { .. }
        | Action::FailQemuSession { .. }
        | Action::LoseQemuSession { .. }
        | Action::BeginQemuSessionCancellation { .. }
        | Action::BeginActiveQemuSessionCancellation
        | Action::ConfirmQemuSessionCancellation
        | Action::CancelQemuSessionCancellation
        | Action::RejectQemuSessionCancellation { .. }
        | Action::CancelQemuSession { .. }
        | Action::InspectWicCapability
        | Action::WicCapabilityLoaded(..)
        | Action::BeginSelectedWicCreate
        | Action::ToggleWicCreateTomlEditor
        | Action::AppendWicCreateTomlEditor(..)
        | Action::BackspaceWicCreateTomlEditor
        | Action::SelectWicCreateField { .. }
        | Action::ActivateWicCreateField
        | Action::CycleWicCreateChoice { .. }
        | Action::AppendWicCreateField(..) => {
            complete_qemu_session_to_append_wic_create_field::reduce_actions(app, action)
        }
        Action::BackspaceWicCreateField
        | Action::FinishWicCreateFieldEdit
        | Action::PreviewWicCreate
        | Action::CancelWicCreate
        | Action::CancelWicCreatePreview
        | Action::ConfirmWicCreate
        | Action::SelectWicOutput { .. }
        | Action::OpenSelectedWicOutput
        | Action::BeginActiveWicSessionCancellation
        | Action::BeginActiveImageRuntimeCancellation
        | Action::CancelWicSessionCancellation
        | Action::BeginWicOutputInventory(..)
        | Action::WicOutputInventoryLoaded { .. }
        | Action::WicOutputInventoryFailed { .. }
        | Action::BeginSelectedWicDeviceWrite
        | Action::BeginWicDeviceInventory(..)
        | Action::WicDeviceInventoryLoaded { .. }
        | Action::WicDeviceInventoryFailed { .. } => {
            backspace_wic_create_field_to_wic_device_inventory_failed::reduce_actions(app, action)
        }
        Action::SelectWicDevice { .. } => select_wic_device::reduce_actions(app, action),
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
