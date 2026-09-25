//! State transitions beginning with OpenSelectedPackageRecipe.
use super::*;

mod layer_browser_enter_to_toggle_layer_browser_hidden;
mod open_recipe_editor_to_layer_browser_expand;
mod open_selected_package_recipe_to_cancel_devtool_deploy_confirmation;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::OpenSelectedPackageRecipe
        | Action::OpenSelectedPackageProvider
        | Action::BeginSelectedRecipeMetadata
        | Action::RecipeMetadataLoaded(..)
        | Action::RecipeMetadataFailed { .. }
        | Action::DependenciesLoaded(..)
        | Action::SelectDependency { .. }
        | Action::OpenSelectedDependency
        | Action::ConfirmRecipeTask
        | Action::CancelRecipeTask
        | Action::ConfirmDevtoolModify
        | Action::CancelDevtoolModify
        | Action::ConfirmDevtoolReset
        | Action::CancelDevtoolReset
        | Action::ConfirmDevtoolUpdateRecipe
        | Action::CancelDevtoolUpdateRecipe
        | Action::SelectDevtoolPatchLayer { .. }
        | Action::PreviewDevtoolPatch
        | Action::CancelDevtoolPatch
        | Action::ConfirmDevtoolPatch
        | Action::CancelDevtoolPatchConfirmation
        | Action::SelectDevtoolFinishLayer { .. }
        | Action::PreviewDevtoolFinish
        | Action::CancelDevtoolFinish
        | Action::ConfirmDevtoolFinish
        | Action::CancelDevtoolFinishConfirmation
        | Action::AppendDevtoolDeployTarget(..)
        | Action::BackspaceDevtoolDeployTarget
        | Action::PreviewDevtoolDeploy
        | Action::CancelDevtoolDeploy
        | Action::ConfirmDevtoolDeploy
        | Action::CancelDevtoolDeployConfirmation => {
            open_selected_package_recipe_to_cancel_devtool_deploy_confirmation::reduce_actions(
                app, action,
            )
        }
        Action::AppendDevtoolUndeployTarget(..)
        | Action::BackspaceDevtoolUndeployTarget
        | Action::PreviewDevtoolUndeploy
        | Action::CancelDevtoolUndeploy
        | Action::ConfirmDevtoolUndeploy
        | Action::CancelDevtoolUndeployConfirmation
        | Action::ConfirmDevtoolUpgrade
        | Action::CancelDevtoolUpgrade => {
            open_selected_package_recipe_to_cancel_devtool_deploy_confirmation::reduce_actions(
                app, action,
            )
        }
        Action::OpenRecipeEditor { .. }
        | Action::SelectRecipeEditorFile { .. }
        | Action::LoadRecipeEditorContent(..)
        | Action::LoadRecipeEditorExternalContent(..)
        | Action::FocusRecipeEditor(..)
        | Action::EditRecipeEditor(..)
        | Action::BeginRecipeEditorSearch
        | Action::AppendRecipeEditorSearch(..)
        | Action::BackspaceRecipeEditorSearch
        | Action::FinishRecipeEditorSearch
        | Action::NextRecipeEditorMatch { .. }
        | Action::ToggleRecipeEditorEditing
        | Action::OpenRecipeEditorExternal
        | Action::AppendRecipeEditor(..)
        | Action::BackspaceRecipeEditor
        | Action::SaveRecipeEditor
        | Action::RecipeEditorSaved
        | Action::BeginRecipeEditorBuild
        | Action::CloseRecipeEditor
        | Action::SelectLayer { .. }
        | Action::OpenSelectedLayer
        | Action::BeginSelectedLayerWorkspaceEditor
        | Action::BeginSelectedLayerBrowser
        | Action::LoadLayerBrowserDirectory { .. }
        | Action::SelectLayerBrowserEntry { .. }
        | Action::LayerBrowserExpand => {
            open_recipe_editor_to_layer_browser_expand::reduce_actions(app, action)
        }
        Action::LayerBrowserEnter
        | Action::LayerBrowserUp
        | Action::CloseLayerBrowser
        | Action::RefreshLayerBrowser
        | Action::ToggleLayerBrowserHidden => {
            layer_browser_enter_to_toggle_layer_browser_hidden::reduce_actions(app, action)
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
