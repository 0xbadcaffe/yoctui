//! State transitions beginning with ResetPaneSubfocus.
use super::*;

mod backspace_sdk_publish_toml_editor_to_preview_sdk_publish;
mod reset_pane_subfocus_to_rootfs_composition_unavailable;
mod rootfs_composition_failed_to_append_sdk_publish_toml_editor;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::ResetPaneSubfocus
        | Action::TogglePaneZoom
        | Action::OpenBuildOptions
        | Action::CloseBuildOptions
        | Action::OpenImagePicker(..)
        | Action::SelectImage { .. }
        | Action::ConfirmImagePicker
        | Action::CancelImagePicker
        | Action::BeginCurrentImageBuild
        | Action::BeginImageArtifactInventory
        | Action::RefreshImageArtifactInventory
        | Action::CancelImageArtifactOperation
        | Action::ImageArtifactInventoryLoaded { .. }
        | Action::ImageArtifactInventoryPartial { .. }
        | Action::ImageArtifactInventoryFailed { .. }
        | Action::SelectImageArtifact { .. }
        | Action::BeginImageArtifactSearch
        | Action::AppendImageArtifactQuery(..)
        | Action::BackspaceImageArtifactQuery
        | Action::ClearImageArtifactQuery
        | Action::FinishImageArtifactSearch
        | Action::BeginSelectedImageArtifactBuild
        | Action::OpenSelectedImageArtifact
        | Action::OpenSelectedImageArtifactAssociation(..)
        | Action::BeginSelectedRootfsComposition
        | Action::ShiftImagesView { .. }
        | Action::RefreshRootfsComposition
        | Action::RootfsCompositionLoaded { .. }
        | Action::RootfsCompositionPartial { .. }
        | Action::RootfsCompositionUnavailable { .. } => {
            reset_pane_subfocus_to_rootfs_composition_unavailable::reduce_actions(app, action)
        }
        Action::RootfsCompositionFailed { .. }
        | Action::SelectRootfsGroup { .. }
        | Action::SelectRootfsPackage { .. }
        | Action::SelectRootfsEntry { .. }
        | Action::SelectRootfsSystemdService { .. }
        | Action::SelectRootfsDbusService { .. }
        | Action::SelectRootfsUdevRule { .. }
        | Action::ScrollRootfsUdevPreview { .. }
        | Action::BrowseRootfsFilesystem
        | Action::EditSelectedRootfsSystemFile
        | Action::BeginSdkBuild(..)
        | Action::ConfirmSdkBuild
        | Action::CancelSdkBuild
        | Action::BeginSdkArtifactInventory
        | Action::RefreshSdkArtifactInventory
        | Action::SdkArtifactInventoryLoaded { .. }
        | Action::SdkArtifactInventoryFailed { .. }
        | Action::SelectSdkArtifact { .. }
        | Action::BeginSdkArtifactSearch
        | Action::AppendSdkArtifactQuery(..)
        | Action::BackspaceSdkArtifactQuery
        | Action::ClearSdkArtifactQuery
        | Action::FinishSdkArtifactSearch
        | Action::OpenSelectedSdkArtifact
        | Action::SdkToolCapabilityLoaded(..)
        | Action::BeginSelectedSdkPublish
        | Action::ToggleSdkPublishTomlEditor
        | Action::AppendSdkPublishTomlEditor(..) => {
            rootfs_composition_failed_to_append_sdk_publish_toml_editor::reduce_actions(app, action)
        }
        Action::BackspaceSdkPublishTomlEditor
        | Action::AppendSdkPublishDestination(..)
        | Action::BackspaceSdkPublishDestination
        | Action::PreviewSdkPublish => {
            backspace_sdk_publish_toml_editor_to_preview_sdk_publish::reduce_actions(app, action)
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
