use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
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
