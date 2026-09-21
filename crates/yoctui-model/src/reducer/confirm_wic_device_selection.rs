//! State transitions beginning with ConfirmWicDeviceSelection.
use super::*;

mod build_completed_to_build_authority_lost;
mod confirm_wic_device_selection_to_confirm_wic_session_cancellation;
mod reject_wic_session_cancellation_to_logs;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::ConfirmWicDeviceSelection
        | Action::CancelWicDevicePicker
        | Action::AppendWicWritePhrase(..)
        | Action::BackspaceWicWritePhrase
        | Action::PreviewWicDeviceWrite
        | Action::CancelWicWritePhrase
        | Action::ConfirmWicDeviceWrite
        | Action::CancelWicWritePreview
        | Action::StartConfirmedWicCreate(..)
        | Action::WicSessionStarting { .. }
        | Action::WicSessionRunning { .. }
        | Action::AppendWicSessionOutput { .. }
        | Action::CompleteWicSession { .. }
        | Action::FailWicSession { .. }
        | Action::LoseWicSession { .. }
        | Action::ConfirmWicSessionCancellation { .. } => {
            confirm_wic_device_selection_to_confirm_wic_session_cancellation::reduce_actions(
                app, action,
            )
        }
        Action::RejectWicSessionCancellation { .. }
        | Action::CancelWicSession { .. }
        | Action::BeginBuildTargetEdit
        | Action::BeginBuildTargetTask(..)
        | Action::ToggleBuildTargetEdit
        | Action::AppendBuildTarget(..)
        | Action::BackspaceBuildTarget
        | Action::CancelBuildTargetEdit
        | Action::ConfirmBuildTarget
        | Action::Start(..)
        | Action::QueueBackgroundJob(..)
        | Action::StartBackgroundJob { .. }
        | Action::RunBackgroundJob { .. }
        | Action::UpdateBackgroundJobProgress { .. }
        | Action::AppendBackgroundJobOutput { .. }
        | Action::RequestBackgroundJobCancellation { .. }
        | Action::RejectBackgroundJobCancellation { .. }
        | Action::SucceedBackgroundJob { .. }
        | Action::FailBackgroundJob { .. }
        | Action::CancelBackgroundJob { .. }
        | Action::LoseBackgroundJob { .. }
        | Action::BuildRequested { .. }
        | Action::SstateSummary(..)
        | Action::BuildStarted
        | Action::TaskStats(..)
        | Action::ParseProgress { .. }
        | Action::TaskStarted(..)
        | Action::TaskQueued(..)
        | Action::TaskProgress { .. }
        | Action::TaskCompleted { .. }
        | Action::TaskEvents(..)
        | Action::ScrollBuildTasks { .. }
        | Action::CycleTaskStateFilter
        | Action::CycleTaskFilterField
        | Action::BeginTaskFilterEdit
        | Action::AppendTaskFilter(..)
        | Action::BackspaceTaskFilter
        | Action::FinishTaskFilterEdit
        | Action::CycleTaskDurationFilter
        | Action::Log(..)
        | Action::Logs(..) => reject_wic_session_cancellation_to_logs::reduce_actions(app, action),
        Action::BuildCompleted { .. } | Action::BuildAuthorityLost { .. } => {
            build_completed_to_build_authority_lost::reduce_actions(app, action)
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
