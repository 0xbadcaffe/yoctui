fn raw_output_scroll_bounds(state: &RawModeState) -> (usize, usize) {
    let Some(execution) = state.selected_execution() else {
        return (0, 0);
    };
    let output = match state.output.stream {
        RawOutputStream::Stdout => &execution.stdout,
        RawOutputStream::Stderr => &execution.stderr,
    };
    let maximum_horizontal = output
        .chunks
        .iter()
        .flat_map(|chunk| chunk.text.lines())
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        .saturating_sub(1);
    (output.retained_lines.saturating_sub(1), maximum_horizontal)
}

fn reconcile_raw_output_scroll(state: &mut RawModeState) {
    let (maximum_vertical, maximum_horizontal) = raw_output_scroll_bounds(state);
    state.output.vertical_scroll = state.output.vertical_scroll.min(maximum_vertical);
    state.output.horizontal_scroll = state.output.horizontal_scroll.min(maximum_horizontal);
    if state.output.follow {
        state.output.vertical_scroll = 0;
    }
}

pub fn confirmed_raw_execution_request(
    state: &RawModeState,
    catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>,
    request_id: RawRequestId,
) -> Result<RawConfirmedExecutionRequest, RawExecutionError> {
    if state.view != RawModeView::Preview {
        return Err(RawExecutionError::InvalidLifecycle);
    }
    let form = state
        .form
        .as_ref()
        .ok_or(RawExecutionError::PreviewRequestMismatch)?;
    let reviewed = state
        .preview
        .as_ref()
        .ok_or(RawExecutionError::PreviewRequestMismatch)?;
    let command = catalog
        .command(&form.command)
        .ok_or(RawExecutionError::InvalidCommand)?;
    let mut parameters = BTreeMap::new();
    for definition in &command.parameters {
        let field = form
            .fields
            .get(&definition.id)
            .ok_or(RawExecutionError::InvalidParameterValue)?;
        if let Some(value) = definition
            .parse_value(&field.editor.text)
            .map_err(|_| RawExecutionError::InvalidParameterValue)?
        {
            parameters.insert(definition.id.clone(), value);
        }
    }
    let preview_request = RawPreviewRequest {
        catalog_version: state.catalog_version,
        command: form.command.clone(),
        parameters,
        additional_arguments: RawAdditionalArguments::parse(&form.additional_arguments.editor.text)
            .map_err(|_| RawExecutionError::InvalidParameterValue)?,
        capability_generation: form.capability_generation,
        build_directory: form.build_directory.clone(),
    };
    let current = catalog
        .preview(&preview_request, authority)
        .map_err(|error| RawExecutionError::InvalidReviewedPreview(error.to_string()))?;
    if &current != reviewed {
        return Err(RawExecutionError::PreviewRequestMismatch);
    }
    RawConfirmedExecutionRequest::from_reviewed_preview(
        request_id,
        catalog,
        &preview_request,
        reviewed,
    )
}

fn raw_visible_commands<'a>(state: &RawModeState, catalog: &'a RawCatalog) -> Vec<&'a RawCommand> {
    let query = state.search.query.to_lowercase();
    if !query.is_empty() {
        return catalog
            .commands
            .iter()
            .filter(|command| {
                let category = catalog.category(&command.category);
                [
                    command.label.as_str(),
                    command.description.as_str(),
                    command.reference.command.as_str(),
                    command.reference.description.as_str(),
                    category.map_or("", |category| category.label.as_str()),
                ]
                .iter()
                .any(|value| value.to_lowercase().contains(&query))
            })
            .collect();
    }
    let Some(category_id) = state.category.as_ref() else {
        return Vec::new();
    };
    if catalog
        .category(category_id)
        .is_some_and(|category| category.kind == RawCategoryKind::Favorites)
    {
        return state
            .favorites
            .iter()
            .filter_map(|favorite| catalog.command(&favorite.command))
            .collect();
    }
    catalog
        .commands
        .iter()
        .filter(|command| &command.category == category_id)
        .collect()
}
