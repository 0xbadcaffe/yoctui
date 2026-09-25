use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::OpenSelectedPackageRecipe => {
            let Some(PackageField::Available(recipe)) =
                app.selected_package().map(|package| &package.recipe)
            else {
                app.notification =
                    Some("The selected package has no authoritative recipe identity.".into());
                return None;
            };
            let Some(index) = app
                .workspace
                .recipes
                .iter()
                .position(|candidate| candidate.name == *recipe)
            else {
                app.notification = Some(format!(
                    "Recipe {recipe} is not present in the current workspace inventory."
                ));
                return None;
            };
            app.recipe_selection = index;
            app.screen = Screen::Recipes;
            app.focus = FocusTarget::Workspace;
        }
        Action::OpenSelectedPackageProvider => {
            let Some(PackageField::Available(provider)) =
                app.selected_package().map(|package| &package.provider)
            else {
                app.notification =
                    Some("The selected package has no authoritative provider path.".into());
                return None;
            };
            if !provider.is_absolute() {
                app.notification =
                    Some("The selected package provider path is not absolute.".into());
                return None;
            }
            return Some(Effect::OpenInEditor(provider.clone()));
        }
        Action::BeginSelectedRecipeMetadata => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                app.recipe_metadata_loading.insert(recipe.name.clone());
                app.recipe_metadata_errors.remove(&recipe.name);
                return Some(Effect::GetRecipeMetadata(recipe.name.clone()));
            }
            app.notification = Some("No recipe is selected for metadata inspection.".into());
        }
        Action::RecipeMetadataLoaded(metadata) => {
            let recipe = metadata.recipe.clone();
            app.recipe_metadata_loading.remove(&recipe);
            app.recipe_metadata_errors.remove(&recipe);
            if let Some(sources) = metadata.sources.as_ref() {
                app.recipe_sources.insert(recipe.clone(), sources.clone());
            } else {
                app.recipe_sources.remove(&recipe);
            }
            app.recipe_metadata.insert(recipe, metadata);
        }
        Action::RecipeMetadataFailed { recipe, message } => {
            app.recipe_metadata_loading.remove(&recipe);
            app.recipe_metadata_errors
                .insert(recipe.clone(), message.clone());
            app.notification = Some(format!(
                "Recipe metadata for {recipe} is unavailable: {message}"
            ));
        }
        Action::DependenciesLoaded(dependencies) => {
            app.screen = Screen::Dependencies;
            let root = DependencyNodeId::recipe(dependencies.recipe.clone());
            let edges = dependencies
                .build
                .iter()
                .map(|name| DependencyEdge {
                    from: root.clone(),
                    to: DependencyNodeId::recipe(name),
                    kind: DependencyEdgeKind::Build,
                })
                .chain(dependencies.runtime.iter().map(|name| DependencyEdge {
                    from: root.clone(),
                    to: DependencyNodeId::recipe(name),
                    kind: DependencyEdgeKind::Runtime,
                }))
                .collect::<Vec<_>>();
            let (graph, _) =
                DependencyGraph::normalize(root, Vec::new(), edges, usize::MAX, usize::MAX);
            set_dependency_graph(app, graph, None);
            app.dependencies = Some(dependencies);
            app.dependency_selection = 0;
        }
        Action::SelectDependency { delta } => {
            let count = app.dependencies.as_ref().map_or(0, |dependencies| {
                dependencies.build.len() + dependencies.runtime.len()
            });
            app.dependency_selection = if delta.is_negative() {
                app.dependency_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.dependency_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::OpenSelectedDependency => {
            let selected = app.dependencies.as_ref().and_then(|dependencies| {
                dependencies
                    .build
                    .iter()
                    .chain(dependencies.runtime.iter())
                    .nth(app.dependency_selection)
            });
            if let Some(name) = selected {
                if let Some(index) = app
                    .workspace
                    .recipes
                    .iter()
                    .position(|recipe| recipe.name == *name)
                {
                    app.recipe_selection = index;
                    app.screen = Screen::Recipes;
                } else {
                    app.notification = Some(format!(
                        "{name} is a dependency but is not an available recipe in this workspace."
                    ));
                }
            }
        }
        Action::ConfirmRecipeTask => {
            if let Some(Dialog::RecipeTaskConfirmation(request)) = app.active_dialog().cloned() {
                close_dialog(app);
                prepare_build(app, request.targets.first().cloned());
                synchronize_focus(app);
                return Some(Effect::Start(request));
            }
        }
        Action::CancelRecipeTask => {
            if matches!(app.active_dialog(), Some(Dialog::RecipeTaskConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolModify => {
            if let Some(Dialog::DevtoolModifyConfirmation(identity)) = app.active_dialog().cloned()
            {
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolModify(identity));
            }
        }
        Action::CancelDevtoolModify => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolModifyConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolReset => {
            if let Some(Dialog::DevtoolResetConfirmation(plan)) = app.active_dialog().cloned() {
                let Some(status) = app.devtool_statuses.get(&plan.identity) else {
                    app.notification =
                        Some("Authoritative Devtool status expired; refresh with t.".into());
                    return None;
                };
                if let Some(reason) = status.disabled_reason(DevtoolAction::Reset) {
                    app.notification = Some(reason);
                    return None;
                }
                let current_source = match &status.workspace {
                    DevtoolWorkspace::Present { source_path, .. }
                    | DevtoolWorkspace::MissingDirectory { source_path } => source_path,
                    DevtoolWorkspace::NotMember => return None,
                };
                if current_source != &plan.source_path || !current_source.is_absolute() {
                    app.notification = Some(
                        "The authoritative Devtool reset source changed; refresh with t.".into(),
                    );
                    return None;
                }
                if let Err(error) = plan.operation().validate() {
                    app.notification = Some(error.to_string());
                    return None;
                }
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolReset(plan));
            }
        }
        Action::CancelDevtoolReset => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolResetConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolUpdateRecipe => {
            if let Some(Dialog::DevtoolUpdateConfirmation(identity)) = app.active_dialog().cloned()
            {
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolUpdateRecipe(identity));
            }
        }
        Action::CancelDevtoolUpdateRecipe => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolUpdateConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::SelectDevtoolFinishLayer { delta } => {
            if let Some(Dialog::DevtoolFinishPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.layers.len().saturating_sub(1))
                };
            }
        }
        Action::PreviewDevtoolFinish => {
            if let Some(Dialog::DevtoolFinishPicker(picker)) = app.active_dialog() {
                let Some(layer) = picker.layers.get(picker.selection).cloned() else {
                    app.notification = Some("Select a configured finish layer.".into());
                    return None;
                };
                if !layer.path.is_absolute()
                    || !app.workspace.layers.iter().any(|configured| {
                        configured.name == layer.name && configured.path == layer.path
                    })
                {
                    app.notification =
                        Some("The selected finish layer is no longer configured.".into());
                    return None;
                }
                replace_dialog(
                    app,
                    Dialog::DevtoolFinishConfirmation(DevtoolFinishPlan {
                        identity: picker.identity.clone(),
                        layer,
                    }),
                );
            }
        }
        Action::CancelDevtoolFinish => {
            if matches!(app.active_dialog(), Some(Dialog::DevtoolFinishPicker(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolFinish => {
            if let Some(Dialog::DevtoolFinishConfirmation(plan)) = app.active_dialog().cloned() {
                let Some(status) = app.devtool_statuses.get(&plan.identity) else {
                    app.notification =
                        Some("Authoritative Devtool status expired; refresh with t.".into());
                    return None;
                };
                if let Some(reason) = status.disabled_reason(DevtoolAction::Finish) {
                    app.notification = Some(reason);
                    return None;
                }
                if !plan.layer.path.is_absolute()
                    || !app.workspace.layers.iter().any(|configured| {
                        configured.name == plan.layer.name && configured.path == plan.layer.path
                    })
                {
                    app.notification =
                        Some("The selected finish layer is no longer configured.".into());
                    return None;
                }
                if let Err(error) = DevtoolOperation::from(plan.request()).validate() {
                    app.notification = Some(error.to_string());
                    return None;
                }
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolFinish(plan));
            }
        }
        Action::CancelDevtoolFinishConfirmation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolFinishConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::AppendDevtoolDeployTarget(character) => {
            if let Some(Dialog::DevtoolDeploy(draft)) = app.active_dialog_mut() {
                draft.target.push(character);
            }
        }
        Action::BackspaceDevtoolDeployTarget => {
            if let Some(Dialog::DevtoolDeploy(draft)) = app.active_dialog_mut() {
                draft.target.pop();
            }
        }
        Action::PreviewDevtoolDeploy => {
            if let Some(Dialog::DevtoolDeploy(draft)) = app.active_dialog() {
                let plan = DevtoolDeployPlan {
                    identity: draft.identity.clone(),
                    target: draft.target.clone(),
                };
                if let Err(error) = DevtoolOperation::from(plan.request()).validate() {
                    app.notification = Some(error.to_string());
                    return None;
                }
                replace_dialog(app, Dialog::DevtoolDeployConfirmation(plan));
            }
        }
        Action::CancelDevtoolDeploy => {
            if matches!(app.active_dialog(), Some(Dialog::DevtoolDeploy(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolDeploy => {
            if let Some(Dialog::DevtoolDeployConfirmation(plan)) = app.active_dialog().cloned() {
                let Some(status) = app.devtool_statuses.get(&plan.identity) else {
                    app.notification =
                        Some("Authoritative Devtool status expired; refresh with t.".into());
                    return None;
                };
                if let Some(reason) = status.disabled_reason(DevtoolAction::Deploy) {
                    app.notification = Some(reason);
                    return None;
                }
                if let Err(error) = DevtoolOperation::from(plan.request()).validate() {
                    app.notification = Some(error.to_string());
                    return None;
                }
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolDeploy(plan));
            }
        }
        Action::CancelDevtoolDeployConfirmation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolDeployConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::AppendDevtoolUndeployTarget(character) => {
            if let Some(Dialog::DevtoolUndeploy(draft)) = app.active_dialog_mut() {
                draft.target.push(character);
            }
        }
        Action::BackspaceDevtoolUndeployTarget => {
            if let Some(Dialog::DevtoolUndeploy(draft)) = app.active_dialog_mut() {
                draft.target.pop();
            }
        }
        Action::PreviewDevtoolUndeploy => {
            if let Some(Dialog::DevtoolUndeploy(draft)) = app.active_dialog() {
                let plan = DevtoolUndeployPlan {
                    identity: draft.identity.clone(),
                    target: draft.target.clone(),
                };
                if let Err(error) = plan.operation().validate() {
                    app.notification = Some(error.to_string());
                    return None;
                }
                replace_dialog(app, Dialog::DevtoolUndeployConfirmation(plan));
            }
        }
        Action::CancelDevtoolUndeploy => {
            if matches!(app.active_dialog(), Some(Dialog::DevtoolUndeploy(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolUndeploy => {
            if let Some(Dialog::DevtoolUndeployConfirmation(plan)) = app.active_dialog().cloned() {
                let Some(status) = app.devtool_statuses.get(&plan.identity) else {
                    app.notification =
                        Some("Authoritative Devtool status expired; refresh with t.".into());
                    return None;
                };
                if let Some(reason) = status.disabled_reason(DevtoolAction::Undeploy) {
                    app.notification = Some(reason);
                    return None;
                }
                if let Err(error) = plan.operation().validate() {
                    app.notification = Some(error.to_string());
                    return None;
                }
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolUndeploy(plan));
            }
        }
        Action::CancelDevtoolUndeployConfirmation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolUndeployConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolUpgrade => {
            if let Some(Dialog::DevtoolUpgradeConfirmation(plan)) = app.active_dialog().cloned() {
                let Some(status) = app.devtool_statuses.get(&plan.identity) else {
                    app.notification =
                        Some("Authoritative Devtool status expired; refresh with t.".into());
                    return None;
                };
                if let Some(reason) = status.disabled_reason(DevtoolAction::Upgrade) {
                    app.notification = Some(reason);
                    return None;
                }
                if let Err(error) = plan.operation().validate() {
                    app.notification = Some(error.to_string());
                    return None;
                }
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolUpgrade(plan));
            }
        }
        Action::CancelDevtoolUpgrade => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolUpgradeConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
