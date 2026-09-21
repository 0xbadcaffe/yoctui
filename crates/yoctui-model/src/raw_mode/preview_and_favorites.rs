fn request_raw_preview(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>,
) {
    let request = {
        let Some(form) = state.form.as_mut() else {
            state.notification = Some("No Raw command form is open.".into());
            return;
        };
        let Some(command) = catalog.command(&form.command) else {
            state.close_unsafe_work("Raw form command is no longer in the catalog.".into());
            return;
        };
        let mut values = BTreeMap::new();
        let mut first_error = None;
        for definition in &command.parameters {
            let Some(field) = form.fields.get_mut(&definition.id) else {
                first_error.get_or_insert_with(|| {
                    format!("Raw form state has no parameter {}.", definition.id)
                });
                continue;
            };
            match definition.parse_value(&field.editor.text) {
                Ok(value) => {
                    field.value.clone_from(&value);
                    field.validation_error = None;
                    if let Some(value) = value {
                        values.insert(definition.id.clone(), value);
                    }
                }
                Err(error) => {
                    field.value = None;
                    field.validation_error = Some(error.clone());
                    first_error.get_or_insert_with(|| error.to_string());
                }
            }
        }
        let additional_arguments = match form.additional_arguments.validate() {
            Ok(arguments) => arguments.clone(),
            Err(error) => {
                first_error.get_or_insert_with(|| error.to_string());
                RawAdditionalArguments::default()
            }
        };
        if let Some(error) = first_error {
            state.notification = Some(error);
            return;
        }
        RawPreviewRequest {
            catalog_version: state.catalog_version,
            command: form.command.clone(),
            parameters: values,
            additional_arguments,
            capability_generation: form.capability_generation,
            build_directory: form.build_directory.clone(),
        }
    };
    match catalog.preview(&request, authority) {
        Ok(preview) => {
            state.preview = Some(preview);
            state.notification = None;
            state.enter_view(RawModeView::Preview, RawModeFocus::Preview);
        }
        Err(error) => state.notification = Some(error.to_string()),
    }
}

fn activate_raw_history(state: &mut RawModeState, catalog: &RawCatalog) {
    let command = state
        .history
        .get(state.history_selection)
        .map(|record| record.command.clone());
    let Some(command) = command.and_then(|command| {
        catalog
            .command(&command)
            .map(|catalog_command| (command, catalog_command.category.clone()))
    }) else {
        state.notification = Some("The retained Raw command is stale or unavailable.".into());
        return;
    };
    state.command = Some(command.0);
    state.category = Some(command.1);
    state.search.query.clear();
    state.search.editing = false;
    state.return_stack.clear();
    state.view = RawModeView::Browser;
    state.browser_column = RawBrowserColumn::Commands;
    state.focus = RawModeFocus::Commands;
}

fn activate_raw_favorite(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>,
) {
    let Some(favorite) = state.favorites.get(state.favorite_selection).cloned() else {
        state.notification = Some("No Raw favorite is selected.".into());
        return;
    };
    let projection = favorite.project(catalog, authority);
    if projection.stale {
        state.notification = projection.reason;
        return;
    }
    if !projection.availability.is_enabled() {
        state.notification = Some(raw_availability_reason(&projection.availability));
        return;
    }
    let Some(command) = catalog.command(&favorite.command) else {
        state.notification = Some("The Raw favorite command is stale.".into());
        return;
    };
    state.command = Some(command.id.clone());
    state.category = Some(command.category.clone());
    state.search.query.clear();
    state.search.editing = false;
    state.return_stack.clear();
    open_raw_form(
        state,
        command,
        authority,
        &favorite.parameter_defaults,
        &favorite.additional_arguments,
    );
}

fn rename_raw_favorite(state: &mut RawModeState, name: String) {
    let Some(current) = state.favorites.get(state.favorite_selection).cloned() else {
        state.notification = Some("No Raw favorite is selected.".into());
        return;
    };
    let replacement = RawFavorite { name, ..current };
    match update_raw_favorite(&mut state.favorites, replacement) {
        Ok(()) => state.notification = Some("Raw favorite renamed.".into()),
        Err(error) => state.notification = Some(error.to_string()),
    }
}

fn move_selected_raw_favorite(state: &mut RawModeState, delta: isize) {
    let Some(current) = state.favorites.get(state.favorite_selection).cloned() else {
        state.notification = Some("No Raw favorite is selected.".into());
        return;
    };
    match move_raw_favorite(&mut state.favorites, &current.command, delta) {
        Ok(()) => {
            state.favorite_selection = state
                .favorites
                .iter()
                .position(|favorite| favorite.command == current.command)
                .unwrap_or(0);
            state.notification = Some("Raw favorite order updated.".into());
        }
        Err(error) => state.notification = Some(error.to_string()),
    }
}

fn request_raw_favorite_removal(state: &mut RawModeState) {
    let Some(command) = state
        .favorites
        .get(state.favorite_selection)
        .map(|favorite| favorite.command.clone())
    else {
        state.notification = Some("No Raw favorite is selected.".into());
        return;
    };
    state.favorite_confirmation = Some(RawFavoriteConfirmation {
        command,
        return_focus: state.focus,
    });
    state.focus = RawModeFocus::FavoriteConfirmation;
    state.notification = Some("Confirm removal of the exact Raw favorite.".into());
}

fn inspect_raw_favorite(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>,
) {
    let Some(favorite) = state.favorites.get(state.favorite_selection) else {
        state.notification = Some("No Raw favorite is selected.".into());
        return;
    };
    let projection = favorite.project(catalog, authority);
    let reason = projection
        .reason
        .unwrap_or_else(|| raw_availability_reason(&projection.availability));
    state.notification = Some(format!("{}: {reason}", favorite.name));
}

fn toggle_raw_favorite(state: &mut RawModeState, catalog: &RawCatalog) {
    let Some(command) = state.command.clone() else {
        state.notification = Some("No Raw command is selected.".into());
        return;
    };
    if catalog.command(&command).is_none() {
        state.notification = Some("The selected Raw command is stale.".into());
        return;
    }
    if state
        .favorites
        .iter()
        .any(|favorite| favorite.command == command)
    {
        state.favorite_confirmation = Some(RawFavoriteConfirmation {
            command,
            return_focus: state.focus,
        });
        state.focus = RawModeFocus::FavoriteConfirmation;
        state.notification = Some("Confirm removal of the exact Raw favorite.".into());
    } else if state.favorites.len() == MAX_RAW_FAVORITES {
        state.notification = Some(format!(
            "Raw favorites are bounded to {MAX_RAW_FAVORITES} entries."
        ));
    } else {
        let catalog_command = catalog
            .command(&command)
            .expect("selected Raw command was checked above");
        let favorite = RawFavorite::new(
            catalog_command,
            catalog_command.label.clone(),
            BTreeMap::new(),
            RawAdditionalArguments::default(),
            state.favorites.len() as u16,
        )
        .expect("validated catalog command creates a bounded favorite");
        state.favorites.push(favorite);
        state.favorite_selection = state.favorites.len().saturating_sub(1);
        state.notification = Some("Raw favorite added.".into());
    }
}

fn confirm_raw_favorite(state: &mut RawModeState) {
    let Some(confirmation) = state.favorite_confirmation.take() else {
        return;
    };
    let _ = remove_raw_favorite(&mut state.favorites, &confirmation.command);
    state.favorite_selection = state
        .favorite_selection
        .min(state.favorites.len().saturating_sub(1));
    state.focus = confirmation.return_focus;
    state.notification = Some("Raw favorite removed.".into());
}

fn cancel_raw_favorite(state: &mut RawModeState) {
    let Some(confirmation) = state.favorite_confirmation.take() else {
        return;
    };
    state.focus = confirmation.return_focus;
    state.notification = None;
}

fn reproject_raw_authority(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>,
) {
    if state.view == RawModeView::Execution {
        return;
    }
    let Some(form) = state.form.as_ref() else {
        return;
    };
    let Some(command) = catalog.command(&form.command) else {
        state.close_unsafe_work("Raw form closed because its command was removed.".into());
        return;
    };
    let availability = command.availability(authority);
    if !availability.is_enabled() {
        state.close_unsafe_work(format!(
            "Raw form closed after capability update: {}",
            raw_availability_reason(&availability)
        ));
        return;
    }
    let Some(authority) = authority else {
        state.close_unsafe_work("Raw form closed because capability authority was lost.".into());
        return;
    };
    let Some(build_directory) = authority.snapshot.environment.build_directory.value() else {
        state.close_unsafe_work(
            "Raw form closed because build-directory authority was lost.".into(),
        );
        return;
    };
    if build_directory != &form.build_directory {
        state.close_unsafe_work(
            "Raw form closed because the authoritative build directory changed.".into(),
        );
        return;
    }
    let preview_stale = state.preview.as_ref().is_some_and(|preview| {
        preview.capability_generation != authority.snapshot.generation
            || preview.build_directory != *build_directory
    });
    if preview_stale && state.view == RawModeView::Preview {
        state.leave_view();
        state.notification = Some(
            "Raw preview closed after a safe capability generation update; review it again.".into(),
        );
    } else if preview_stale {
        state.preview = None;
    }
    if let Some(form) = state.form.as_mut() {
        form.capability_generation = authority.snapshot.generation;
        form.build_directory.clone_from(build_directory);
    }
}
