pub fn daemon_job_state_from_app(app: &yoctui_model::App) -> yoctui_model::DaemonJobState {
    yoctui_model::DaemonJobState::capture(app)
}

pub fn install_daemon_job_replica(
    app: &mut yoctui_model::App,
    jobs: &yoctui_model::DaemonJobState,
) {
    jobs.install_replica(app);
}

pub fn raw_selector_authority(
    app: &yoctui_model::App,
    selected_recipe: Option<&str>,
) -> yoctui_model::RawSelectorAuthority {
    let workspace_current = app.workspace.build_dir.is_some()
        || app.workspace.source_dir.is_some()
        || app.workspace.bitbake_version.is_some()
        || !app.workspace.variables.is_empty()
        || !app.workspace.recipes.is_empty();
    let recent_targets = app
        .build_history
        .iter()
        .rev()
        .filter_map(|record| record.target.clone())
        .collect::<Vec<_>>();
    let metadata = selected_recipe.and_then(|recipe| app.recipe_metadata.get(recipe));
    let metadata_pending =
        selected_recipe.is_some_and(|recipe| app.recipe_metadata_loading.contains(recipe));
    let metadata_error = selected_recipe
        .and_then(|recipe| app.recipe_metadata_errors.get(recipe))
        .map(String::as_str);
    let multiconfig = workspace_current.then(|| {
        app.workspace
            .variables
            .get("BBMULTICONFIG")
            .map(String::as_str)
            .unwrap_or("")
    });

    yoctui_model::RawSelectorAuthority::project(yoctui_model::RawSelectorSources {
        recipes: workspace_current.then_some(app.workspace.recipes.as_slice()),
        images: workspace_current.then_some(app.available_images.as_slice()),
        current_target: app.build.target.as_deref(),
        recent_targets: Some(&recent_targets),
        selected_recipe,
        recipe_metadata: metadata,
        recipe_metadata_pending: metadata_pending,
        recipe_metadata_error: metadata_error,
        multiconfig,
    })
}

pub fn raw_selector_for_command(
    app: &yoctui_model::App,
    command: &yoctui_model::RawCommand,
    parameter: &yoctui_model::RawParameterId,
    selected_recipe: Option<&str>,
) -> Result<yoctui_model::RawParameterSelector, yoctui_model::RawSelectorError> {
    command.selector(parameter, &raw_selector_authority(app, selected_recipe))
}

pub fn raw_form_selected_recipe<'a>(
    app: &'a yoctui_model::App,
    command: &yoctui_model::RawCommand,
) -> Option<&'a str> {
    let form = app.raw_mode.form.as_ref()?;
    command.parameters.iter().find_map(|parameter| {
        if parameter.kind != yoctui_model::RawParameterKind::Recipe {
            return None;
        }
        match form.fields.get(&parameter.id)?.value.as_ref()? {
            yoctui_model::RawParameterValue::Recipe(recipe) => Some(recipe.as_str()),
            _ => None,
        }
    })
}

pub fn raw_form_parameter_selector(
    app: &yoctui_model::App,
    command: &yoctui_model::RawCommand,
    parameter: &yoctui_model::RawParameterId,
) -> Result<yoctui_model::RawParameterSelector, yoctui_model::RawSelectorError> {
    raw_selector_for_command(
        app,
        command,
        parameter,
        raw_form_selected_recipe(app, command),
    )
}

pub(crate) fn raw_form_selector_choice(
    app: &yoctui_model::App,
    parameter: &yoctui_model::RawParameterId,
    delta: isize,
) -> Option<yoctui_model::RawModeAction> {
    let catalog = yoctui_model::builtin_raw_catalog();
    let form = app.raw_mode.form.as_ref()?;
    let command = catalog.command(&form.command)?;
    let field = form.fields.get(parameter)?;
    let selector = raw_form_parameter_selector(app, command, parameter).ok()?;
    let choices = selector.inventory.choices()?;
    if choices.is_empty() {
        return None;
    }
    let current = field
        .value
        .as_ref()
        .and_then(|value| choices.iter().position(|choice| &choice.value == value));
    let next = current.map_or(0, |current| {
        if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta as usize)
                .min(choices.len().saturating_sub(1))
        }
    });
    Some(yoctui_model::RawModeAction::ChooseParameter {
        parameter: parameter.clone(),
        value: choices[next].value.clone(),
    })
}

pub fn raw_execution_request_to_protocol(
    request: &yoctui_model::RawConfirmedExecutionRequest,
) -> Result<yoctui_protocol::daemon::RawExecutionRequestData, String> {
    use yoctui_protocol::daemon::{
        RAW_EXECUTION_SCHEMA_VERSION, RawExecutionParameterData, RawExecutionRequestData,
    };
    request.validate().map_err(|error| error.to_string())?;
    let wire = RawExecutionRequestData {
        schema_version: RAW_EXECUTION_SCHEMA_VERSION,
        request_id: request.id.as_str().into(),
        catalog_version: request.catalog_version,
        command_id: request.command.as_str().into(),
        parameters: request
            .parameters
            .iter()
            .map(|(id, value)| RawExecutionParameterData {
                id: id.as_str().into(),
                value: raw_parameter_to_protocol(value),
            })
            .collect(),
        additional_arguments: request.additional_arguments.clone(),
        interaction: raw_interaction_to_protocol(request.interaction),
        safety: raw_safety_to_protocol(request.safety),
        capability_generation: request.capability_generation,
        build_directory: request.build_directory.to_string_lossy().into_owned(),
        preview_digest: request.preview_digest.to_hex(),
    };
    wire.validate().map_err(|error| error.to_string())?;
    Ok(wire)
}

pub fn raw_execution_request_from_protocol(
    wire: &yoctui_protocol::daemon::RawExecutionRequestData,
) -> Result<yoctui_model::RawConfirmedExecutionRequest, String> {
    wire.validate().map_err(|error| error.to_string())?;
    let mut parameters = std::collections::BTreeMap::new();
    for parameter in &wire.parameters {
        let id =
            yoctui_model::RawParameterId::new(&parameter.id).map_err(|error| error.to_string())?;
        let value = raw_parameter_from_protocol(&parameter.value)?;
        if parameters.insert(id, value).is_some() {
            return Err("duplicate Raw execution parameter".into());
        }
    }
    let request = yoctui_model::RawConfirmedExecutionRequest {
        id: yoctui_model::RawRequestId::new(&wire.request_id).map_err(|error| error.to_string())?,
        catalog_version: wire.catalog_version,
        command: yoctui_model::RawCommandId::new(&wire.command_id)
            .map_err(|error| error.to_string())?,
        parameters,
        additional_arguments: wire.additional_arguments.clone(),
        interaction: raw_interaction_from_protocol(wire.interaction)?,
        safety: raw_safety_from_protocol(wire.safety)?,
        capability_generation: wire.capability_generation,
        build_directory: std::path::PathBuf::from(&wire.build_directory),
        preview_digest: yoctui_model::RawPreviewDigest::from_hex(&wire.preview_digest)
            .map_err(|error| error.to_string())?,
    };
    request.validate().map_err(|error| error.to_string())?;
    Ok(request)
}

pub(crate) fn raw_parameter_to_protocol(
    value: &yoctui_model::RawParameterValue,
) -> yoctui_protocol::daemon::RawParameterValueData {
    use yoctui_model::RawParameterValue as Model;
    use yoctui_protocol::daemon::RawParameterValueData as Wire;
    match value {
        Model::Recipe(value) => Wire::Recipe(value.clone()),
        Model::Image(value) => Wire::Image(value.clone()),
        Model::Target(value) => Wire::Target(value.clone()),
        Model::Task(value) => Wire::Task(value.clone()),
        Model::UserInterface(value) => Wire::UserInterface(value.clone()),
        Model::File(value) => Wire::File(value.clone()),
        Model::Integer(value) => Wire::Integer(*value),
        Model::Text(value) => Wire::Text(value.clone()),
        Model::Multiconfig(value) => Wire::Multiconfig(value.clone()),
    }
}

pub(crate) fn raw_parameter_from_protocol(
    value: &yoctui_protocol::daemon::RawParameterValueData,
) -> Result<yoctui_model::RawParameterValue, String> {
    use yoctui_model::RawParameterValue as Model;
    use yoctui_protocol::daemon::RawParameterValueData as Wire;
    Ok(match value {
        Wire::Recipe(value) => Model::Recipe(value.clone()),
        Wire::Image(value) => Model::Image(value.clone()),
        Wire::Target(value) => Model::Target(value.clone()),
        Wire::Task(value) => Model::Task(value.clone()),
        Wire::UserInterface(value) => Model::UserInterface(value.clone()),
        Wire::File(value) => Model::File(value.clone()),
        Wire::Integer(value) => Model::Integer(*value),
        Wire::Text(value) => Model::Text(value.clone()),
        Wire::Multiconfig(value) => Model::Multiconfig(value.clone()),
        Wire::Unknown => return Err("unknown required Raw parameter kind".into()),
    })
}

pub(crate) fn raw_interaction_to_protocol(
    value: yoctui_model::RawInteractionMode,
) -> yoctui_protocol::daemon::RawInteractionData {
    match value {
        yoctui_model::RawInteractionMode::NoninteractiveJob => {
            yoctui_protocol::daemon::RawInteractionData::NoninteractiveJob
        }
        yoctui_model::RawInteractionMode::InteractivePty => {
            yoctui_protocol::daemon::RawInteractionData::InteractivePty
        }
    }
}

pub(crate) fn raw_interaction_from_protocol(
    value: yoctui_protocol::daemon::RawInteractionData,
) -> Result<yoctui_model::RawInteractionMode, String> {
    match value {
        yoctui_protocol::daemon::RawInteractionData::NoninteractiveJob => {
            Ok(yoctui_model::RawInteractionMode::NoninteractiveJob)
        }
        yoctui_protocol::daemon::RawInteractionData::InteractivePty => {
            Ok(yoctui_model::RawInteractionMode::InteractivePty)
        }
        yoctui_protocol::daemon::RawInteractionData::Unknown => {
            Err("unknown required Raw interaction class".into())
        }
    }
}

pub(crate) fn raw_safety_to_protocol(
    value: yoctui_model::RawSafetyClass,
) -> yoctui_protocol::daemon::RawSafetyData {
    use yoctui_model::RawSafetyClass as Model;
    use yoctui_protocol::daemon::RawSafetyData as Wire;
    match value {
        Model::Inspection => Wire::Inspection,
        Model::Build => Wire::Build,
        Model::MetadataMutation => Wire::MetadataMutation,
        Model::Destructive => Wire::Destructive,
        Model::ServerLifecycle => Wire::ServerLifecycle,
    }
}

pub(crate) fn raw_safety_from_protocol(
    value: yoctui_protocol::daemon::RawSafetyData,
) -> Result<yoctui_model::RawSafetyClass, String> {
    use yoctui_model::RawSafetyClass as Model;
    use yoctui_protocol::daemon::RawSafetyData as Wire;
    Ok(match value {
        Wire::Inspection => Model::Inspection,
        Wire::Build => Model::Build,
        Wire::MetadataMutation => Model::MetadataMutation,
        Wire::Destructive => Model::Destructive,
        Wire::ServerLifecycle => Model::ServerLifecycle,
        Wire::Unknown => return Err("unknown required Raw safety class".into()),
    })
}

pub(crate) fn raw_owner_to_protocol(
    owner: &yoctui_model::RawExecutionOwner,
) -> yoctui_protocol::daemon::RawExecutionOwnerData {
    match owner {
        yoctui_model::RawExecutionOwner::Job(id) => {
            yoctui_protocol::daemon::RawExecutionOwnerData::Job(id.as_str().into())
        }
        yoctui_model::RawExecutionOwner::Pty(id) => {
            yoctui_protocol::daemon::RawExecutionOwnerData::Pty(id.as_str().into())
        }
    }
}

pub(crate) fn raw_owner_from_protocol(
    owner: &yoctui_protocol::daemon::RawExecutionOwnerData,
) -> Result<yoctui_model::RawExecutionOwner, String> {
    match owner {
        yoctui_protocol::daemon::RawExecutionOwnerData::Job(id) => {
            Ok(yoctui_model::RawExecutionOwner::Job(
                yoctui_model::RawJobId::new(id).map_err(|error| error.to_string())?,
            ))
        }
        yoctui_protocol::daemon::RawExecutionOwnerData::Pty(id) => {
            Ok(yoctui_model::RawExecutionOwner::Pty(
                yoctui_model::RawSessionId::new(id).map_err(|error| error.to_string())?,
            ))
        }
        yoctui_protocol::daemon::RawExecutionOwnerData::Unknown => {
            Err("unknown required Raw owner kind".into())
        }
    }
}
