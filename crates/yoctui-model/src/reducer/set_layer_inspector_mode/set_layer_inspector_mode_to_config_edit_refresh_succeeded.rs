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
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
