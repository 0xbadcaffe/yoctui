//! State transitions beginning with OpenSelectedPackageRecipe.
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
        Action::OpenRecipeEditor {
            recipe,
            root,
            files,
        } => {
            let language = files.first().map_or(SourceLanguage::PlainText, |path| {
                SourceLanguage::from_path(path)
            });
            open_dialog(
                app,
                Dialog::RecipeEditor(RecipeEditor {
                    recipe,
                    root,
                    files,
                    selection: 0,
                    focus: RecipeEditorFocus::Files,
                    language,
                    document: TextAreaState::new(String::new()),
                    searching: false,
                }),
            );
            if let Some(path) = app.active_dialog().and_then(|dialog| match dialog {
                Dialog::RecipeEditor(editor) => editor.selected_path(),
                _ => None,
            }) {
                synchronize_focus(app);
                return Some(Effect::LoadRecipeEditorFile(path));
            }
            app.notification = Some("The Devtool workspace contains no editable files.".into());
        }
        Action::SelectRecipeEditorFile { delta } => {
            let path = if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                if editor.is_dirty() {
                    app.notification =
                        Some("Save changes with Ctrl+S before selecting another file.".into());
                    None
                } else {
                    editor.selection = if delta.is_negative() {
                        editor.selection.saturating_sub(delta.unsigned_abs())
                    } else {
                        editor
                            .selection
                            .saturating_add(delta as usize)
                            .min(editor.files.len().saturating_sub(1))
                    };
                    editor.selected_path()
                }
            } else {
                None
            };
            if let Some(path) = path {
                return Some(Effect::LoadRecipeEditorFile(path));
            }
        }
        Action::LoadRecipeEditorContent(content) => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                editor.document.accept_external_text(content);
                editor.document.set_mode(TextAreaMode::Normal);
                editor.searching = false;
                editor.refresh_language_and_validation();
            }
        }
        Action::LoadRecipeEditorExternalContent(content) => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                editor.document.accept_external_edit(content);
                editor.refresh_language_and_validation();
                app.notification = Some(
                    "External editor returned; review the diff, then Ctrl+B builds this recipe."
                        .into(),
                );
            }
        }
        Action::FocusRecipeEditor(focus) => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                editor.focus = focus;
                if focus == RecipeEditorFocus::Files {
                    editor.document.set_mode(TextAreaMode::Normal);
                    editor.searching = false;
                }
            }
        }
        Action::EditRecipeEditor(command) => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                match apply_recipe_editor_command(&mut editor.document, command) {
                    Ok(Some(content)) => return Some(Effect::CopyToClipboard(content)),
                    Ok(None) => editor.refresh_language_and_validation(),
                    Err(message) => app.notification = Some(message),
                }
            }
        }
        Action::BeginRecipeEditorSearch => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                editor.focus = RecipeEditorFocus::Document;
                editor.document.set_mode(TextAreaMode::Normal);
                editor.searching = true;
            }
        }
        Action::AppendRecipeEditorSearch(character) => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut()
                && editor.searching
                && !character.is_control()
            {
                let mut query = editor.document.search_state().query.clone();
                if query.len() + character.len_utf8() <= 4_096 {
                    query.push(character);
                    let _ = editor.document.search(query, false);
                }
            }
        }
        Action::BackspaceRecipeEditorSearch => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut()
                && editor.searching
            {
                let mut query = editor.document.search_state().query.clone();
                query.pop();
                let _ = editor.document.search(query, false);
            }
        }
        Action::FinishRecipeEditorSearch => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                editor.searching = false;
            }
        }
        Action::NextRecipeEditorMatch { backwards } => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                editor.document.next_match(backwards);
            }
        }
        Action::ToggleRecipeEditorEditing => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                editor.focus = RecipeEditorFocus::Document;
                editor.document.toggle_insert();
            }
        }
        Action::OpenRecipeEditorExternal => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog()
                && let Some(path) = editor.selected_path()
            {
                return Some(Effect::OpenInEditor(path));
            }
            app.notification = Some("No recipe file is selected for the external editor.".into());
        }
        Action::AppendRecipeEditor(character) => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut()
                && editor.document.editing
                && !character.is_control()
            {
                editor.document.insert(&character.to_string());
                editor.refresh_language_and_validation();
            }
        }
        Action::BackspaceRecipeEditor => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut()
                && editor.document.editing
            {
                editor.document.backspace();
                editor.refresh_language_and_validation();
            }
        }
        Action::SaveRecipeEditor => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog()
                && editor.is_dirty()
                && let Some(path) = editor.selected_path()
            {
                return Some(Effect::SaveRecipeEditorFile {
                    root: editor.root.clone(),
                    path,
                    content: editor.document.text.clone(),
                    expected: editor.document.base_revision(),
                });
            }
        }
        Action::RecipeEditorSaved => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                let content = editor.document.text.clone();
                editor.document.accept_external_text(content);
                editor.document.set_mode(TextAreaMode::Normal);
                editor.refresh_language_and_validation();
                app.notification = Some("Recipe file saved. Press Esc to return to Yoctui.".into());
            }
        }
        Action::BeginRecipeEditorBuild => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog().cloned() {
                if editor.is_dirty() {
                    app.notification =
                        Some("Save workspace changes before starting the recipe build.".into());
                } else {
                    let recipe = editor.recipe;
                    close_dialog(app);
                    begin_recipe_task_for(app, &recipe, None, false);
                }
            }
        }
        Action::CloseRecipeEditor => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() {
                if editor.is_dirty() {
                    app.notification = Some(
                        "Save changes with Ctrl+S before closing the workspace editor.".into(),
                    );
                } else {
                    close_dialog(app);
                }
            }
        }
        Action::SelectLayer { delta } => {
            app.layer_selection =
                shifted_index(app.layer_selection, delta, app.workspace.layers.len());
        }
        Action::OpenSelectedLayer => {
            if let Some(layer) = app.workspace.layers.get(app.layer_selection) {
                return Some(Effect::OpenInEditor(layer.path.clone()));
            }
            app.notification = Some("No layer is selected to open.".into());
        }
        Action::BeginSelectedLayerWorkspaceEditor => {
            if let Some(layer) = app.workspace.layers.get(app.layer_selection) {
                return Some(Effect::OpenWorkspaceEditor {
                    label: format!("Layer: {}", layer.name),
                    root: layer.path.clone(),
                });
            }
            app.notification = Some("No layer is selected to edit.".into());
        }
        Action::BeginSelectedLayerBrowser => {
            if let Some(layer) = app.workspace.layers.get(app.layer_selection) {
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: layer.name.clone(),
                    root: layer.path.clone(),
                    directory: layer.path.clone(),
                });
            }
            app.notification = Some("No layer is selected to browse.".into());
        }
        Action::LoadLayerBrowserDirectory {
            layer,
            root,
            directory,
            mut entries,
        } => {
            entries.sort_by_key(|entry| {
                (
                    !entry.is_dir,
                    entry.path.file_name().map(|name| name.to_owned()),
                )
            });
            let preferred = app
                .layer_browser
                .as_ref()
                .and_then(LayerBrowser::selected_entry)
                .map(|entry| entry.path.clone());
            if let Some(browser) = app
                .layer_browser
                .as_mut()
                .filter(|browser| browser.layer == layer && browser.root == root)
            {
                browser.directory = directory.clone();
                browser.nodes.insert(directory.clone(), entries);
                browser.expanded.insert(directory);
                browser.rebuild(preferred.as_ref());
            } else {
                let mut browser = LayerBrowser::new(layer, root);
                browser.directory = directory.clone();
                browser.nodes.insert(directory, entries);
                browser.rebuild(None);
                app.layer_browser = Some(browser);
            }
            if let Some(path) = app
                .layer_browser
                .as_ref()
                .and_then(LayerBrowser::selected_entry)
                .filter(|entry| !entry.is_dir)
                .map(|entry| entry.path.clone())
            {
                return Some(Effect::LoadLayerBrowserPreview(path));
            }
        }
        Action::SelectLayerBrowserEntry { delta } => {
            let query = app.metadata_query.to_ascii_lowercase();
            let path = if let Some(browser) = app.layer_browser.as_mut() {
                let matches = browser
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| {
                        query.is_empty()
                            || entry
                                .path
                                .to_string_lossy()
                                .to_ascii_lowercase()
                                .contains(&query)
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                let position = matches
                    .iter()
                    .position(|index| *index == browser.selection)
                    .unwrap_or(0);
                let position = shifted_index(position, delta, matches.len());
                browser.selection = matches.get(position).copied().unwrap_or(0);
                browser.preview_scroll = 0;
                if browser.selected_entry().is_some_and(|entry| entry.is_dir) {
                    browser.preview.clear();
                    browser.preview_kind = PreviewKind::Unavailable;
                    browser.preview_truncated = false;
                }
                browser
                    .selected_entry()
                    .filter(|entry| !entry.is_dir)
                    .map(|entry| entry.path.clone())
            } else {
                None
            };
            if let Some(path) = path {
                return Some(Effect::LoadLayerBrowserPreview(path));
            }
        }
        Action::LayerBrowserExpand => {
            let selected = app
                .layer_browser
                .as_ref()
                .and_then(LayerBrowser::selected_entry)
                .cloned();
            let selected_is_file = selected.as_ref().is_some_and(|entry| !entry.is_dir);
            if let Some(entry) = selected.filter(|entry| entry.is_dir) {
                let browser = app.layer_browser.as_mut().expect("browser was selected");
                if browser.expanded.contains(&entry.path) {
                    return None;
                }
                if browser.nodes.contains_key(&entry.path) {
                    browser.expanded.insert(entry.path.clone());
                    browser.rebuild(Some(&entry.path));
                    return None;
                }
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    directory: entry.path,
                });
            } else if selected_is_file && let Some(browser) = app.layer_browser.as_mut() {
                browser.preview_focused = true;
            }
        }
        Action::LayerBrowserEnter => {
            let selected = app
                .layer_browser
                .as_ref()
                .and_then(LayerBrowser::selected_entry)
                .cloned();
            if let Some(entry) = selected.filter(|entry| entry.is_dir) {
                let browser = app.layer_browser.as_mut().expect("browser was selected");
                if browser.expanded.remove(&entry.path) {
                    browser.rebuild(Some(&entry.path));
                    return None;
                }
                if browser.nodes.contains_key(&entry.path) {
                    browser.expanded.insert(entry.path.clone());
                    browser.rebuild(Some(&entry.path));
                    return None;
                }
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    directory: entry.path,
                });
            }
            return update(app, Action::EditSelectedLayerBrowserFile);
        }
        Action::LayerBrowserUp => {
            if let Some(browser) = app.layer_browser.as_mut()
                && let Some(entry) = browser.selected_entry().cloned()
            {
                if entry.is_dir && browser.expanded.remove(&entry.path) {
                    browser.rebuild(Some(&entry.path));
                } else if let Some(parent) = entry.path.parent()
                    && parent != browser.root
                    && let Some(index) = browser
                        .entries
                        .iter()
                        .position(|candidate| candidate.path == parent)
                {
                    browser.selection = index;
                }
            }
        }
        Action::CloseLayerBrowser => app.layer_browser = None,
        Action::RefreshLayerBrowser => {
            if let Some(browser) = app.layer_browser.as_ref() {
                let directory = browser
                    .selected_entry()
                    .map(|entry| {
                        if entry.is_dir {
                            entry.path.clone()
                        } else {
                            entry.path.parent().unwrap_or(&browser.root).to_path_buf()
                        }
                    })
                    .unwrap_or_else(|| browser.root.clone());
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    directory,
                });
            }
        }
        Action::ToggleLayerBrowserHidden => {
            if let Some(browser) = app.layer_browser.as_mut() {
                let selected = browser.selected_entry().map(|entry| entry.path.clone());
                browser.show_hidden = !browser.show_hidden;
                browser.rebuild(selected.as_ref());
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
