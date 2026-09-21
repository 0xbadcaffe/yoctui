//! State transitions beginning with SetLayerInspectorMode.
use super::*;

mod config_edit_refresh_failed_to_tick;
mod set_layer_inspector_mode_to_config_edit_refresh_succeeded;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::SetLayerInspectorMode(..)
        | Action::ScrollLayerBrowserPreview { .. }
        | Action::FocusLayerBrowserTree
        | Action::LoadLayerBrowserPreview { .. }
        | Action::EditSelectedLayerBrowserFile
        | Action::BeginLayerRelationships
        | Action::LayerRelationshipsLoaded(..)
        | Action::SelectConfigVariable { .. }
        | Action::BeginSelectedConfigDetail
        | Action::CopySelectedConfigEffective
        | Action::CopySelectedConfigUnexpanded
        | Action::VariableDetailFailed { .. }
        | Action::OpenSelectedConfigSource
        | Action::SelectConfigSource { .. }
        | Action::OpenSelectedConfigSourceChoice
        | Action::CancelConfigSourcePicker
        | Action::OpenConfigScopePicker
        | Action::SelectConfigScope { .. }
        | Action::ConfirmConfigScope
        | Action::CancelConfigScopePicker
        | Action::OpenConfigComparison
        | Action::CloseConfigComparison
        | Action::BeginConfigEdit
        | Action::ToggleConfigEdit
        | Action::AppendConfigEdit(..)
        | Action::BackspaceConfigEdit
        | Action::PreviewConfigEdit
        | Action::CancelConfigEdit
        | Action::ConfirmConfigEdit
        | Action::CancelConfigEditConfirmation
        | Action::ConfigEditWriteSucceeded { .. }
        | Action::ConfigEditWriteFailed { .. }
        | Action::ConfigEditRefreshSucceeded { .. } => {
            set_layer_inspector_mode_to_config_edit_refresh_succeeded::reduce_actions(app, action)
        }
        Action::ConfigEditRefreshFailed { .. }
        | Action::BeginBbmaskEdit
        | Action::ToggleBbmaskEdit
        | Action::AppendBbmask(..)
        | Action::BackspaceBbmask
        | Action::PreviewBbmaskEdit
        | Action::CancelBbmaskEdit
        | Action::ConfirmBbmaskWrite
        | Action::CancelBbmaskWrite
        | Action::BeginMetadataSearch
        | Action::ClearMetadataQuery
        | Action::FinishMetadataSearch
        | Action::Notify(..)
        | Action::ActivateNotification
        | Action::DismissNotification
        | Action::Quit
        | Action::ConfirmQuit
        | Action::CancelQuit
        | Action::WorkspaceLoaded(..)
        | Action::RecipesLoaded(..)
        | Action::LayersLoaded(..)
        | Action::VariableLoaded(..)
        | Action::RecipeSourcesLoaded { .. }
        | Action::HostTelemetryUpdated(..)
        | Action::Failure(..)
        | Action::Tick => config_edit_refresh_failed_to_tick::reduce_actions(app, action),
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
