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
