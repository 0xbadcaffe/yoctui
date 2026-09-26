use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::OpenRecipeEditor {
            recipe,
            root,
            mut files,
        } => {
            let file_inventory_truncated = files.len() > MAX_RECIPE_EDITOR_FILES;
            files.truncate(MAX_RECIPE_EDITOR_FILES);
            let language = files.first().map_or(SourceLanguage::PlainText, |path| {
                SourceLanguage::from_path(path)
            });
            open_dialog(
                app,
                Dialog::RecipeEditor(RecipeEditor {
                    recipe,
                    root,
                    files,
                    file_inventory_truncated,
                    selection: 0,
                    focus: RecipeEditorFocus::Files,
                    language,
                    document: TextAreaState::new(String::new()),
                    searching: false,
                    pending_search_position: None,
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
                if let Some((line, column)) = editor.pending_search_position.take() {
                    editor.document.select_position(line, column, false);
                }
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
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
