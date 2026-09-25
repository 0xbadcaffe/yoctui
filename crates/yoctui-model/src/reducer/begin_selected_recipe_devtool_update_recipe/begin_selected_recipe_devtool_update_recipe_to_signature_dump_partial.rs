use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::BeginSelectedRecipeDevtoolUpdateRecipe => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification = Some(
                    "Refresh authoritative Devtool status with t before update-recipe.".into(),
                );
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::UpdateRecipe) {
                app.notification = Some(reason);
                return None;
            }
            open_dialog(app, Dialog::DevtoolUpdateConfirmation(identity));
        }
        Action::BeginSelectedRecipeDevtoolFinish => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification =
                    Some("Refresh authoritative Devtool status with t before finish.".into());
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::Finish) {
                app.notification = Some(reason);
                return None;
            }
            let layers = app
                .workspace
                .layers
                .iter()
                .filter(|layer| layer.path.is_absolute())
                .cloned()
                .collect::<Vec<_>>();
            if layers.is_empty() {
                app.notification =
                    Some("No configured layer has an absolute finish destination.".into());
                return None;
            }
            let provider_layer = app
                .workspace
                .recipes
                .get(app.recipe_selection)
                .and_then(|recipe| recipe.layer.as_deref());
            let selection = provider_layer
                .and_then(|name| layers.iter().position(|layer| layer.name == name))
                .unwrap_or(0);
            open_dialog(
                app,
                Dialog::DevtoolFinishPicker(DevtoolFinishPicker {
                    identity,
                    layers,
                    selection,
                }),
            );
        }
        Action::BeginSelectedRecipeDevtoolDeploy => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification = Some(
                    "Refresh authoritative Devtool status with t before deploy-target.".into(),
                );
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::Deploy) {
                app.notification = Some(reason);
                return None;
            }
            open_dialog(
                app,
                Dialog::DevtoolDeploy(DevtoolDeployDraft {
                    identity,
                    target: String::new(),
                }),
            );
        }
        Action::BeginSelectedRecipeDevtoolUndeploy => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification = Some(
                    "Refresh authoritative Devtool status with t before undeploy-target.".into(),
                );
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::Undeploy) {
                app.notification = Some(reason);
                return None;
            }
            open_dialog(
                app,
                Dialog::DevtoolUndeploy(DevtoolUndeployDraft {
                    identity,
                    target: String::new(),
                }),
            );
        }
        Action::BeginSelectedRecipeDevtoolUpgrade => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification =
                    Some("Refresh authoritative Devtool status with t before upgrade.".into());
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::Upgrade) {
                app.notification = Some(reason);
                return None;
            }
            open_dialog(
                app,
                Dialog::DevtoolUpgradeConfirmation(DevtoolUpgradePlan { identity }),
            );
        }
        Action::BeginSelectedRecipeDependencies => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                return update(
                    app,
                    Action::BeginDependencyGraph {
                        root: DependencyNodeId::recipe(recipe.name.clone()),
                    },
                );
            }
            app.notification = Some("No recipe is selected for dependency inspection.".into());
        }
        Action::BeginDependencyGraph { root } => {
            app.dependency_graph = DependencyGraphState::Loading { root: root.clone() };
            app.dependency_graph_selection = Some(root.clone());
            app.dependency_graph_anchor = None;
            return Some(Effect::GetDependencies(root.recipe_name().to_owned()));
        }
        Action::DependencyGraphLoaded(graph) => {
            set_dependency_graph(app, graph, None);
        }
        Action::DependencyGraphPartial { graph, limitations } => {
            set_dependency_graph(app, graph, Some(limitations));
        }
        Action::DependencyGraphFailed { root, message } => {
            app.dependency_graph = DependencyGraphState::Failed {
                root: root.clone(),
                message: message.clone(),
            };
            app.dependency_graph_selection = Some(root);
            app.notification = Some(format!("Dependency graph is unavailable: {message}"));
        }
        Action::SelectDependencyGraphNode { delta } => {
            let graph = app.dependency_graph.graph()?;
            let anchor = app.dependency_graph_anchor.as_ref().unwrap_or(&graph.root);
            let projection = graph.project(
                anchor,
                app.dependency_graph_reverse,
                &app.dependency_graph_query,
                &app.dependency_graph_collapsed,
                64,
                8_192,
            );
            if projection.rows.is_empty() {
                app.dependency_graph_selection = None;
                return None;
            }
            let current = app
                .dependency_graph_selection
                .as_ref()
                .and_then(|selected| projection.rows.iter().position(|row| &row.id == selected))
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(projection.rows.len().saturating_sub(1))
            };
            app.dependency_graph_selection = Some(projection.rows[next].id.clone());
        }
        Action::SelectDependencyGraphNodeAt { identity } => {
            let graph = app.dependency_graph.graph()?;
            if graph.contains(&identity) {
                app.dependency_graph_selection = Some(identity);
            }
        }
        Action::ToggleDependencyGraphReverse => {
            app.dependency_graph_reverse = !app.dependency_graph_reverse;
            app.dependency_graph_anchor = app
                .dependency_graph_reverse
                .then(|| app.dependency_graph_selection.clone())
                .flatten();
        }
        Action::CollapseSelectedDependencyGraphNode => {
            if let Some(selected) = app.dependency_graph_selection.clone() {
                app.dependency_graph_collapsed.insert(selected);
            }
        }
        Action::ExpandSelectedDependencyGraphNode => {
            if let Some(selected) = &app.dependency_graph_selection {
                app.dependency_graph_collapsed.remove(selected);
            }
        }
        Action::ToggleSelectedDependencyGraphNode => {
            if let Some(selected) = app.dependency_graph_selection.clone()
                && !app.dependency_graph_collapsed.remove(&selected)
            {
                app.dependency_graph_collapsed.insert(selected);
            }
        }
        Action::BeginDependencyGraphSearch => {
            app.dependency_graph_searching = true;
        }
        Action::AppendDependencyGraphQuery(character) => {
            if app.dependency_graph_searching
                && !character.is_control()
                && app.dependency_graph_query.len() + character.len_utf8()
                    <= DEPENDENCY_GRAPH_MAX_QUERY_BYTES
            {
                app.dependency_graph_query.push(character);
            }
        }
        Action::BackspaceDependencyGraphQuery => {
            if app.dependency_graph_searching {
                app.dependency_graph_query.pop();
            }
        }
        Action::ClearDependencyGraphQuery => {
            app.dependency_graph_query.clear();
        }
        Action::FinishDependencyGraphSearch => {
            app.dependency_graph_searching = false;
        }
        Action::RefreshDependencyGraph => {
            let Some(root) = app.dependency_graph.root().cloned() else {
                return update(
                    app,
                    Action::OpenRecipePicker(RecipePickerPurpose::Dependencies),
                );
            };
            return update(app, Action::BeginDependencyGraph { root });
        }
        Action::OpenSelectedDependencyRecipe => {
            let identity = match &app.dependency_graph {
                DependencyGraphState::Available(graph)
                | DependencyGraphState::Partial { graph, .. } => app
                    .dependency_graph_selection
                    .as_ref()
                    .filter(|selected| graph.contains(selected))
                    .cloned(),
                DependencyGraphState::AvailableEmpty { root } => Some(root.clone()),
                DependencyGraphState::NotLoaded
                | DependencyGraphState::Loading { .. }
                | DependencyGraphState::Failed { .. } => None,
            };
            let Some(identity) = identity else {
                app.notification =
                    Some("No current dependency graph node is available to open.".into());
                return None;
            };
            let recipe = identity.recipe_name();
            if let Some(index) = app
                .workspace
                .recipes
                .iter()
                .position(|candidate| candidate.name == recipe)
            {
                app.recipe_selection = index;
                app.screen = Screen::Recipes;
            } else {
                app.notification = Some(format!(
                    "{recipe} is in the dependency graph but not in the authoritative recipe inventory."
                ));
            }
        }
        Action::OpenSelectedDependencyProvider => {
            let selected = app.dependency_graph_selection.as_ref();
            let provider = app.dependency_graph.graph().and_then(|graph| {
                graph
                    .nodes
                    .iter()
                    .find(|node| Some(&node.id) == selected)
                    .and_then(|node| node.provider.clone())
                    .filter(|path| path.is_absolute())
            });
            if let Some(provider) = provider {
                return Some(Effect::OpenInEditor(provider));
            }
            app.notification =
                Some("The selected dependency node has no authoritative provider path.".into());
        }
        Action::OpenSelectedDependencyTaskLog => {
            let selected = app.dependency_graph_selection.as_ref();
            if !matches!(selected, Some(DependencyNodeId::Task { .. })) {
                app.notification =
                    Some("Task logs are available only for typed task dependency nodes.".into());
                return None;
            }
            let log = app.dependency_graph.graph().and_then(|graph| {
                graph
                    .nodes
                    .iter()
                    .find(|node| Some(&node.id) == selected)
                    .and_then(|node| node.log.clone())
                    .filter(|path| path.is_absolute())
            });
            if let Some(log) = log {
                return Some(Effect::OpenInEditor(log));
            }
            app.notification =
                Some("The selected task dependency has no authoritative log path.".into());
        }
        Action::BeginSignatureDump(target) => {
            return begin_signature_dump(app, target);
        }
        Action::RefreshSignatureDump => {
            let Some(target) = app.signature_dump.target().cloned() else {
                app.notification = Some("No signature target is available to refresh.".into());
                return None;
            };
            if signature_operation_is_loading(app) {
                app.notification = Some("A signature operation is already running.".into());
                return None;
            }
            return begin_signature_dump(app, target);
        }
        Action::LeaveSignatureWorkspace => {
            if signature_operation_is_loading(app) {
                return Some(Effect::CancelSignatureOperation);
            }
            if let Some(identity) = app.signature_recipe.as_ref()
                && let Some(index) = app.workspace.recipes.iter().position(|recipe| {
                    recipe.name == identity.name && recipe.file.as_ref() == Some(&identity.file)
                })
            {
                app.recipe_selection = index;
            }
            app.screen = Screen::Recipes;
            app.focus = FocusTarget::Workspace;
        }
        Action::OpenSignatureProvider => {
            let Some(identity) = app.signature_recipe.as_ref() else {
                app.notification =
                    Some("No signature recipe provider is available to open.".into());
                return None;
            };
            if !identity.file.is_absolute() {
                app.notification =
                    Some("The signature recipe provider path is not absolute.".into());
                return None;
            }
            return Some(Effect::OpenInEditor(identity.file.clone()));
        }
        Action::SignatureDumpLoaded { target, records } => {
            if !matches!(
                &app.signature_dump,
                SignatureDumpState::Loading { target: requested } if requested == &target
            ) {
                return None;
            }
            set_signature_dump(app, target, records, None);
        }
        Action::SignatureDumpPartial {
            target,
            records,
            limitations,
        } => {
            if !matches!(
                &app.signature_dump,
                SignatureDumpState::Loading { target: requested } if requested == &target
            ) {
                return None;
            }
            set_signature_dump(app, target, records, Some(limitations));
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
