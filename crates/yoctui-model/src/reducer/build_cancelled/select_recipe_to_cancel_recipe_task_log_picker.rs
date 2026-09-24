use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::SelectRecipe { delta } => {
            let matches = app
                .workspace
                .recipes
                .iter()
                .enumerate()
                .filter(|(_, recipe)| recipe_matches_query(recipe, &app.metadata_query))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let position = matches
                .iter()
                .position(|index| *index == app.recipe_selection)
                .unwrap_or(0);
            let position = shifted_index(position, delta, matches.len());
            app.recipe_selection = matches.get(position).copied().unwrap_or(0);
            app.recipe_preview_scroll = 0;
        }
        Action::ScrollRecipePreview { delta } => {
            app.recipe_preview_scroll = if delta.is_negative() {
                app.recipe_preview_scroll
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.recipe_preview_scroll
                    .saturating_add(delta as usize)
                    .min(4_095)
            };
        }
        Action::BeginSelectedRecipeBuild => {
            begin_recipe_task(app, None, false);
        }
        Action::BeginSelectedRecipeClean => {
            begin_recipe_task(app, Some("clean".into()), false);
        }
        Action::BeginSelectedRecipeMenuConfig => {
            begin_terminal_creation(app, Some("menuconfig"));
        }
        Action::BeginSelectedRecipeCleanState => {
            begin_recipe_task(app, Some("cleansstate".into()), false);
        }
        Action::BeginSelectedRecipeDevshell => {
            begin_terminal_creation(app, Some("devshell"));
        }
        Action::BeginSelectedRecipeDevtoolWorkspaceShell => {
            if let Some(request) = devtool_terminal_request(app, false) {
                open_terminal_launch(app, request);
            }
        }
        Action::BeginSelectedRecipeDevtoolGitUi => {
            if let Some(request) = devtool_gitui_request(app) {
                open_terminal_launch(app, request);
            }
        }
        Action::BeginSelectedRecipeDevtoolEditRecipe => {
            if let Some(request) = devtool_terminal_request(app, true) {
                open_terminal_launch(app, request);
            }
        }
        Action::BeginSelectedRecipeDiffconfig => {
            begin_recipe_task(app, Some("diffconfig".into()), false);
        }
        Action::BeginSelectedRecipeDiffsigs => {
            begin_recipe_task(app, Some("diffsigs".into()), false);
        }
        Action::BeginSelectedRecipeSignatures => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.replace("Devtool status", "signatures"));
                    return None;
                }
            };
            let Some(tasks) = app
                .recipe_metadata
                .get(&identity.name)
                .and_then(|metadata| metadata.tasks.as_ref())
            else {
                app.notification = Some(
                    "Load authoritative recipe tasks with Enter before inspecting signatures."
                        .into(),
                );
                return None;
            };
            let mut tasks = tasks
                .iter()
                .filter(|task| {
                    SignatureTarget {
                        recipe: identity.name.clone(),
                        task: (*task).clone(),
                    }
                    .validate()
                    .is_ok()
                })
                .cloned()
                .collect::<Vec<_>>();
            tasks.sort();
            tasks.dedup();
            if tasks.is_empty() {
                app.notification =
                    Some("BitBake reported no valid signature tasks for this recipe.".into());
                return None;
            }
            open_dialog(
                app,
                Dialog::SignatureTaskPicker(SignatureTaskPicker {
                    recipe: identity,
                    tasks,
                    selection: 0,
                }),
            );
        }
        Action::BeginSelectedRecipeCveCheck => {
            begin_recipe_task(app, Some("cve_check".into()), false);
        }
        Action::BeginSelectedRecipeSpdx => {
            begin_recipe_task(app, Some("create_spdx".into()), false);
        }
        Action::BeginSelectedRecipeTask { task, force } => {
            begin_recipe_task(app, task, force);
        }
        Action::BeginSelectedRecipeForceTask => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected for forced task execution.".into());
                return None;
            };
            let Some(tasks) = app
                .recipe_metadata
                .get(&recipe.name)
                .and_then(|metadata| metadata.tasks.as_ref())
            else {
                app.notification = Some(
                    "Load authoritative recipe tasks with Enter before forcing a task.".into(),
                );
                return None;
            };
            let mut tasks = tasks
                .iter()
                .map(|task| task.strip_prefix("do_").unwrap_or(task).to_owned())
                .filter(|task| {
                    !task.is_empty()
                        && task.chars().all(|character| {
                            character.is_ascii_alphanumeric()
                                || matches!(character, '-' | '_' | '.' | '+')
                        })
                })
                .collect::<Vec<_>>();
            tasks.sort();
            tasks.dedup();
            if tasks.is_empty() {
                app.notification =
                    Some("BitBake reported no forceable tasks for this recipe.".into());
            } else {
                open_dialog(
                    app,
                    Dialog::RecipeTaskPicker(RecipeTaskPicker {
                        recipe: recipe.name.clone(),
                        tasks,
                        selection: 0,
                        force: true,
                    }),
                );
            }
        }
        Action::SelectRecipeTask { delta } => {
            if let Some(Dialog::RecipeTaskPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.tasks.len().saturating_sub(1))
                };
            }
        }
        Action::PreviewSelectedRecipeTask => {
            if let Some(Dialog::RecipeTaskPicker(picker)) = app.active_dialog()
                && let Some(task) = picker.tasks.get(picker.selection)
            {
                let request = BuildRequest {
                    targets: vec![picker.recipe.clone()],
                    task: Some(task.clone()),
                    force: picker.force,
                };
                replace_dialog(app, Dialog::RecipeTaskConfirmation(request));
            }
        }
        Action::CancelRecipeTaskPicker => {
            if matches!(app.active_dialog(), Some(Dialog::RecipeTaskPicker(_))) {
                close_dialog(app);
            }
        }
        Action::SelectSignatureTask { delta } => {
            if let Some(Dialog::SignatureTaskPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.tasks.len().saturating_sub(1))
                };
            }
        }
        Action::ConfirmSignatureTask => {
            let Some(Dialog::SignatureTaskPicker(picker)) = app.active_dialog().cloned() else {
                return None;
            };
            let Some(task) = picker.tasks.get(picker.selection).cloned() else {
                app.notification = Some("No authoritative signature task is selected.".into());
                return None;
            };
            let target = SignatureTarget {
                recipe: picker.recipe.name.clone(),
                task,
            };
            if let Err(message) = target.validate() {
                app.notification = Some(message.into());
                return None;
            }
            close_dialog(app);
            app.screen = Screen::Signatures;
            app.focus = FocusTarget::Workspace;
            app.focus_return = None;
            app.signature_recipe = Some(picker.recipe);
            app.signature_selection = None;
            app.signature_comparison = SignatureComparisonState::NotSelected;
            return begin_signature_dump(app, target);
        }
        Action::CancelSignatureTaskPicker => {
            if matches!(app.active_dialog(), Some(Dialog::SignatureTaskPicker(_))) {
                close_dialog(app);
            }
        }
        Action::OpenSelectedRecipeProvider => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected to open.".into());
                return None;
            };
            if let Some(path) = recipe.file.clone() {
                return Some(Effect::OpenInEditor(path));
            }
            app.notification = Some(format!(
                "BitBake did not report an authoritative provider path for {}.",
                recipe.name
            ));
        }
        Action::BeginSelectedRecipeTaskLog => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected for task-log inspection.".into());
                return None;
            };
            let recipe_name = recipe.name.clone();
            let mut logs = app
                .tasks
                .values()
                .chain(app.completed_tasks.iter().map(|completed| &completed.task))
                .filter(|task| task.recipe == recipe_name)
                .filter_map(|task| {
                    task.log_path.clone().map(|path| RecipeTaskLogChoice {
                        task: task.task.clone(),
                        state: task.state,
                        path,
                    })
                })
                .collect::<Vec<_>>();
            logs.sort_by(|left, right| {
                left.task
                    .cmp(&right.task)
                    .then_with(|| left.path.cmp(&right.path))
            });
            logs.dedup_by(|left, right| left.path == right.path);
            match logs.len() {
                0 => {
                    app.notification = Some(format!(
                        "No retained task log path is available for {recipe_name}; BitBake may not have reported one or it may have been evicted."
                    ));
                }
                1 => return Some(Effect::OpenInEditor(logs.remove(0).path)),
                _ => open_dialog(
                    app,
                    Dialog::RecipeTaskLogPicker(RecipeTaskLogPicker {
                        recipe: recipe_name,
                        logs,
                        selection: 0,
                    }),
                ),
            }
        }
        Action::SelectRecipeTaskLog { delta } => {
            if let Some(Dialog::RecipeTaskLogPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.logs.len().saturating_sub(1))
                };
            }
        }
        Action::OpenSelectedRecipeTaskLog => {
            if let Some(Dialog::RecipeTaskLogPicker(picker)) = app.active_dialog()
                && let Some(path) = picker
                    .logs
                    .get(picker.selection)
                    .map(|choice| choice.path.clone())
            {
                close_dialog(app);
                return Some(Effect::OpenInEditor(path));
            }
        }
        Action::CancelRecipeTaskLogPicker => {
            if matches!(app.active_dialog(), Some(Dialog::RecipeTaskLogPicker(_))) {
                close_dialog(app);
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
