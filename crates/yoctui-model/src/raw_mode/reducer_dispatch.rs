pub fn reduce_raw_mode(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>,
    action: RawModeAction,
) {
    reconcile_raw_mode(state, catalog);
    match action {
        RawModeAction::SelectCategory { delta } => select_raw_category(state, catalog, delta),
        RawModeAction::SelectCommand { delta } => select_raw_command(state, catalog, delta),
        RawModeAction::FocusCategories => {
            state.browser_column = RawBrowserColumn::Categories;
            state.focus = RawModeFocus::Categories;
        }
        RawModeAction::FocusCommands => {
            state.browser_column = RawBrowserColumn::Commands;
            state.focus = RawModeFocus::Commands;
        }
        RawModeAction::OpenSelected => open_raw_selected(state, catalog, authority),
        RawModeAction::Back => raw_mode_back(state),
        RawModeAction::BeginSearch => {
            state.search.editing = true;
            state.focus = RawModeFocus::Search;
        }
        RawModeAction::AppendSearch(character) => {
            if state.search.editing
                && !character.is_control()
                && state.search.query.len() + character.len_utf8() <= MAX_RAW_SEARCH_BYTES
            {
                state.search.query.push(character);
                reconcile_raw_command(state, catalog);
            }
        }
        RawModeAction::BackspaceSearch => {
            if state.search.editing {
                state.search.query.pop();
                reconcile_raw_command(state, catalog);
            }
        }
        RawModeAction::FinishSearch => {
            state.search.editing = false;
            state.focus = match state.browser_column {
                RawBrowserColumn::Categories => RawModeFocus::Categories,
                RawBrowserColumn::Commands => RawModeFocus::Commands,
            };
        }
        RawModeAction::ClearSearch => {
            state.search.query.clear();
            reconcile_raw_command(state, catalog);
        }
        RawModeAction::SetParameterInput { parameter, input } => {
            set_raw_parameter_input(state, catalog, &parameter, input)
        }
        RawModeAction::ChooseParameter { parameter, value } => {
            choose_raw_parameter(state, catalog, &parameter, value)
        }
        RawModeAction::EditParameterInput { parameter, command } => {
            edit_raw_parameter_input(state, catalog, &parameter, command)
        }
        RawModeAction::SelectFormField { delta } => select_raw_form_field(state, delta),
        RawModeAction::EditAdditionalArguments(command) => {
            if let Some(form) = state.form.as_mut()
                && let Err(error) = form.additional_arguments.apply(command)
            {
                state.notification = Some(error.to_string());
            }
        }
        RawModeAction::RequestPreview => request_raw_preview(state, catalog, authority),
        RawModeAction::ConfirmPreview
        | RawModeAction::CancelExecution(_)
        | RawModeAction::SetExecutionAttachment { .. } => {}
        RawModeAction::CloseExecution => raw_mode_back(state),
        RawModeAction::ToggleOutputFollow => {
            state.output.follow = !state.output.follow;
            if state.output.follow {
                state.output.vertical_scroll = 0;
            }
        }
        RawModeAction::SelectOutputStream(stream) => {
            state.output.stream = stream;
            state.output.vertical_scroll = 0;
            state.output.horizontal_scroll = 0;
        }
        RawModeAction::ScrollOutput {
            vertical,
            horizontal,
        } => {
            let (maximum_vertical, maximum_horizontal) = raw_output_scroll_bounds(state);
            state.output.vertical_scroll = shifted_index(
                state.output.vertical_scroll,
                maximum_vertical.saturating_add(1),
                vertical,
            );
            state.output.horizontal_scroll = shifted_index(
                state.output.horizontal_scroll,
                maximum_horizontal.saturating_add(1),
                horizontal,
            );
            if vertical != 0 {
                state.output.follow = false;
            }
        }
        RawModeAction::BeginOutputSearch => state.output.searching = true,
        RawModeAction::AppendOutputSearch(character) => {
            if state.output.searching
                && !character.is_control()
                && state.output.query.len() + character.len_utf8() <= MAX_RAW_SEARCH_BYTES
            {
                state.output.query.push(character);
            }
        }
        RawModeAction::BackspaceOutputSearch => {
            if state.output.searching {
                state.output.query.pop();
            }
        }
        RawModeAction::FinishOutputSearch => state.output.searching = false,
        RawModeAction::ClearOutputSearch => state.output.query.clear(),
        RawModeAction::OpenExecution(command) => {
            if catalog.command(&command).is_some() {
                state.execution = Some(command);
                state.output = RawOutputViewState::default();
                state.enter_view(RawModeView::Execution, RawModeFocus::Execution);
            }
        }
        RawModeAction::OpenHistory => {
            state.history_selection = state
                .history_selection
                .min(state.history.len().saturating_sub(1));
            state.enter_view(RawModeView::History, RawModeFocus::History);
        }
        RawModeAction::SelectHistory { delta } => {
            state.history_selection =
                shifted_index(state.history_selection, state.history.len(), delta)
        }
        RawModeAction::ActivateHistory => activate_raw_history(state, catalog),
        RawModeAction::OpenFavorites => {
            state.favorite_selection = state
                .favorite_selection
                .min(state.favorites.len().saturating_sub(1));
            state.enter_view(RawModeView::Favorites, RawModeFocus::Favorites);
        }
        RawModeAction::SelectFavorite { delta } => {
            state.favorite_selection =
                shifted_index(state.favorite_selection, state.favorites.len(), delta)
        }
        RawModeAction::ActivateFavorite => activate_raw_favorite(state, catalog, authority),
        RawModeAction::ToggleFavorite => toggle_raw_favorite(state, catalog),
        RawModeAction::RenameFavorite { name } => rename_raw_favorite(state, name),
        RawModeAction::MoveFavorite { delta } => move_selected_raw_favorite(state, delta),
        RawModeAction::RemoveFavorite => request_raw_favorite_removal(state),
        RawModeAction::InspectFavorite => inspect_raw_favorite(state, catalog, authority),
        RawModeAction::ConfirmFavorite => confirm_raw_favorite(state),
        RawModeAction::CancelFavorite => cancel_raw_favorite(state),
        RawModeAction::ReprojectCatalog => reconcile_raw_mode(state, catalog),
        RawModeAction::ReprojectAuthority => reproject_raw_authority(state, catalog, authority),
        RawModeAction::DismissNotification => state.notification = None,
    }
    reconcile_raw_mode(state, catalog);
}
