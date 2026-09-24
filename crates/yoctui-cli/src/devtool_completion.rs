//! Devtool completion.
use super::*;

pub(crate) async fn inspect_devtool_status(
    app: &App,
    build_dir: &Path,
    identity: RecipeIdentity,
) -> yoctui_model::DevtoolStatus {
    let authority = app.workspace_compatibility.authority().cloned();
    inspect_devtool_status_with_authority(build_dir, identity, authority).await
}

pub(crate) async fn inspect_devtool_status_with_authority(
    build_dir: &Path,
    identity: RecipeIdentity,
    authority: Option<yoctui_model::DaemonCompatibilitySnapshot>,
) -> yoctui_model::DevtoolStatus {
    let inspector = DevtoolInspector::default();
    match authority {
        Some(authority) => {
            inspector
                .inspect_with_compatibility(
                    build_dir,
                    identity,
                    &authority,
                    authority.snapshot.generation,
                )
                .await
        }
        None => inspector.inspect(build_dir, identity).await,
    }
}

pub(crate) async fn complete_devtool_modify(
    app: &mut App,
    build_dir: &Path,
    identity: RecipeIdentity,
) {
    let status = inspect_devtool_status(app, build_dir, identity).await;
    apply_completed_devtool_modify_status(app, status).await;
}

pub(crate) async fn apply_completed_devtool_modify_status(
    app: &mut App,
    status: yoctui_model::DevtoolStatus,
) {
    let editor = if let Some(error) = &status.error {
        app.notification = Some(format!(
            "Devtool modify completed, but authoritative status refresh failed: {error:?}"
        ));
        None
    } else {
        match &status.workspace {
            DevtoolWorkspace::Present { source_path, .. } => {
                Some((status.identity.name.clone(), source_path.clone()))
            }
            DevtoolWorkspace::MissingDirectory { source_path } => {
                app.notification = Some(format!(
                    "Devtool modify completed, but the reported workspace source is missing: {}",
                    source_path.display()
                ));
                None
            }
            DevtoolWorkspace::NotMember => {
                app.notification = Some(
                    "Devtool modify completed, but the recipe is not reported in the workspace."
                        .into(),
                );
                None
            }
        }
    };
    let _ = update(app, Action::DevtoolStatusLoaded(status));
    if let Some((recipe, root)) = editor {
        open_workspace_editor(app, recipe, root).await;
    }
}

pub(crate) async fn complete_devtool_update(
    app: &mut App,
    build_dir: &Path,
    identity: RecipeIdentity,
) {
    let status = inspect_devtool_status(app, build_dir, identity).await;
    apply_completed_devtool_update_status(app, status);
}

pub(crate) fn apply_completed_devtool_update_status(
    app: &mut App,
    status: yoctui_model::DevtoolStatus,
) {
    let notification = if let Some(error) = &status.error {
        format!(
            "Devtool update-recipe completed, but authoritative status refresh failed: {error:?}"
        )
    } else {
        match &status.workspace {
            DevtoolWorkspace::Present { .. } => {
                "Devtool update-recipe completed and workspace status was refreshed.".into()
            }
            DevtoolWorkspace::MissingDirectory { source_path } => format!(
                "Devtool update-recipe completed, but the reported workspace source is missing: {}",
                source_path.display()
            ),
            DevtoolWorkspace::NotMember => {
                "Devtool update-recipe completed, but the recipe is no longer reported in the workspace."
                    .into()
            }
        }
    };
    let _ = update(app, Action::DevtoolStatusLoaded(status));
    app.notification = Some(notification);
}

pub(crate) async fn complete_devtool_finish(
    app: &mut App,
    build_dir: &Path,
    identity: RecipeIdentity,
) {
    let status = inspect_devtool_status(app, build_dir, identity).await;
    apply_completed_devtool_finish_status(app, status);
}

pub(crate) fn apply_completed_devtool_finish_status(
    app: &mut App,
    status: yoctui_model::DevtoolStatus,
) {
    let notification = if let Some(error) = &status.error {
        format!("Devtool finish completed, but authoritative status refresh failed: {error:?}")
    } else if let DevtoolWorkspace::MissingDirectory { source_path } = &status.workspace {
        format!(
            "Devtool finish completed, but the refreshed workspace source is missing: {}",
            source_path.display()
        )
    } else {
        "Devtool finish completed and workspace status was refreshed.".into()
    };
    let _ = update(app, Action::DevtoolStatusLoaded(status));
    app.notification = Some(notification);
}

pub(crate) async fn complete_devtool_deploy(
    app: &mut App,
    build_dir: &Path,
    identity: RecipeIdentity,
) {
    let status = inspect_devtool_status(app, build_dir, identity).await;
    apply_completed_devtool_deploy_status(app, status);
}

pub(crate) fn apply_completed_devtool_deploy_status(
    app: &mut App,
    status: yoctui_model::DevtoolStatus,
) {
    let notification = if let Some(error) = &status.error {
        format!(
            "Devtool deploy-target completed, but authoritative status refresh failed: {error:?}"
        )
    } else if let DevtoolWorkspace::MissingDirectory { source_path } = &status.workspace {
        format!(
            "Devtool deploy-target completed, but the refreshed workspace source is missing: {}",
            source_path.display()
        )
    } else {
        "Devtool deploy-target completed and workspace status was refreshed.".into()
    };
    let _ = update(app, Action::DevtoolStatusLoaded(status));
    app.notification = Some(notification);
}

pub(crate) async fn complete_devtool_reset(
    app: &mut App,
    build_dir: &Path,
    identity: RecipeIdentity,
) {
    let status = inspect_devtool_status(app, build_dir, identity).await;
    apply_completed_devtool_reset_status(app, status);
}

pub(crate) fn apply_completed_devtool_reset_status(
    app: &mut App,
    status: yoctui_model::DevtoolStatus,
) {
    let notification = if let Some(error) = &status.error {
        format!("Devtool reset completed, but authoritative status refresh failed: {error:?}")
    } else {
        match &status.workspace {
            DevtoolWorkspace::NotMember => {
                "Devtool reset completed; the recipe is no longer in the workspace.".into()
            }
            DevtoolWorkspace::MissingDirectory { source_path } => format!(
                "Devtool reset completed, but the missing workspace is still reported: {}",
                source_path.display()
            ),
            DevtoolWorkspace::Present { source_path, .. } => format!(
                "Devtool reset completed, but the workspace is still reported at {}",
                source_path.display()
            ),
        }
    };
    let _ = update(app, Action::DevtoolStatusLoaded(status));
    app.notification = Some(notification);
}
