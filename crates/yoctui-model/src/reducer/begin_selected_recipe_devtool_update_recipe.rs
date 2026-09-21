//! State transitions beginning with BeginSelectedRecipeDevtoolUpdateRecipe.
use super::*;

mod begin_selected_recipe_devtool_update_recipe_to_signature_dump_partial;
mod open_package_dependency_to_back_package_navigation;
mod signature_dump_failed_to_package_detail_failed;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::BeginSelectedRecipeDevtoolUpdateRecipe
        | Action::BeginSelectedRecipeDevtoolFinish
        | Action::BeginSelectedRecipeDevtoolDeploy
        | Action::BeginSelectedRecipeDependencies
        | Action::BeginDependencyGraph { .. }
        | Action::DependencyGraphLoaded(..)
        | Action::DependencyGraphPartial { .. }
        | Action::DependencyGraphFailed { .. }
        | Action::SelectDependencyGraphNode { .. }
        | Action::SelectDependencyGraphNodeAt { .. }
        | Action::ToggleDependencyGraphReverse
        | Action::CollapseSelectedDependencyGraphNode
        | Action::ExpandSelectedDependencyGraphNode
        | Action::ToggleSelectedDependencyGraphNode
        | Action::BeginDependencyGraphSearch
        | Action::AppendDependencyGraphQuery(..)
        | Action::BackspaceDependencyGraphQuery
        | Action::ClearDependencyGraphQuery
        | Action::FinishDependencyGraphSearch
        | Action::RefreshDependencyGraph
        | Action::OpenSelectedDependencyRecipe
        | Action::OpenSelectedDependencyProvider
        | Action::OpenSelectedDependencyTaskLog
        | Action::BeginSignatureDump(..)
        | Action::RefreshSignatureDump
        | Action::LeaveSignatureWorkspace
        | Action::OpenSignatureProvider
        | Action::SignatureDumpLoaded { .. }
        | Action::SignatureDumpPartial { .. } => {
            begin_selected_recipe_devtool_update_recipe_to_signature_dump_partial::reduce_actions(
                app, action,
            )
        }
        Action::SignatureDumpFailed { .. }
        | Action::SelectSignatureRecord { .. }
        | Action::SetSelectedSignatureComparisonSide(..)
        | Action::BeginSignatureComparison
        | Action::SignatureComparisonLoaded { .. }
        | Action::SignatureComparisonPartial { .. }
        | Action::SignatureComparisonFailed { .. }
        | Action::BeginPackageInventory
        | Action::RefreshPackageInventory
        | Action::CancelPackageOperation
        | Action::PackageInventoryLoaded { .. }
        | Action::PackageInventoryPartial { .. }
        | Action::PackageInventoryFailed { .. }
        | Action::SelectPackage { .. }
        | Action::BeginPackageSearch
        | Action::AppendPackageQuery(..)
        | Action::BackspacePackageQuery
        | Action::ClearPackageQuery
        | Action::FinishPackageSearch
        | Action::BeginSelectedPackageDetail
        | Action::PackageDetailLoaded { .. }
        | Action::PackageDetailPartial { .. }
        | Action::PackageDetailFailed { .. } => {
            signature_dump_failed_to_package_detail_failed::reduce_actions(app, action)
        }
        Action::OpenPackageDependency { .. }
        | Action::TogglePackageDependencyKind
        | Action::SelectPackageDependency { .. }
        | Action::OpenSelectedPackageDependency
        | Action::BackPackageNavigation => {
            open_package_dependency_to_back_package_navigation::reduce_actions(app, action)
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
