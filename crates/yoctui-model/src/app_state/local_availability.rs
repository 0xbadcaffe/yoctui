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
    if matches!(
        action_id,
        "recipes.devtool_gitui" | "devtool.gitui" | "devtool.shell" | "devtool.build"
    ) {
        let identity = match selected_recipe_identity(app) {
            Ok(identity) => identity,
            Err(message) => return Some(message.into()),
        };
        if matches!(action_id, "recipes.devtool_gitui" | "devtool.gitui")
            && app.gitui_program.is_none()
        {
            return Some("Install GitUI first.".into());
        }
        let Some(status) = app.devtool_statuses.get(&identity) else {
            return Some("Refresh Devtool status after running modify.".into());
        };
        if status.capability != DevtoolCapability::Available || status.error.is_some() {
            return Some(
                status
                    .disabled_reason(DevtoolAction::ModifyOrEdit)
                    .unwrap_or_else(|| "The Devtool workspace is unavailable.".into()),
            );
        }
        match &status.workspace {
            DevtoolWorkspace::Present { source_path, .. } if source_path.is_absolute() => {}
            DevtoolWorkspace::Present { .. } => {
                return Some("The Devtool workspace source path is not absolute.".into());
            }
            DevtoolWorkspace::NotMember | DevtoolWorkspace::MissingDirectory { .. } => {
                return Some("Run Devtool modify first.".into());
            }
        }
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
