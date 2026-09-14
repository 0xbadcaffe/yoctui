//! Raw mapping.

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

pub(crate) fn raw_output_stream_to_protocol(
    stream: yoctui_model::RawOutputStream,
) -> yoctui_protocol::daemon::RawOutputStreamData {
    match stream {
        yoctui_model::RawOutputStream::Stdout => {
            yoctui_protocol::daemon::RawOutputStreamData::Stdout
        }
        yoctui_model::RawOutputStream::Stderr => {
            yoctui_protocol::daemon::RawOutputStreamData::Stderr
        }
    }
}

pub(crate) fn raw_output_stream_from_protocol(
    stream: yoctui_protocol::daemon::RawOutputStreamData,
) -> Result<yoctui_model::RawOutputStream, String> {
    match stream {
        yoctui_protocol::daemon::RawOutputStreamData::Stdout => {
            Ok(yoctui_model::RawOutputStream::Stdout)
        }
        yoctui_protocol::daemon::RawOutputStreamData::Stderr => {
            Ok(yoctui_model::RawOutputStream::Stderr)
        }
        yoctui_protocol::daemon::RawOutputStreamData::Unknown => {
            Err("unknown required Raw output stream".into())
        }
    }
}

pub fn raw_output_chunk_to_protocol(
    chunk: &yoctui_model::RawOutputChunk,
) -> Result<yoctui_protocol::daemon::RawOutputChunkData, String> {
    chunk.validate().map_err(|error| error.to_string())?;
    Ok(yoctui_protocol::daemon::RawOutputChunkData {
        schema_version: yoctui_protocol::daemon::RAW_EXECUTION_SCHEMA_VERSION,
        stream_id: chunk.stream_id.as_str().into(),
        stream: raw_output_stream_to_protocol(chunk.stream),
        sequence: chunk.sequence,
        text: chunk.text.clone(),
        truncated_bytes: chunk.truncated_bytes,
        dropped_lines: chunk.dropped_lines,
    })
}

pub fn raw_output_chunk_from_protocol(
    chunk: &yoctui_protocol::daemon::RawOutputChunkData,
) -> Result<yoctui_model::RawOutputChunk, String> {
    chunk.validate().map_err(|error| error.to_string())?;
    let chunk = yoctui_model::RawOutputChunk {
        stream_id: yoctui_model::RawStreamId::new(&chunk.stream_id)
            .map_err(|error| error.to_string())?,
        stream: raw_output_stream_from_protocol(chunk.stream)?,
        sequence: chunk.sequence,
        text: chunk.text.clone(),
        truncated_bytes: chunk.truncated_bytes,
        dropped_lines: chunk.dropped_lines,
    };
    chunk.validate().map_err(|error| error.to_string())?;
    Ok(chunk)
}

pub(crate) fn raw_outcome_to_protocol(
    outcome: yoctui_model::RawExecutionOutcome,
) -> yoctui_protocol::daemon::RawExecutionOutcomeData {
    use yoctui_model::RawExecutionOutcome as Model;
    use yoctui_protocol::daemon::RawExecutionOutcomeData as Wire;
    match outcome {
        Model::Succeeded => Wire::Succeeded,
        Model::Failed => Wire::Failed,
        Model::Cancelled => Wire::Cancelled,
        Model::Lost => Wire::Lost,
    }
}

pub(crate) fn raw_outcome_from_protocol(
    outcome: yoctui_protocol::daemon::RawExecutionOutcomeData,
) -> Result<yoctui_model::RawExecutionOutcome, String> {
    use yoctui_model::RawExecutionOutcome as Model;
    use yoctui_protocol::daemon::RawExecutionOutcomeData as Wire;
    Ok(match outcome {
        Wire::Succeeded => Model::Succeeded,
        Wire::Failed => Model::Failed,
        Wire::Cancelled => Model::Cancelled,
        Wire::Lost => Model::Lost,
        Wire::Unknown => return Err("unknown required Raw terminal outcome".into()),
    })
}

pub fn raw_execution_result_to_protocol(
    result: &yoctui_model::RawExecutionResult,
) -> Result<yoctui_protocol::daemon::RawExecutionResultData, String> {
    result.validate().map_err(|error| error.to_string())?;
    Ok(yoctui_protocol::daemon::RawExecutionResultData {
        schema_version: yoctui_protocol::daemon::RAW_EXECUTION_SCHEMA_VERSION,
        outcome: raw_outcome_to_protocol(result.outcome),
        exit_code: result.exit_code,
        message: result.message.clone(),
        elapsed_ms: result.elapsed_ms,
        durable_reference: result
            .durable_reference
            .as_ref()
            .map(|reference| reference.as_str().into()),
    })
}

pub fn raw_execution_result_from_protocol(
    result: &yoctui_protocol::daemon::RawExecutionResultData,
) -> Result<yoctui_model::RawExecutionResult, String> {
    result.validate().map_err(|error| error.to_string())?;
    let result = yoctui_model::RawExecutionResult {
        outcome: raw_outcome_from_protocol(result.outcome)?,
        exit_code: result.exit_code,
        message: result.message.clone(),
        elapsed_ms: result.elapsed_ms,
        durable_reference: result
            .durable_reference
            .as_deref()
            .map(yoctui_model::RawDurableReferenceId::new)
            .transpose()
            .map_err(|error| error.to_string())?,
    };
    result.validate().map_err(|error| error.to_string())?;
    Ok(result)
}

pub fn raw_execution_event_to_protocol(
    event: &yoctui_model::RawExecutionEvent,
) -> Result<yoctui_protocol::daemon::RawExecutionEventData, String> {
    use yoctui_model::RawExecutionEventKind as Model;
    use yoctui_protocol::daemon::{RawAttachmentData, RawExecutionEventKindData as Wire};
    let kind = match &event.kind {
        Model::Starting { owner } => Wire::Starting {
            owner: raw_owner_to_protocol(owner),
        },
        Model::Running { started_unix_ms } => Wire::Running {
            started_unix_ms: *started_unix_ms,
        },
        Model::CancellationRequested => Wire::CancellationRequested,
        Model::Cancelling => Wire::Cancelling,
        Model::AttachmentChanged { attachment } => Wire::AttachmentChanged {
            attachment: match attachment {
                yoctui_model::RawAttachmentState::Attached => RawAttachmentData::Attached,
                yoctui_model::RawAttachmentState::Detached => RawAttachmentData::Detached,
            },
        },
        Model::Elapsed { elapsed_ms } => Wire::Elapsed {
            elapsed_ms: *elapsed_ms,
        },
        Model::Output { chunk } => Wire::Output {
            chunk: raw_output_chunk_to_protocol(chunk)?,
        },
        Model::Finished { result } => Wire::Finished {
            result: raw_execution_result_to_protocol(result)?,
        },
    };
    let wire = yoctui_protocol::daemon::RawExecutionEventData {
        schema_version: yoctui_protocol::daemon::RAW_EXECUTION_SCHEMA_VERSION,
        request_id: event.request_id.as_str().into(),
        sequence: event.sequence,
        generation: event.generation,
        event: kind,
    };
    wire.validate().map_err(|error| error.to_string())?;
    Ok(wire)
}

pub fn raw_execution_event_from_protocol(
    event: &yoctui_protocol::daemon::RawExecutionEventData,
) -> Result<yoctui_model::RawExecutionEvent, String> {
    use yoctui_model::{RawAttachmentState, RawExecutionEventKind as Model};
    use yoctui_protocol::daemon::{RawAttachmentData, RawExecutionEventKindData as Wire};
    event.validate().map_err(|error| error.to_string())?;
    let kind = match &event.event {
        Wire::Starting { owner } => Model::Starting {
            owner: raw_owner_from_protocol(owner)?,
        },
        Wire::Running { started_unix_ms } => Model::Running {
            started_unix_ms: *started_unix_ms,
        },
        Wire::CancellationRequested => Model::CancellationRequested,
        Wire::Cancelling => Model::Cancelling,
        Wire::AttachmentChanged { attachment } => Model::AttachmentChanged {
            attachment: match attachment {
                RawAttachmentData::Attached => RawAttachmentState::Attached,
                RawAttachmentData::Detached => RawAttachmentState::Detached,
                RawAttachmentData::Unknown => {
                    return Err("unknown required Raw attachment state".into());
                }
            },
        },
        Wire::Elapsed { elapsed_ms } => Model::Elapsed {
            elapsed_ms: *elapsed_ms,
        },
        Wire::Output { chunk } => Model::Output {
            chunk: raw_output_chunk_from_protocol(chunk)?,
        },
        Wire::Finished { result } => Model::Finished {
            result: raw_execution_result_from_protocol(result)?,
        },
        Wire::Unknown => return Err("unknown required Raw execution event".into()),
    };
    Ok(yoctui_model::RawExecutionEvent {
        request_id: yoctui_model::RawRequestId::new(&event.request_id)
            .map_err(|error| error.to_string())?,
        sequence: event.sequence,
        generation: event.generation,
        kind,
    })
}

pub(crate) fn raw_retained_output_to_protocol(
    output: &yoctui_model::RawRetainedOutput,
) -> Result<yoctui_protocol::daemon::RawRetainedOutputData, String> {
    output.validate().map_err(|error| error.to_string())?;
    Ok(yoctui_protocol::daemon::RawRetainedOutputData {
        stream_id: output.stream_id.as_str().into(),
        stream: raw_output_stream_to_protocol(output.stream),
        chunks: output
            .chunks
            .iter()
            .map(raw_output_chunk_to_protocol)
            .collect::<Result<_, _>>()?,
        next_sequence: output.next_sequence,
        retained_bytes: output.retained_bytes as u64,
        retained_lines: output.retained_lines as u64,
        dropped_bytes: output.dropped_bytes,
        dropped_lines: output.dropped_lines,
        truncated_chunks: output.truncated_chunks,
    })
}

pub(crate) fn raw_retained_output_from_protocol(
    output: &yoctui_protocol::daemon::RawRetainedOutputData,
) -> Result<yoctui_model::RawRetainedOutput, String> {
    let output = yoctui_model::RawRetainedOutput {
        stream_id: yoctui_model::RawStreamId::new(&output.stream_id)
            .map_err(|error| error.to_string())?,
        stream: raw_output_stream_from_protocol(output.stream)?,
        chunks: output
            .chunks
            .iter()
            .map(raw_output_chunk_from_protocol)
            .collect::<Result<std::collections::VecDeque<_>, _>>()?,
        next_sequence: output.next_sequence,
        retained_bytes: usize::try_from(output.retained_bytes)
            .map_err(|_| "Raw retained byte count exceeds platform bounds")?,
        retained_lines: usize::try_from(output.retained_lines)
            .map_err(|_| "Raw retained line count exceeds platform bounds")?,
        dropped_bytes: output.dropped_bytes,
        dropped_lines: output.dropped_lines,
        truncated_chunks: output.truncated_chunks,
    };
    output.validate().map_err(|error| error.to_string())?;
    Ok(output)
}

pub fn raw_execution_snapshot_to_protocol(
    state: &yoctui_model::RawExecutionState,
) -> Result<yoctui_protocol::daemon::RawExecutionSnapshotData, String> {
    use yoctui_model::RawExecutionPhase as ModelPhase;
    use yoctui_protocol::daemon::{
        RawAttachmentData, RawExecutionPhaseData as WirePhase, RawExecutionSnapshotData,
    };
    state.validate().map_err(|error| error.to_string())?;
    let wire = RawExecutionSnapshotData {
        schema_version: yoctui_protocol::daemon::RAW_EXECUTION_SCHEMA_VERSION,
        request: raw_execution_request_to_protocol(&state.request)?,
        phase: match state.phase {
            ModelPhase::Queued => WirePhase::Queued,
            ModelPhase::Starting => WirePhase::Starting,
            ModelPhase::Running => WirePhase::Running,
            ModelPhase::Cancelling => WirePhase::Cancelling,
            ModelPhase::Terminal(outcome) => WirePhase::Terminal(raw_outcome_to_protocol(outcome)),
        },
        attachment: match state.attachment {
            yoctui_model::RawAttachmentState::Attached => RawAttachmentData::Attached,
            yoctui_model::RawAttachmentState::Detached => RawAttachmentData::Detached,
        },
        owner: state.owner.as_ref().map(raw_owner_to_protocol),
        cancellation_requested: state.cancellation_requested,
        queued_unix_ms: state.queued_unix_ms,
        started_unix_ms: state.started_unix_ms,
        elapsed_ms: state.elapsed_ms,
        result: state
            .result
            .as_ref()
            .map(raw_execution_result_to_protocol)
            .transpose()?,
        stdout: raw_retained_output_to_protocol(&state.stdout)?,
        stderr: raw_retained_output_to_protocol(&state.stderr)?,
        sequence: state.cursor.sequence,
        generation: state.cursor.generation,
    };
    wire.validate().map_err(|error| error.to_string())?;
    Ok(wire)
}

pub fn raw_execution_snapshot_from_protocol(
    wire: &yoctui_protocol::daemon::RawExecutionSnapshotData,
) -> Result<yoctui_model::RawExecutionState, String> {
    use yoctui_model::{RawAttachmentState, RawExecutionPhase as ModelPhase};
    use yoctui_protocol::daemon::{RawAttachmentData, RawExecutionPhaseData as WirePhase};
    wire.validate().map_err(|error| error.to_string())?;
    let state = yoctui_model::RawExecutionState {
        request: raw_execution_request_from_protocol(&wire.request)?,
        phase: match wire.phase {
            WirePhase::Queued => ModelPhase::Queued,
            WirePhase::Starting => ModelPhase::Starting,
            WirePhase::Running => ModelPhase::Running,
            WirePhase::Cancelling => ModelPhase::Cancelling,
            WirePhase::Terminal(outcome) => {
                ModelPhase::Terminal(raw_outcome_from_protocol(outcome)?)
            }
            WirePhase::Unknown => return Err("unknown required Raw lifecycle phase".into()),
        },
        attachment: match wire.attachment {
            RawAttachmentData::Attached => RawAttachmentState::Attached,
            RawAttachmentData::Detached => RawAttachmentState::Detached,
            RawAttachmentData::Unknown => {
                return Err("unknown required Raw attachment state".into());
            }
        },
        owner: wire
            .owner
            .as_ref()
            .map(raw_owner_from_protocol)
            .transpose()?,
        cancellation_requested: wire.cancellation_requested,
        queued_unix_ms: wire.queued_unix_ms,
        started_unix_ms: wire.started_unix_ms,
        elapsed_ms: wire.elapsed_ms,
        result: wire
            .result
            .as_ref()
            .map(raw_execution_result_from_protocol)
            .transpose()?,
        stdout: raw_retained_output_from_protocol(&wire.stdout)?,
        stderr: raw_retained_output_from_protocol(&wire.stderr)?,
        cursor: yoctui_model::RawEventCursor {
            sequence: wire.sequence,
            generation: wire.generation,
        },
    };
    state.validate().map_err(|error| error.to_string())?;
    Ok(state)
}

pub(crate) fn install_raw_execution_snapshots(
    app: &mut yoctui_model::App,
    snapshots: &[yoctui_protocol::daemon::RawExecutionSnapshotData],
) -> Result<(), String> {
    if snapshots.len() > yoctui_model::MAX_RAW_EXECUTION_REQUESTS {
        return Err("too many Raw execution snapshots".into());
    }
    let mut executions = std::collections::BTreeMap::new();
    for snapshot in snapshots {
        let state = raw_execution_snapshot_from_protocol(snapshot)?;
        if executions.insert(state.request.id.clone(), state).is_some() {
            return Err("duplicate Raw execution request snapshot".into());
        }
    }
    app.raw_mode.execution_states = executions;
    Ok(())
}

pub fn raw_history_record_from_protocol(
    wire: &yoctui_protocol::daemon::RawHistoryRecordData,
) -> Result<yoctui_model::RawHistoryRecord, String> {
    wire.validate().map_err(|error| error.to_string())?;
    let mut parameters = std::collections::BTreeMap::new();
    for parameter in &wire.parameters {
        let id =
            yoctui_model::RawParameterId::new(&parameter.id).map_err(|error| error.to_string())?;
        let value = raw_parameter_from_protocol(&parameter.value)?;
        if parameters.insert(id, value).is_some() {
            return Err("duplicate Raw history parameter".into());
        }
    }
    let record = yoctui_model::RawHistoryRecord {
        schema_version: wire.schema_version,
        request_id: yoctui_model::RawRequestId::new(&wire.request_id)
            .map_err(|error| error.to_string())?,
        catalog_version: wire.catalog_version,
        command: yoctui_model::RawCommandId::new(&wire.command_id)
            .map_err(|error| error.to_string())?,
        parameters,
        interaction: match wire.interaction {
            yoctui_protocol::daemon::RawInteractionData::NoninteractiveJob => {
                yoctui_model::RawInteractionMode::NoninteractiveJob
            }
            yoctui_protocol::daemon::RawInteractionData::InteractivePty => {
                yoctui_model::RawInteractionMode::InteractivePty
            }
            yoctui_protocol::daemon::RawInteractionData::Unknown => {
                return Err("unknown required Raw history interaction".into());
            }
        },
        started_unix_ms: wire.started_unix_ms,
        ended_unix_ms: wire.ended_unix_ms,
        outcome: raw_outcome_from_protocol(wire.outcome)?,
        exit_code: wire.exit_code,
        durable_reference: wire
            .durable_reference
            .as_deref()
            .map(yoctui_model::RawDurableReferenceId::new)
            .transpose()
            .map_err(|error| error.to_string())?,
    };
    record.validate().map_err(|error| error.to_string())?;
    Ok(record)
}

pub(crate) fn install_raw_history_snapshots(
    app: &mut yoctui_model::App,
    records: &[yoctui_protocol::daemon::RawHistoryRecordData],
) -> Result<(), String> {
    let records = records
        .iter()
        .map(raw_history_record_from_protocol)
        .collect::<Result<Vec<_>, _>>()?;
    yoctui_model::install_raw_history(&mut app.raw_mode.history, records)
        .map_err(|error| error.to_string())?;
    app.raw_mode.history_selection = app
        .raw_mode
        .history_selection
        .min(app.raw_mode.history.len().saturating_sub(1));
    Ok(())
}
