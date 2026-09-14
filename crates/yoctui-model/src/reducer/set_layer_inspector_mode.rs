//! State transitions beginning with SetLayerInspectorMode.
use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::SetLayerInspectorMode(mode) => {
            if let Some(browser) = app.layer_browser.as_mut() {
                browser.inspector_mode = if browser.inspector_mode == mode {
                    LayerInspectorMode::Preview
                } else {
                    mode
                };
            }
        }
        Action::ScrollLayerBrowserPreview { delta } => {
            if let Some(browser) = app.layer_browser.as_mut() {
                let maximum = browser.preview.lines().count().saturating_sub(1);
                browser.preview_scroll = if delta.is_negative() {
                    browser.preview_scroll.saturating_sub(delta.unsigned_abs())
                } else {
                    browser
                        .preview_scroll
                        .saturating_add(delta as usize)
                        .min(maximum)
                };
            }
        }
        Action::FocusLayerBrowserTree => {
            if let Some(browser) = app.layer_browser.as_mut() {
                browser.preview_focused = false;
            }
        }
        Action::LoadLayerBrowserPreview {
            path,
            content,
            kind,
            truncated,
        } => {
            if let Some(browser) = app.layer_browser.as_mut()
                && browser
                    .selected_entry()
                    .is_some_and(|entry| entry.path == path && !entry.is_dir)
            {
                browser.preview = content;
                browser.preview_kind = kind;
                browser.preview_truncated = truncated;
            }
        }
        Action::EditSelectedLayerBrowserFile => {
            if let Some(browser) = app.layer_browser.as_ref()
                && let Some(entry) = browser.selected_entry()
                && !entry.is_dir
                && let Ok(file) = entry.path.strip_prefix(&browser.root)
            {
                return Some(Effect::OpenLayerBrowserEditor {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    file: file.to_path_buf(),
                });
            }
            app.notification = Some("Select a file to edit.".into());
        }
        Action::BeginLayerRelationships => return Some(Effect::GetLayerRelationships),
        Action::LayerRelationshipsLoaded(relationships) => {
            app.layer_relationships = Some(relationships);
            app.screen = Screen::LayerRelationships;
        }
        Action::SelectConfigVariable { delta } => {
            let count = filtered_config_identities(app).len();
            app.config_selection = if delta.is_negative() {
                app.config_selection.saturating_sub(delta.unsigned_abs())
            } else {
                app.config_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::BeginSelectedConfigDetail => {
            let Some(identity) = selected_config_identity(app) else {
                app.notification = Some("No configuration variable is selected to inspect.".into());
                return None;
            };
            app.variable_detail_loading.insert(identity.clone());
            app.variable_detail_errors.remove(&identity);
            return Some(Effect::GetVariable(identity));
        }
        Action::CopySelectedConfigEffective => {
            match selected_config_copy_value(app, ConfigCopyValue::Effective) {
                Ok(value) => return Some(Effect::CopyToClipboard(value.to_owned())),
                Err(reason) => app.notification = Some(reason),
            }
        }
        Action::CopySelectedConfigUnexpanded => {
            match selected_config_copy_value(app, ConfigCopyValue::Unexpanded) {
                Ok(value) => return Some(Effect::CopyToClipboard(value.to_owned())),
                Err(reason) => app.notification = Some(reason),
            }
        }
        Action::VariableDetailFailed { identity, message } => {
            app.variable_detail_loading.remove(&identity);
            app.variable_detail_errors
                .insert(identity.clone(), message.clone());
            app.notification = Some(format!(
                "Configuration detail for {} is unavailable: {message}",
                identity.name
            ));
        }
        Action::OpenSelectedConfigSource => match selected_config_sources(app) {
            Ok((_, sources)) if sources.len() == 1 => {
                match resolve_config_source(app, &sources[0].path) {
                    Ok(path) => return Some(Effect::OpenInEditor(path)),
                    Err(reason) => app.notification = Some(reason),
                }
            }
            Ok((identity, sources)) => {
                open_dialog(
                    app,
                    Dialog::ConfigSourcePicker(ConfigSourcePicker {
                        identity,
                        sources,
                        selection: 0,
                    }),
                );
            }
            Err(reason) => app.notification = Some(reason),
        },
        Action::SelectConfigSource { delta } => {
            if let Some(Dialog::ConfigSourcePicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.sources.len().saturating_sub(1))
                };
            }
        }
        Action::OpenSelectedConfigSourceChoice => {
            let path = app.active_dialog().and_then(|dialog| match dialog {
                Dialog::ConfigSourcePicker(picker) => picker
                    .sources
                    .get(picker.selection)
                    .map(|source| source.path.clone()),
                _ => None,
            });
            if let Some(path) = path {
                match resolve_config_source(app, &path) {
                    Ok(path) => {
                        close_dialog(app);
                        synchronize_focus(app);
                        return Some(Effect::OpenInEditor(path));
                    }
                    Err(reason) => app.notification = Some(reason),
                }
            } else {
                app.notification = Some("The selected configuration source is stale.".into());
            }
        }
        Action::CancelConfigSourcePicker => {
            if matches!(app.active_dialog(), Some(Dialog::ConfigSourcePicker(_))) {
                close_dialog(app);
            }
        }
        Action::OpenConfigScopePicker => {
            let Some(identity) = selected_config_identity(app) else {
                app.notification =
                    Some("No configuration variable is selected for scope inspection.".into());
                return None;
            };
            let mut recipes = app
                .workspace
                .recipes
                .iter()
                .map(|recipe| recipe.name.clone())
                .collect::<Vec<_>>();
            recipes.sort();
            recipes.dedup();
            let mut scopes = vec![None];
            scopes.extend(recipes.into_iter().map(Some));
            let selection = scopes
                .iter()
                .position(|scope| scope == &app.config_scope)
                .unwrap_or(0);
            open_dialog(
                app,
                Dialog::ConfigScopePicker(ConfigScopePicker {
                    variable: identity.name,
                    scopes,
                    selection,
                }),
            );
        }
        Action::SelectConfigScope { delta } => {
            if let Some(Dialog::ConfigScopePicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.scopes.len().saturating_sub(1))
                };
            }
        }
        Action::ConfirmConfigScope => {
            let scope = if let Some(Dialog::ConfigScopePicker(picker)) = app.active_dialog() {
                picker.scopes.get(picker.selection).cloned()
            } else {
                None
            };
            let Some(scope) = scope else {
                app.notification = Some("The selected configuration scope is stale.".into());
                return None;
            };
            app.config_scope = scope;
            close_dialog(app);
            synchronize_focus(app);
            let Some(identity) = selected_config_identity(app) else {
                app.notification =
                    Some("No configuration variable is selected for scope inspection.".into());
                return None;
            };
            app.variable_detail_loading.insert(identity.clone());
            app.variable_detail_errors.remove(&identity);
            return Some(Effect::GetVariable(identity));
        }
        Action::CancelConfigScopePicker => {
            if matches!(app.active_dialog(), Some(Dialog::ConfigScopePicker(_))) {
                close_dialog(app);
            }
        }
        Action::OpenConfigComparison => match config_comparison(app) {
            Ok(comparison) => open_dialog(app, Dialog::ConfigComparison(comparison)),
            Err(reason) => app.notification = Some(reason),
        },
        Action::CloseConfigComparison => {
            if matches!(app.active_dialog(), Some(Dialog::ConfigComparison(_))) {
                close_dialog(app);
            }
        }
        Action::BeginConfigEdit => match config_edit_context(app) {
            Ok((identity, value, _)) => {
                let mut editor =
                    PopupEditor::new(popup_toml_document("value", &value, Some(&identity.name)));
                let _ = editor.select_toml_value("value");
                open_dialog(app, Dialog::ConfigEdit { identity, editor });
            }
            Err(reason) => app.notification = Some(reason),
        },
        Action::ToggleConfigEdit => {
            if let Some(Dialog::ConfigEdit { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendConfigEdit(character) => {
            if character.is_control() {
                app.notification =
                    Some("Configuration values cannot contain control characters.".into());
            } else if let Some(Dialog::ConfigEdit { editor, .. }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceConfigEdit => {
            if let Some(Dialog::ConfigEdit { editor, .. }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::PreviewConfigEdit => {
            let edit = app.active_dialog().and_then(|dialog| match dialog {
                Dialog::ConfigEdit {
                    identity, editor, ..
                } => Some((identity.clone(), popup_toml_value(&editor.text, "value"))),
                _ => None,
            });
            let (identity, value) = edit?;
            let value = match value {
                Ok(value) => value,
                Err(reason) => {
                    app.notification = Some(reason);
                    return None;
                }
            };
            match config_edit_assignment(&identity.name, &value) {
                Ok(assignment) => {
                    let Some(build_dir) = app.workspace.build_dir.as_ref() else {
                        app.notification =
                            Some("An active build directory is required for editing.".into());
                        return None;
                    };
                    replace_dialog(
                        app,
                        Dialog::ConfigEditConfirmation(ConfigEditRequest {
                            identity,
                            value,
                            destination: build_dir.join("conf/local.conf"),
                            assignment,
                        }),
                    );
                }
                Err(reason) => app.notification = Some(reason),
            }
        }
        Action::CancelConfigEdit => {
            if matches!(app.active_dialog(), Some(Dialog::ConfigEdit { .. })) {
                close_dialog(app);
            }
        }
        Action::ConfirmConfigEdit => {
            if let Some(Dialog::ConfigEditConfirmation(request)) = app.active_dialog().cloned() {
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::WriteConfigAssignment(request));
            }
        }
        Action::CancelConfigEditConfirmation => {
            if matches!(app.active_dialog(), Some(Dialog::ConfigEditConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfigEditWriteSucceeded { identity } => {
            if identity.recipe.is_some()
                || !EDITABLE_CONFIG_VARIABLES.contains(&identity.name.as_str())
            {
                app.notification =
                    Some("The completed configuration edit identity is invalid.".into());
                return None;
            }
            app.variable_detail_loading.insert(identity.clone());
            app.variable_detail_errors.remove(&identity);
            app.notification = Some(format!(
                "{} was saved; refreshing authoritative configuration detail.",
                identity.name
            ));
            return Some(Effect::GetVariable(identity));
        }
        Action::ConfigEditWriteFailed { identity, message } => {
            app.notification = Some(format!(
                "Could not save configuration variable {}: {message}",
                identity.name
            ));
        }
        Action::ConfigEditRefreshSucceeded { identity } => {
            app.notification = Some(format!("{} saved and refreshed.", identity.name));
        }
        Action::ConfigEditRefreshFailed { identity, message } => {
            app.variable_detail_loading.remove(&identity);
            app.notification = Some(format!(
                "{} was saved, but authoritative refresh failed: {message}",
                identity.name
            ));
        }
        Action::BeginBbmaskEdit => {
            let input = app
                .workspace
                .variables
                .get("BBMASK")
                .cloned()
                .unwrap_or_default();
            let mut editor = PopupEditor::new(popup_toml_document("bbmask", &input, None));
            let _ = editor.select_toml_value("bbmask");
            open_dialog(app, Dialog::BbmaskEdit(editor));
        }
        Action::ToggleBbmaskEdit => {
            if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendBbmask(character) => {
            if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceBbmask => {
            if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::PreviewBbmaskEdit => {
            if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog() {
                let input = match popup_toml_value(&editor.text, "bbmask") {
                    Ok(value) => value,
                    Err(reason) => {
                        app.notification = Some(reason);
                        return None;
                    }
                };
                if input.contains(['\n', '\r']) {
                    app.notification = Some("BBMASK must be entered on one line.".into());
                } else {
                    replace_dialog(app, Dialog::BbmaskConfirmation(input));
                }
            }
        }
        Action::CancelBbmaskEdit => {
            if matches!(app.active_dialog(), Some(Dialog::BbmaskEdit(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmBbmaskWrite => {
            if let Some(Dialog::BbmaskConfirmation(value)) = app.active_dialog().cloned() {
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::WriteBbmask(value));
            }
        }
        Action::CancelBbmaskWrite => {
            if matches!(app.active_dialog(), Some(Dialog::BbmaskConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::BeginMetadataSearch => app.metadata_searching = true,
        Action::ClearMetadataQuery => {
            app.metadata_query.clear();
            app.recipe_selection = 0;
            app.layer_selection = 0;
            app.config_selection = 0;
            select_first_matching_layer_entry(app);
            select_first_matching_recipe(app);
        }
        Action::FinishMetadataSearch => app.metadata_searching = false,
        Action::Notify(message) => app.notification = Some(message),
        Action::ActivateNotification => {
            if app.build.status == BuildStatus::Failed && app.logs.diagnostics().next().is_some() {
                app.screen = Screen::Errors;
                app.error_selection = app.logs.diagnostics().count().saturating_sub(1);
            }
            app.notification = None;
        }
        Action::DismissNotification => app.notification = None,
        Action::Quit => open_dialog(app, Dialog::QuitConfirmation),
        Action::ConfirmQuit => {
            if matches!(app.active_dialog(), Some(Dialog::QuitConfirmation)) {
                app.should_quit = true;
            }
        }
        Action::CancelQuit => {
            if matches!(app.active_dialog(), Some(Dialog::QuitConfirmation)) {
                close_dialog(app);
            }
        }
        Action::WorkspaceLoaded(w) => {
            let selected = selected_config_identity(app);
            app.workspace = w;
            app.available_images = if app.build_environment.connected() {
                app.workspace
                    .recipes
                    .iter()
                    .filter(|recipe| {
                        recipe.name.starts_with("core-image-") || recipe.name.ends_with("-image")
                    })
                    .map(|recipe| recipe.name.clone())
                    .collect()
            } else {
                Vec::new()
            };
            if app.config_scope.as_ref().is_some_and(|scope| {
                !app.workspace
                    .recipes
                    .iter()
                    .any(|recipe| &recipe.name == scope)
            }) {
                app.config_scope = None;
            }
            let names = app
                .workspace
                .variables
                .keys()
                .cloned()
                .collect::<HashSet<_>>();
            app.variable_details
                .retain(|identity, _| names.contains(&identity.name));
            app.variable_detail_loading
                .retain(|identity| names.contains(&identity.name));
            app.variable_detail_errors
                .retain(|identity, _| names.contains(&identity.name));
            let identities = filtered_config_identities(app);
            app.config_selection = selected
                .and_then(|selected| {
                    identities
                        .iter()
                        .position(|identity| identity.name == selected.name)
                })
                .unwrap_or_else(|| app.config_selection.min(identities.len().saturating_sub(1)));
        }
        Action::RecipesLoaded(mut recipes) => {
            let selected = app
                .workspace
                .recipes
                .get(app.recipe_selection)
                .map(|recipe| recipe.name.clone());
            recipes.sort_by(|left, right| left.name.cmp(&right.name));
            let names = recipes
                .iter()
                .map(|recipe| recipe.name.clone())
                .collect::<HashSet<_>>();
            app.workspace.recipes = recipes;
            app.available_images = if app.build_environment.connected() {
                app.workspace
                    .recipes
                    .iter()
                    .filter(|recipe| {
                        recipe.name.starts_with("core-image-") || recipe.name.ends_with("-image")
                    })
                    .map(|recipe| recipe.name.clone())
                    .collect()
            } else {
                Vec::new()
            };
            if app
                .config_scope
                .as_ref()
                .is_some_and(|scope| !names.contains(scope))
            {
                app.config_scope = None;
            }
            app.recipe_metadata
                .retain(|recipe, _| names.contains(recipe));
            app.recipe_sources
                .retain(|recipe, _| names.contains(recipe));
            app.recipe_metadata_loading
                .retain(|recipe| names.contains(recipe));
            app.recipe_metadata_errors
                .retain(|recipe, _| names.contains(recipe));
            app.recipe_selection = selected
                .and_then(|selected| {
                    app.workspace
                        .recipes
                        .iter()
                        .position(|recipe| recipe.name == selected)
                })
                .unwrap_or_else(|| {
                    app.recipe_selection
                        .min(app.workspace.recipes.len().saturating_sub(1))
                });
        }
        Action::LayersLoaded(mut layers) => {
            layers.sort_by(|left, right| left.name.cmp(&right.name));
            app.workspace.layers = layers;
            app.layer_selection = app
                .layer_selection
                .min(app.workspace.layers.len().saturating_sub(1));
        }
        Action::VariableLoaded(detail) => {
            app.variable_detail_loading.remove(&detail.identity);
            app.variable_detail_errors.remove(&detail.identity);
            if detail.identity.recipe.is_none() {
                if let Some(value) = detail.effective_value.clone() {
                    app.workspace
                        .variables
                        .insert(detail.identity.name.clone(), value);
                } else {
                    app.workspace.variables.remove(&detail.identity.name);
                }
                if let Some(provenance) = detail.provenance.clone() {
                    app.workspace
                        .variable_provenance
                        .insert(detail.identity.name.clone(), provenance);
                } else {
                    app.workspace
                        .variable_provenance
                        .remove(&detail.identity.name);
                }
                app.workspace.variable_provenance_chain.insert(
                    detail.identity.name.clone(),
                    detail
                        .operations
                        .iter()
                        .filter_map(|operation| {
                            operation.file.as_ref().map(|file| {
                                operation.line.map_or_else(
                                    || file.display().to_string(),
                                    |line| format!("{}:{line}", file.display()),
                                )
                            })
                        })
                        .collect(),
                );
            }
            app.variable_details.insert(detail.identity.clone(), detail);
        }
        Action::RecipeSourcesLoaded { recipe, paths } => {
            app.recipe_sources.insert(recipe, paths);
        }
        Action::HostTelemetryUpdated(telemetry) => {
            app.host_telemetry_history.record(&telemetry);
            app.host_telemetry = telemetry;
        }
        Action::Failure(e) => {
            let message = e.to_string();
            insert_system_log(app, Severity::Error, message.clone());
            app.notification = Some(message);
            app.build.status = BuildStatus::Failed;
            app.build.errors = app.build.errors.max(1);
        }
        Action::Tick if !app.reduced_motion => {
            app.animation_frame = app.animation_frame.wrapping_add(1)
        }
        Action::Tick => {}
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
