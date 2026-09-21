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
