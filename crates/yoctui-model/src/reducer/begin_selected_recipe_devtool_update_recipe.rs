//! State transitions beginning with BeginSelectedRecipeDevtoolUpdateRecipe.
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
                app.notification = Some("No dependency graph root is available to refresh.".into());
                return None;
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
        Action::SignatureDumpFailed { target, message } => {
            if !matches!(
                &app.signature_dump,
                SignatureDumpState::Loading { target: requested } if requested == &target
            ) {
                return None;
            }
            app.signature_dump = SignatureDumpState::Failed {
                target,
                message: message.clone(),
            };
            app.notification = Some(format!("Signature dump is unavailable: {message}"));
        }
        Action::SelectSignatureRecord { delta } => {
            let records = app.signature_dump.records()?;
            if records.is_empty() {
                app.signature_selection = None;
                return None;
            }
            let current = app
                .signature_selection
                .as_ref()
                .and_then(|selected| {
                    records
                        .iter()
                        .position(|record| &record.identity == selected)
                })
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(records.len().saturating_sub(1))
            };
            app.signature_selection = Some(records[next].identity.clone());
        }
        Action::SetSelectedSignatureComparisonSide(side) => {
            let Some(selected) = app.signature_selection.clone() else {
                app.notification = Some("No signature record is selected.".into());
                return None;
            };
            if !app
                .signature_dump
                .records()
                .is_some_and(|records| records.iter().any(|record| record.identity == selected))
            {
                app.notification =
                    Some("The selected signature is not in the current dump result.".into());
                return None;
            }
            let (mut left, mut right) = signature_comparison_inputs(&app.signature_comparison);
            match side {
                SignatureComparisonSide::Left => left = Some(selected),
                SignatureComparisonSide::Right => right = Some(selected),
            }
            app.signature_comparison = SignatureComparisonState::Ready { left, right };
        }
        Action::BeginSignatureComparison => {
            let (Some(left), Some(right)) = signature_comparison_inputs(&app.signature_comparison)
            else {
                app.notification =
                    Some("Select both left and right signature records before comparing.".into());
                return None;
            };
            let request = SignatureComparisonRequest { left, right };
            if let Err(message) = request.validate() {
                app.notification = Some(message.into());
                return None;
            }
            if !app.signature_dump.records().is_some_and(|records| {
                records.iter().any(|record| record.identity == request.left)
                    && records
                        .iter()
                        .any(|record| record.identity == request.right)
            }) {
                app.notification = Some(
                    "Both signature comparison inputs must be in the current dump result.".into(),
                );
                return None;
            }
            app.signature_comparison = SignatureComparisonState::Loading {
                request: request.clone(),
            };
            return Some(Effect::CompareSignatures(request));
        }
        Action::SignatureComparisonLoaded {
            request,
            differences,
        } => {
            if !matches!(
                &app.signature_comparison,
                SignatureComparisonState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            let (differences, report) =
                normalize_signature_differences(differences, MAX_SIGNATURE_DIFFERENCES);
            app.signature_comparison = if report.is_partial() {
                SignatureComparisonState::Partial {
                    request,
                    differences,
                    limitations: vec![format!(
                        "Model bounds truncated {} signature differences.",
                        report.truncated_differences
                    )],
                }
            } else if differences.is_empty() {
                SignatureComparisonState::AvailableEmpty { request }
            } else {
                SignatureComparisonState::Available {
                    request,
                    differences,
                }
            };
        }
        Action::SignatureComparisonPartial {
            request,
            differences,
            mut limitations,
        } => {
            if !matches!(
                &app.signature_comparison,
                SignatureComparisonState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            let (differences, report) =
                normalize_signature_differences(differences, MAX_SIGNATURE_DIFFERENCES);
            if report.is_partial() {
                limitations.push(format!(
                    "Model bounds truncated {} signature differences.",
                    report.truncated_differences
                ));
            }
            app.signature_comparison = SignatureComparisonState::Partial {
                request,
                differences,
                limitations,
            };
        }
        Action::SignatureComparisonFailed { request, message } => {
            if !matches!(
                &app.signature_comparison,
                SignatureComparisonState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            app.signature_comparison = SignatureComparisonState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Signature comparison failed: {message}"));
        }
        Action::BeginPackageInventory => {
            if package_operation_is_loading(app) {
                app.notification = Some("A package-data operation is already running.".into());
                return None;
            }
            return Some(begin_package_inventory(app));
        }
        Action::RefreshPackageInventory => {
            if package_operation_is_loading(app) {
                app.notification = Some("A package-data operation is already running.".into());
                return None;
            }
            return Some(begin_package_inventory(app));
        }
        Action::CancelPackageOperation => {
            if package_operation_is_loading(app) {
                return Some(Effect::CancelPackageOperation);
            }
            app.notification = Some("No package-data operation is running.".into());
        }
        Action::PackageInventoryLoaded { request, packages } => {
            if !matches!(
                app.package_inventory,
                PackageInventoryState::Loading { request: pending } if pending == request
            ) {
                return None;
            }
            set_package_inventory(app, request, packages, None);
        }
        Action::PackageInventoryPartial {
            request,
            packages,
            limitations,
        } => {
            if !matches!(
                app.package_inventory,
                PackageInventoryState::Loading { request: pending } if pending == request
            ) {
                return None;
            }
            set_package_inventory(app, request, packages, Some(limitations));
        }
        Action::PackageInventoryFailed { request, message } => {
            if !matches!(
                app.package_inventory,
                PackageInventoryState::Loading { request: pending } if pending == request
            ) {
                return None;
            }
            app.package_inventory = PackageInventoryState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Package inventory is unavailable: {message}"));
        }
        Action::SelectPackage { delta } => {
            let visible = app
                .filtered_packages()
                .into_iter()
                .map(|package| package.identity.clone())
                .collect::<Vec<_>>();
            if visible.is_empty() {
                app.package_selection = None;
                return None;
            }
            let current = app
                .package_selection
                .as_ref()
                .and_then(|identity| visible.iter().position(|candidate| candidate == identity))
                .unwrap_or(0);
            let next = shifted_index(current, delta, visible.len());
            app.package_selection = Some(visible[next].clone());
            app.package_dependency_selection = 0;
        }
        Action::BeginPackageSearch => app.package_searching = true,
        Action::AppendPackageQuery(character) => {
            if !character.is_control() && app.package_query.len() < 256 {
                app.package_query.push(character);
                set_package_selection_to_current_or_first(app, app.package_selection.clone());
            }
        }
        Action::BackspacePackageQuery => {
            app.package_query.pop();
            set_package_selection_to_current_or_first(app, app.package_selection.clone());
        }
        Action::ClearPackageQuery => {
            app.package_query.clear();
            set_package_selection_to_current_or_first(app, app.package_selection.clone());
        }
        Action::FinishPackageSearch => app.package_searching = false,
        Action::BeginSelectedPackageDetail => {
            let Some(identity) = app
                .selected_package()
                .map(|package| package.identity.clone())
            else {
                app.notification = Some("No current package is selected for inspection.".into());
                return None;
            };
            if package_operation_is_loading(app) {
                app.notification = Some("A package-data operation is already running.".into());
                return None;
            }
            return Some(begin_package_detail(app, identity));
        }
        Action::PackageDetailLoaded { request, detail } => {
            if !app.package_details.get(&request.identity).is_some_and(
                |state| matches!(state, PackageDetailState::Loading { request: pending } if pending == &request),
            ) {
                return None;
            }
            let (detail, report) = normalize_package_detail(&request.identity, detail);
            let Some(detail) = detail else {
                app.package_details.insert(
                    request.identity.clone(),
                    PackageDetailState::Failed {
                        request,
                        message: "backend returned detail for a different or invalid package"
                            .into(),
                    },
                );
                return None;
            };
            let mut limitations = Vec::new();
            append_package_normalization_limitations(&mut limitations, &report);
            let limitations = normalize_package_limitations(limitations);
            let state = if !limitations.is_empty() {
                PackageDetailState::Partial {
                    request,
                    detail,
                    limitations,
                }
            } else if package_detail_is_empty(&detail) {
                PackageDetailState::AvailableEmpty { request }
            } else {
                PackageDetailState::Available { request, detail }
            };
            app.package_details
                .insert(state.request().unwrap().identity.clone(), state);
            app.package_dependency_selection = 0;
        }
        Action::PackageDetailPartial {
            request,
            detail,
            mut limitations,
        } => {
            if !app.package_details.get(&request.identity).is_some_and(
                |state| matches!(state, PackageDetailState::Loading { request: pending } if pending == &request),
            ) {
                return None;
            }
            let (detail, report) = normalize_package_detail(&request.identity, detail);
            let Some(detail) = detail else {
                app.package_details.insert(
                    request.identity.clone(),
                    PackageDetailState::Failed {
                        request,
                        message: "backend returned detail for a different or invalid package"
                            .into(),
                    },
                );
                return None;
            };
            append_package_normalization_limitations(&mut limitations, &report);
            let limitations = normalize_package_limitations(limitations);
            app.package_details.insert(
                request.identity.clone(),
                PackageDetailState::Partial {
                    request,
                    detail,
                    limitations,
                },
            );
            app.package_dependency_selection = 0;
        }
        Action::PackageDetailFailed { request, message } => {
            if !app.package_details.get(&request.identity).is_some_and(
                |state| matches!(state, PackageDetailState::Loading { request: pending } if pending == &request),
            ) {
                return None;
            }
            app.package_details.insert(
                request.identity.clone(),
                PackageDetailState::Failed {
                    request,
                    message: message.clone(),
                },
            );
            app.notification = Some(format!("Package detail is unavailable: {message}"));
        }
        Action::OpenPackageDependency { identity, reverse } => {
            let available = app
                .selected_package_detail()
                .and_then(PackageDetailState::detail)
                .and_then(|detail| {
                    if reverse {
                        detail.reverse_dependencies.available()
                    } else {
                        detail.runtime_dependencies.available()
                    }
                })
                .is_some_and(|dependencies| dependencies.contains(&identity));
            if !available {
                app.notification = Some(
                    "The requested package dependency is not in the current typed detail.".into(),
                );
                return None;
            }
            if app
                .package_inventory
                .packages()
                .is_some_and(|packages| packages.iter().any(|package| package.identity == identity))
            {
                return select_package_identity(app, identity, false);
            } else {
                app.notification =
                    Some("The dependency is not present in the current package inventory.".into());
            }
        }
        Action::TogglePackageDependencyKind => {
            app.package_dependency_reverse = !app.package_dependency_reverse;
            app.package_dependency_selection = 0;
        }
        Action::SelectPackageDependency { delta } => {
            let count = app
                .selected_package_dependencies()
                .map_or(0, <[PackageIdentity]>::len);
            app.package_dependency_selection = if delta.is_negative() {
                app.package_dependency_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.package_dependency_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::OpenSelectedPackageDependency => {
            let Some(identity) = app.selected_package_dependency().cloned() else {
                app.notification = Some(format!(
                    "No {} dependency is selected.",
                    if app.package_dependency_reverse {
                        "reverse"
                    } else {
                        "runtime"
                    }
                ));
                return None;
            };
            return select_package_identity(app, identity, true);
        }
        Action::BackPackageNavigation => {
            let Some(identity) = app.package_navigation.pop() else {
                app.notification = Some("Package navigation history is empty.".into());
                return None;
            };
            app.package_selection = Some(identity);
            app.package_dependency_selection = 0;
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
