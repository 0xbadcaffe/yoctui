//! State transitions beginning with BuildCancelled.
use super::*;

mod begin_selected_recipe_patch_review_to_begin_selected_recipe_devtool_reset;
mod build_cancelled_to_open_selected_error_source;
mod select_recipe_to_cancel_recipe_task_log_picker;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::BuildCancelled { .. }
        | Action::BuildCancellationRejected(..)
        | Action::DismissBuildCompletion
        | Action::OpenBuildCompletionErrors
        | Action::SelectBuildHistory { .. }
        | Action::Cancel
        | Action::ConfirmBuildCancellation
        | Action::CancelBuildCancellation
        | Action::CycleLogWorkspaceView
        | Action::InternalLog(..)
        | Action::InternalLogIngressDropped(..)
        | Action::ToggleInternalLogFollow
        | Action::ScrollInternalLogs { .. }
        | Action::BeginInternalLogSearch
        | Action::ClearInternalLogQuery
        | Action::FinishInternalLogSearch
        | Action::CycleInternalLogLevelFilter
        | Action::CycleInternalLogTargetFilter
        | Action::ClearInternalLogs
        | Action::ExportInternalLogs
        | Action::ToggleLogFollow
        | Action::ToggleLogWrap
        | Action::CycleLogSeverity
        | Action::ScrollLogs { .. }
        | Action::BeginLogSearch
        | Action::ClearLogQuery
        | Action::FinishLogSearch
        | Action::ScrollLogsHorizontally { .. }
        | Action::CycleLogRecipeFilter
        | Action::CycleLogTaskFilter
        | Action::CycleLogBuildFilter
        | Action::CycleLogSourceFilter
        | Action::CycleLogTimeRange
        | Action::ToggleSelectedLogBookmark
        | Action::NextLogBookmark
        | Action::PreviousLogBookmark
        | Action::OpenSelectedLogSource
        | Action::CopySelectedLog
        | Action::ExportFilteredLogs
        | Action::SelectError { .. }
        | Action::JumpToSelectedError
        | Action::OpenSelectedErrorSource => build_cancelled_to_open_selected_error_source::reduce_actions(app, action),
        Action::SelectRecipe { .. }
        | Action::ScrollRecipePreview { .. }
        | Action::BeginSelectedRecipeBuild
        | Action::BeginSelectedRecipeClean
        | Action::BeginSelectedRecipeMenuConfig
        | Action::BeginSelectedRecipeCleanState
        | Action::BeginSelectedRecipeDevshell
        | Action::BeginSelectedRecipeDevtoolWorkspaceShell
        | Action::BeginSelectedRecipeDevtoolGitUi
        | Action::BeginSelectedRecipeDevtoolEditRecipe
        | Action::BeginSelectedRecipeDiffconfig
        | Action::BeginSelectedRecipeDiffsigs
        | Action::BeginSelectedRecipeSignatures
        | Action::BeginSelectedRecipeCveCheck
        | Action::BeginSelectedRecipeSpdx
        | Action::BeginSelectedRecipeTask { .. }
        | Action::BeginSelectedRecipeForceTask
        | Action::SelectRecipeTask { .. }
        | Action::PreviewSelectedRecipeTask
        | Action::CancelRecipeTaskPicker
        | Action::SelectSignatureTask { .. }
        | Action::ConfirmSignatureTask
        | Action::CancelSignatureTaskPicker
        | Action::OpenSelectedRecipeProvider
        | Action::BeginSelectedRecipeTaskLog
        | Action::SelectRecipeTaskLog { .. }
        | Action::OpenSelectedRecipeTaskLog
        | Action::CancelRecipeTaskLogPicker => select_recipe_to_cancel_recipe_task_log_picker::reduce_actions(app, action),
        Action::BeginSelectedRecipePatchReview
        | Action::SelectRecipePatch { .. }
        | Action::OpenSelectedRecipePatch
        | Action::CancelRecipePatchPicker
        | Action::BeginSelectedRecipeDevtoolModify
        | Action::BeginSelectedRecipeDevtoolStatus
        | Action::DevtoolStatusLoaded(..)
        | Action::BeginSelectedRecipeDevtoolReset => begin_selected_recipe_patch_review_to_begin_selected_recipe_devtool_reset::reduce_actions(app, action),
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
