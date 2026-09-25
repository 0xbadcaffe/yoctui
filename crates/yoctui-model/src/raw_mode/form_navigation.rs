fn reconcile_raw_mode(state: &mut RawModeState, catalog: &RawCatalog) {
    if state.catalog_version != catalog.version {
        state.catalog_version = catalog.version;
        if state.form.is_some() || state.preview.is_some() {
            state.close_unsafe_work(
                "Raw form closed because the catalog version was replaced.".into(),
            );
        }
    }
    if state
        .category
        .as_ref()
        .is_none_or(|category| catalog.category(category).is_none())
    {
        state.category = catalog
            .browser_categories()
            .first()
            .map(|category| category.id.clone());
    }
    reconcile_raw_command(state, catalog);
    state.history_selection = state
        .history_selection
        .min(state.history.len().saturating_sub(1));
    state.favorite_selection = state
        .favorite_selection
        .min(state.favorites.len().saturating_sub(1));
    reconcile_raw_output_scroll(state);
}

fn reconcile_raw_command(state: &mut RawModeState, catalog: &RawCatalog) {
    let visible = raw_visible_commands(state, catalog);
    if state
        .command
        .as_ref()
        .is_none_or(|selected| !visible.iter().any(|command| &command.id == selected))
    {
        state.command = visible.first().map(|command| command.id.clone());
    }
}

fn shifted_index(current: usize, length: usize, delta: isize) -> usize {
    if length == 0 {
        return 0;
    }
    if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current
            .saturating_add(delta as usize)
            .min(length.saturating_sub(1))
    }
}

fn select_raw_category(state: &mut RawModeState, catalog: &RawCatalog, delta: isize) {
    let categories = catalog.browser_categories();
    let current = state
        .category
        .as_ref()
        .and_then(|selected| {
            categories
                .iter()
                .position(|category| &category.id == selected)
        })
        .unwrap_or(0);
    state.category = categories
        .get(shifted_index(current, categories.len(), delta))
        .map(|category| category.id.clone());
    state.command = None;
    reconcile_raw_command(state, catalog);
}

fn select_raw_command(state: &mut RawModeState, catalog: &RawCatalog, delta: isize) {
    let visible = raw_visible_commands(state, catalog);
    let current = state
        .command
        .as_ref()
        .and_then(|selected| visible.iter().position(|command| &command.id == selected))
        .unwrap_or(0);
    state.command = visible
        .get(shifted_index(current, visible.len(), delta))
        .map(|command| command.id.clone());
}

fn open_raw_selected(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>,
) {
    let Some(command) = state.selected_command(catalog) else {
        state.notification = Some("No Raw command is selected.".into());
        return;
    };
    let RawExecutionPolicy::Executable { .. } = &command.execution else {
        state.notification = Some(
            "This catalog entry is reference-only; its exact help remains inspectable.".into(),
        );
        return;
    };
    open_raw_form(
        state,
        command,
        authority,
        &BTreeMap::new(),
        &RawAdditionalArguments::default(),
    );
}

fn open_raw_form(
    state: &mut RawModeState,
    command: &RawCommand,
    authority: Option<&DaemonCompatibilitySnapshot>,
    defaults: &BTreeMap<RawParameterId, RawParameterValue>,
    additional_arguments: &RawAdditionalArguments,
) {
    let availability = command.availability(authority);
    if !availability.is_enabled() {
        state.notification = Some(raw_availability_reason(&availability));
        return;
    }
    let Some(authority) = authority else {
        state.notification = Some("No current Raw capability authority is installed.".into());
        return;
    };
    let Some(build_directory) = authority.snapshot.environment.build_directory.value() else {
        state.notification =
            Some("The current capability authority has no build-directory identity.".into());
        return;
    };
    let fields = command
        .parameters
        .iter()
        .map(|parameter| {
            let value = defaults.get(&parameter.id).cloned();
            (
                parameter.id.clone(),
                RawFormField {
                    parameter: parameter.id.clone(),
                    editor: PopupEditor::new(
                        value
                            .as_ref()
                            .map_or_else(String::new, RawParameterValue::argument),
                    ),
                    value,
                    validation_error: None,
                },
            )
        })
        .collect();
    let input = additional_arguments
        .as_slice()
        .iter()
        .map(|argument| {
            let escaped = argument.replace('\\', "\\\\").replace('\'', "\\'");
            format!("'{escaped}'")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut argv = RawArgvEditor::new(input).expect("validated favorite argv input is bounded");
    argv.validated = Some(additional_arguments.clone());
    state.form = Some(RawCommandForm {
        command: command.id.clone(),
        fields,
        field_order: command
            .parameters
            .iter()
            .map(|parameter| parameter.id.clone())
            .collect(),
        field_selection: 0,
        additional_arguments: argv,
        capability_generation: authority.snapshot.generation,
        build_directory: build_directory.clone(),
    });
    state.notification = None;
    state.enter_view(RawModeView::Form, RawModeFocus::Form);
}

fn raw_availability_reason(availability: &RawCommandAvailability) -> String {
    if availability.issues.is_empty() {
        return format!("Raw command is {:?}.", availability.state);
    }
    availability
        .issues
        .iter()
        .map(|issue| issue.reason.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn raw_mode_back(state: &mut RawModeState) {
    if state.search.editing {
        state.search.editing = false;
        state.focus = match state.browser_column {
            RawBrowserColumn::Categories => RawModeFocus::Categories,
            RawBrowserColumn::Commands => RawModeFocus::Commands,
        };
        return;
    }
    if state.view != RawModeView::Browser {
        state.leave_view();
    } else if state.browser_column == RawBrowserColumn::Commands {
        state.browser_column = RawBrowserColumn::Categories;
        state.focus = RawModeFocus::Categories;
    }
}

fn set_raw_parameter_input(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    parameter: &RawParameterId,
    input: String,
) {
    let Some(form) = state.form.as_mut() else {
        return;
    };
    let Some(definition) = catalog
        .command(&form.command)
        .and_then(|command| command.parameters.iter().find(|item| &item.id == parameter))
    else {
        state.notification = Some(format!("Raw form has no parameter {parameter}."));
        return;
    };
    let Some(field) = form.fields.get_mut(parameter) else {
        state.notification = Some(format!("Raw form state has no parameter {parameter}."));
        return;
    };
    if input.len() > raw_parameter_input_limit(definition.kind) {
        state.notification = Some(format!(
            "Raw parameter {parameter} input exceeds its typed byte limit."
        ));
        return;
    }
    field.editor = PopupEditor::new(input);
    match definition.parse_value(&field.editor.text) {
        Ok(value) => {
            field.value = value;
            field.validation_error = None;
        }
        Err(error) => {
            field.value = None;
            field.validation_error = Some(error);
        }
    }
    state.preview = None;
}

fn raw_parameter_input_limit(kind: RawParameterKind) -> usize {
    match kind {
        RawParameterKind::Recipe => MAX_RAW_RECIPE_BYTES,
        RawParameterKind::Image => MAX_RAW_IMAGE_BYTES,
        RawParameterKind::Target => MAX_RAW_TARGET_BYTES,
        RawParameterKind::Task => MAX_RAW_TASK_BYTES,
        RawParameterKind::UserInterface => MAX_RAW_UI_BYTES,
        RawParameterKind::File => MAX_RAW_FILE_BYTES,
        RawParameterKind::Integer => MAX_RAW_INTEGER_INPUT_BYTES,
        RawParameterKind::Text => MAX_RAW_PARAMETER_TEXT_BYTES,
        RawParameterKind::Multiconfig => MAX_RAW_MULTICONFIG_BYTES,
    }
}

fn edit_raw_parameter_input(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    parameter: &RawParameterId,
    command: PopupEditorCommand,
) {
    let Some(form) = state.form.as_mut() else {
        return;
    };
    let Some(definition) = catalog
        .command(&form.command)
        .and_then(|command| command.parameters.iter().find(|item| &item.id == parameter))
    else {
        state.notification = Some(format!("Raw form has no parameter {parameter}."));
        return;
    };
    let Some(field) = form.fields.get_mut(parameter) else {
        state.notification = Some(format!("Raw form state has no parameter {parameter}."));
        return;
    };
    let previous = field.editor.clone();
    match command {
        PopupEditorCommand::ToggleInsert => field.editor.toggle_insert(),
        PopupEditorCommand::ToggleVisual => {
            let mode = if field.editor.mode() == crate::TextAreaMode::Visual {
                crate::TextAreaMode::Normal
            } else {
                crate::TextAreaMode::Visual
            };
            field.editor.set_mode(mode);
        }
        PopupEditorCommand::Insert(character)
            if field.editor.editing && !character.is_control() =>
        {
            field.editor.insert(&character.to_string());
        }
        PopupEditorCommand::Insert(_) => {}
        PopupEditorCommand::Newline if field.editor.editing => field.editor.insert("\n"),
        PopupEditorCommand::Newline => {}
        PopupEditorCommand::Backspace if field.editor.editing => field.editor.backspace(),
        PopupEditorCommand::Backspace => {}
        PopupEditorCommand::Delete => field.editor.delete_forward(),
        PopupEditorCommand::Left => field.editor.left(),
        PopupEditorCommand::Right => field.editor.right(),
        PopupEditorCommand::WordLeft => field.editor.move_cursor(crate::TextAreaMotion::WordLeft),
        PopupEditorCommand::WordRight => field.editor.move_cursor(crate::TextAreaMotion::WordRight),
        PopupEditorCommand::Up => field.editor.up(),
        PopupEditorCommand::Down => field.editor.down(),
        PopupEditorCommand::Home => field.editor.home(),
        PopupEditorCommand::End => field.editor.end(),
        PopupEditorCommand::PageUp => field.editor.move_cursor(crate::TextAreaMotion::PageUp),
        PopupEditorCommand::PageDown => field.editor.move_cursor(crate::TextAreaMotion::PageDown),
        PopupEditorCommand::Undo => {
            field.editor.undo();
        }
        PopupEditorCommand::Redo => {
            field.editor.redo();
        }
        PopupEditorCommand::SelectPosition {
            line,
            column,
            extend,
        } => field.editor.select_position(line, column, extend),
        PopupEditorCommand::PasteText { text, source } if field.editor.editing => {
            let _ = field.editor.paste_text(&text, source);
        }
        PopupEditorCommand::PasteText { .. } => {}
        PopupEditorCommand::SelectValue => {
            field.editor.select_range(0, field.editor.text.len());
            field.editor.set_mode(crate::TextAreaMode::Insert);
        }
        PopupEditorCommand::Copy => {
            field.editor.copy_selection_or_line();
        }
        PopupEditorCommand::Paste if field.editor.editing => field.editor.paste(),
        PopupEditorCommand::Paste => {}
    }
    if field.editor.text.len() > raw_parameter_input_limit(definition.kind) {
        field.editor = previous;
    }
    match definition.parse_value(&field.editor.text) {
        Ok(value) => {
            field.value = value;
            field.validation_error = None;
        }
        Err(error) => {
            field.value = None;
            field.validation_error = Some(error);
        }
    }
    state.preview = None;
}

fn choose_raw_parameter(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    parameter: &RawParameterId,
    value: RawParameterValue,
) {
    let Some(form) = state.form.as_ref() else {
        return;
    };
    let Some(definition) = catalog
        .command(&form.command)
        .and_then(|command| command.parameters.iter().find(|item| &item.id == parameter))
    else {
        state.notification = Some(format!("Raw form has no parameter {parameter}."));
        return;
    };
    match definition.validate_value(&value) {
        Ok(()) => set_raw_parameter_input(state, catalog, parameter, value.argument()),
        Err(error) => {
            if let Some(field) = state
                .form
                .as_mut()
                .and_then(|form| form.fields.get_mut(parameter))
            {
                field.value = None;
                field.validation_error = Some(error.clone());
            }
            state.notification = Some(error.to_string());
        }
    }
}

fn select_raw_form_field(state: &mut RawModeState, delta: isize) {
    if let Some(form) = state.form.as_mut() {
        for field in form.fields.values_mut() {
            field.editor.editing = false;
        }
        form.additional_arguments.editor.editing = false;
        let count = form.field_order.len().saturating_add(1);
        if count > 0 {
            form.field_selection = if delta.is_negative() {
                (form.field_selection + count - delta.unsigned_abs() % count) % count
            } else {
                (form.field_selection + delta as usize % count) % count
            };
        }
    }
}
