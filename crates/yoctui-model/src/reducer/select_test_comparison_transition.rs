//! State transitions beginning with SelectTestComparisonTransition.
use super::*;

mod qemu_capability_loaded_to_qemu_session_starting;
mod qemu_session_running_to_append_qemu_session_output;
mod select_test_comparison_transition_to_inspect_qemu_capability;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::SelectTestComparisonTransition { .. }
        | Action::OpenSelectedTestTransitionLog
        | Action::BeginTestJunitExport
        | Action::ToggleTestJunitTomlEditor
        | Action::AppendTestJunitTomlEditor(..)
        | Action::BackspaceTestJunitTomlEditor
        | Action::MoveTestJunitTomlEditorLeft
        | Action::MoveTestJunitTomlEditorRight
        | Action::MoveTestJunitTomlEditorUp
        | Action::MoveTestJunitTomlEditorDown
        | Action::MoveTestJunitTomlEditorHome
        | Action::MoveTestJunitTomlEditorEnd
        | Action::SelectTestJunitDestination
        | Action::CopyTestJunitTomlEditor
        | Action::PasteTestJunitTomlEditor
        | Action::AppendTestJunitDestination(..)
        | Action::BackspaceTestJunitDestination
        | Action::PreviewTestJunitExport
        | Action::CancelTestJunitExport
        | Action::TestJunitDestinationInspected { .. }
        | Action::CancelTestJunitExportPreview
        | Action::ConfirmTestJunitExport
        | Action::TestJunitExportSucceeded { .. }
        | Action::TestJunitExportFailed { .. }
        | Action::TestJunitExportCancelled { .. }
        | Action::TestJunitExportTimedOut { .. }
        | Action::TestJunitExportLost { .. }
        | Action::InspectQemuCapability => {
            select_test_comparison_transition_to_inspect_qemu_capability::reduce_actions(
                app, action,
            )
        }
        Action::QemuCapabilityLoaded(..)
        | Action::SshClientCapabilityDetected(..)
        | Action::BeginSelectedImageConsole
        | Action::SelectImageConsoleField { .. }
        | Action::CycleImageConsoleChoice { .. }
        | Action::AppendImageConsoleField(..)
        | Action::BackspaceImageConsoleField
        | Action::ConfirmImageConsole
        | Action::CancelImageConsole
        | Action::BeginSelectedQemuLaunch
        | Action::UpdateQemuLaunchDraft(..)
        | Action::SelectQemuLaunchField { .. }
        | Action::ActivateQemuLaunchField
        | Action::CycleQemuLaunchChoice { .. }
        | Action::AppendQemuLaunchField(..)
        | Action::BackspaceQemuLaunchField
        | Action::FinishQemuLaunchFieldEdit
        | Action::PreviewQemuLaunch
        | Action::CancelQemuLaunch
        | Action::CancelQemuLaunchPreview
        | Action::ConfirmQemuLaunchInTerminal
        | Action::ConfirmQemuLaunch
        | Action::QemuSessionStarting { .. } => {
            qemu_capability_loaded_to_qemu_session_starting::reduce_actions(app, action)
        }
        Action::QemuSessionRunning { .. } | Action::AppendQemuSessionOutput { .. } => {
            qemu_session_running_to_append_qemu_session_output::reduce_actions(app, action)
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
