pub(crate) fn context_action_local_disabled_reason(
    app: &App,
    destination: WorkspaceDestination,
    action_id: &str,
) -> Option<String> {
    let workspace_required = matches!(
        destination,
        WorkspaceDestination::Dashboard
            | WorkspaceDestination::Recipes
            | WorkspaceDestination::Layers
            | WorkspaceDestination::Configuration
            | WorkspaceDestination::Tasks
            | WorkspaceDestination::Dependencies
            | WorkspaceDestination::Signatures
            | WorkspaceDestination::Packages
            | WorkspaceDestination::Images
            | WorkspaceDestination::Sdk
            | WorkspaceDestination::Testing
            | WorkspaceDestination::Security
            | WorkspaceDestination::Qa
            | WorkspaceDestination::RawMode
            | WorkspaceDestination::Devtool
            | WorkspaceDestination::QemuWic
            | WorkspaceDestination::Maintenance
    );
    if workspace_required && app.workspace.build_dir.is_none() {
        return Some("Load a Yocto workspace first.".into());
    }
    if destination == WorkspaceDestination::Recipes
        && action_id != "recipes.metadata"
        && app.workspace.recipes.get(app.recipe_selection).is_none()
    {
        return Some("Select a recipe first.".into());
    }
    if destination == WorkspaceDestination::Packages
        && action_id != "packages.inventory"
        && action_id != "packages.cancel"
        && app.selected_package().is_none()
    {
        return Some("Select a package first.".into());
    }
    if destination == WorkspaceDestination::Images
        && matches!(
            action_id,
            "images.build" | "images.qemu" | "images.wic" | "images.device_write"
        )
        && app.selected_image_artifact().is_none()
    {
        return Some("Select a deployed image artifact first.".into());
    }
    if matches!(action_id, "dashboard.cancel" | "tasks.cancel")
        && !matches!(
            app.build.status,
            BuildStatus::LoadingWorkspace
                | BuildStatus::Parsing
                | BuildStatus::Running
                | BuildStatus::Cancelling
        )
    {
        return Some("No active build is available to cancel.".into());
    }
    None
}
pub(crate) fn contains_case_insensitive(value: &str, query: &str) -> bool {
    query.is_empty() || value.to_lowercase().contains(&query.to_lowercase())
}
pub(crate) fn task_state_order(state: TaskState) -> u8 {
    match state {
        TaskState::Active => 0,
        TaskState::Queued => 1,
        TaskState::Waiting => 2,
        TaskState::Failed | TaskState::Cancelled | TaskState::Lost => 3,
        TaskState::Completed => 4,
    }
}
