//! State transitions beginning with CancelSdkPublish.
use super::*;

mod append_test_session_output_to_complete_test_session;
mod cancel_sdk_publish_to_fail_sdk_session;
mod lose_sdk_session_to_test_session_running;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::CancelSdkPublish
        | Action::CancelSdkPublishPreview
        | Action::ConfirmSdkPublish
        | Action::BeginSdkNative
        | Action::ToggleSdkNativeTomlEditor
        | Action::AppendSdkNativeTomlEditor(..)
        | Action::BackspaceSdkNativeTomlEditor
        | Action::UpdateSdkNativeDraft(..)
        | Action::SelectSdkNativeField { .. }
        | Action::ActivateSdkNativeField
        | Action::CycleSdkNativeMode
        | Action::AppendSdkNativeField(..)
        | Action::BackspaceSdkNativeField
        | Action::FinishSdkNativeFieldEdit
        | Action::PreviewSdkNative
        | Action::CancelSdkNative
        | Action::CancelSdkNativePreview
        | Action::ConfirmSdkNative
        | Action::SdkSessionStarting { .. }
        | Action::SdkSessionRunning { .. }
        | Action::AppendSdkSessionOutput { .. }
        | Action::CompleteSdkSession { .. }
        | Action::FailSdkSession { .. } => {
            cancel_sdk_publish_to_fail_sdk_session::reduce_actions(app, action)
        }
        Action::LoseSdkSession { .. }
        | Action::BeginActiveSdkSessionCancellation
        | Action::ConfirmSdkSessionCancellation
        | Action::CancelSdkSessionCancellation
        | Action::RejectSdkSessionCancellation { .. }
        | Action::CancelSdkSession { .. }
        | Action::InspectTestCapability
        | Action::TestCapabilityLoaded(..)
        | Action::SelectTestFamily { .. }
        | Action::BeginSelectedTestLaunch
        | Action::ToggleTestLaunchTomlEditor
        | Action::AppendTestLaunchTomlEditor(..)
        | Action::BackspaceTestLaunchTomlEditor
        | Action::UpdateTestLaunchDraft(..)
        | Action::SelectTestLaunchField { .. }
        | Action::ActivateTestLaunchField
        | Action::AppendTestLaunchField(..)
        | Action::BackspaceTestLaunchField
        | Action::FinishTestLaunchFieldEdit
        | Action::PreviewTestLaunch
        | Action::CancelTestLaunch
        | Action::CancelTestLaunchPreview
        | Action::ConfirmTestLaunch
        | Action::AttachTestBuildSession { .. }
        | Action::TestSessionStarting { .. }
        | Action::TestSessionRunning { .. } => {
            lose_sdk_session_to_test_session_running::reduce_actions(app, action)
        }
        Action::AppendTestSessionOutput { .. } | Action::CompleteTestSession { .. } => {
            append_test_session_output_to_complete_test_session::reduce_actions(app, action)
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
