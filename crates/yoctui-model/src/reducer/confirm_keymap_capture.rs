//! State transitions beginning with ConfirmKeymapCapture.
use super::*;

mod begin_build_environment_edit_to_cycle_pane_subfocus;
mod confirm_keymap_capture_to_raw_mode;
mod edit_active_popup_to_configure_build_environment;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::ConfirmKeymapCapture
        | Action::CancelKeymapCapture
        | Action::RemoveKeymapBinding
        | Action::ResetKeymapBinding
        | Action::ResetAllKeymapBindings
        | Action::ExportEffectiveKeymap
        | Action::SettingsPersisted
        | Action::SettingsPersistenceFailed(..)
        | Action::SetCompatibilityFilter(..)
        | Action::SelectCompatibilityCapability { .. }
        | Action::BeginCompatibilitySearch
        | Action::AppendCompatibilityQuery(..)
        | Action::BackspaceCompatibilityQuery
        | Action::ClearCompatibilityQuery
        | Action::FinishCompatibilitySearch
        | Action::RawMode(..) => confirm_keymap_capture_to_raw_mode::reduce_actions(app, action),
        Action::EditActivePopup(..)
        | Action::OpenBuildEnvironmentCloneEditor
        | Action::ToggleBuildEnvironmentCloneEditor
        | Action::AppendBuildEnvironmentCloneEditor(..)
        | Action::BackspaceBuildEnvironmentCloneEditor
        | Action::ReviewBuildEnvironmentClone
        | Action::ConfirmBuildEnvironmentClone
        | Action::CancelBuildEnvironmentClone
        | Action::EnvironmentSetup(..)
        | Action::OpenBuildEnvironmentEditor
        | Action::ToggleBuildEnvironmentEditor
        | Action::AppendBuildEnvironmentEditor(..)
        | Action::BackspaceBuildEnvironmentEditor
        | Action::ApplyBuildEnvironmentEditor
        | Action::CloseBuildEnvironmentEditor
        | Action::OpenThemePicker
        | Action::SelectTheme { .. }
        | Action::ApplySelectedTheme
        | Action::CloseThemePicker
        | Action::ConfigureBuildEnvironment(..) => {
            edit_active_popup_to_configure_build_environment::reduce_actions(app, action)
        }
        Action::BeginBuildEnvironmentEdit
        | Action::SelectBuildEnvironmentField { .. }
        | Action::AppendBuildEnvironmentField(..)
        | Action::BackspaceBuildEnvironmentField
        | Action::FinishBuildEnvironmentEdit
        | Action::CancelBuildEnvironmentEdit
        | Action::ApplyBuildEnvironmentProfile
        | Action::BeginBuildEnvironmentVerification
        | Action::BuildEnvironmentVerified { .. }
        | Action::BuildEnvironmentVerificationFailed { .. }
        | Action::CycleFocus { .. }
        | Action::CyclePaneSubfocus { .. } => {
            begin_build_environment_edit_to_cycle_pane_subfocus::reduce_actions(app, action)
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
