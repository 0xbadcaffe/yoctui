use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::BeginSelectedRecipePatchReview => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected for patch review.".into());
                return None;
            };
            let recipe_name = recipe.name.clone();
            let Some(patches) = app
                .recipe_metadata
                .get(&recipe_name)
                .and_then(|metadata| metadata.patches.as_ref())
            else {
                app.notification = Some(format!(
                    "Load authoritative metadata for {recipe_name} with Enter before reviewing patches."
                ));
                return None;
            };
            let mut local_patches = patches
                .iter()
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .collect::<Vec<_>>();
            local_patches.sort();
            local_patches.dedup();
            if local_patches.is_empty() {
                app.notification = Some(if patches.is_empty() {
                    format!("BitBake reported no patches for {recipe_name}.")
                } else {
                    format!(
                        "The patches for {recipe_name} are remote or unresolved; no authoritative local path is available."
                    )
                });
            } else {
                open_dialog(
                    app,
                    Dialog::RecipePatchPicker(RecipePatchPicker {
                        recipe: recipe_name,
                        patches: local_patches,
                        selection: 0,
                    }),
                );
            }
        }
        Action::SelectRecipePatch { delta } => {
            if let Some(Dialog::RecipePatchPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.patches.len().saturating_sub(1))
                };
            }
        }
        Action::OpenSelectedRecipePatch => {
            if let Some(Dialog::RecipePatchPicker(picker)) = app.active_dialog()
                && let Some(path) = picker.patches.get(picker.selection).cloned()
            {
                close_dialog(app);
                return Some(Effect::OpenInEditor(path));
            }
        }
        Action::CancelRecipePatchPicker => {
            if matches!(app.active_dialog(), Some(Dialog::RecipePatchPicker(_))) {
                close_dialog(app);
            }
        }
        Action::BeginSelectedRecipeDevtoolModify => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification =
                    Some("Refresh authoritative Devtool status with t before modifying.".into());
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::ModifyOrEdit) {
                app.notification = Some(reason);
                return None;
            }
            if let DevtoolWorkspace::Present { source_path, .. } = &status.workspace {
                return Some(Effect::OpenWorkspaceEditor {
                    label: identity.name,
                    root: source_path.clone(),
                });
            }
            open_dialog(app, Dialog::DevtoolModifyConfirmation(identity));
        }
        Action::BeginSelectedRecipeDevtoolStatus => match selected_recipe_identity(app) {
            Ok(identity) => {
                app.devtool_status_loading.insert(identity.clone());
                return Some(Effect::InspectDevtoolStatus(identity));
            }
            Err(message) => app.notification = Some(message.into()),
        },
        Action::DevtoolStatusLoaded(status) => {
            app.devtool_status_loading.remove(&status.identity);
            app.devtool_statuses.insert(status.identity.clone(), status);
        }
        Action::BeginSelectedRecipeDevtoolReset => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification =
                    Some("Refresh authoritative Devtool status with t before reset.".into());
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::Reset) {
                app.notification = Some(reason);
                return None;
            }
            let source_path = match &status.workspace {
                DevtoolWorkspace::Present { source_path, .. }
                | DevtoolWorkspace::MissingDirectory { source_path } => source_path.clone(),
                DevtoolWorkspace::NotMember => return None,
            };
            if !source_path.is_absolute() {
                app.notification =
                    Some("The authoritative Devtool reset source path is not absolute.".into());
                return None;
            }
            open_dialog(
                app,
                Dialog::DevtoolResetConfirmation(DevtoolResetPlan {
                    identity,
                    source_path,
                }),
            );
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
