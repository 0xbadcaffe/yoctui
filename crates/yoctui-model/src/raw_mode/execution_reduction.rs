#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionEvent {
    pub request_id: RawRequestId,
    pub sequence: u64,
    pub generation: u64,
    pub kind: RawExecutionEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RawExecutionEventKind {
    Starting { owner: RawExecutionOwner },
    Running { started_unix_ms: u64 },
    CancellationRequested,
    Cancelling,
    AttachmentChanged { attachment: RawAttachmentState },
    Elapsed { elapsed_ms: u64 },
    Output { chunk: RawOutputChunk },
    Finished { result: RawExecutionResult },
}

pub fn reduce_raw_execution(
    state: &mut RawExecutionState,
    event: RawExecutionEvent,
) -> Result<bool, RawExecutionError> {
    if event.request_id != state.request.id {
        return Err(RawExecutionError::WrongRequest);
    }
    if event.sequence <= state.cursor.sequence || event.generation <= state.cursor.generation {
        return Ok(false);
    }
    let expected_sequence = state
        .cursor
        .sequence
        .checked_add(1)
        .ok_or(RawExecutionError::SequenceExhausted)?;
    let expected_generation = state
        .cursor
        .generation
        .checked_add(1)
        .ok_or(RawExecutionError::GenerationExhausted)?;
    if event.sequence != expected_sequence || event.generation != expected_generation {
        return Err(RawExecutionError::EventGap {
            expected_sequence,
            actual_sequence: event.sequence,
            expected_generation,
            actual_generation: event.generation,
        });
    }
    let mut next = state.clone();
    match event.kind {
        RawExecutionEventKind::Starting { owner } => {
            if next.phase != RawExecutionPhase::Queued {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            next.owner = Some(owner);
            next.phase = RawExecutionPhase::Starting;
        }
        RawExecutionEventKind::Running { started_unix_ms } => {
            if next.phase != RawExecutionPhase::Starting || next.owner.is_none() {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            next.started_unix_ms = Some(started_unix_ms);
            next.phase = RawExecutionPhase::Running;
        }
        RawExecutionEventKind::CancellationRequested => {
            if next.phase.is_terminal() {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            next.cancellation_requested = true;
        }
        RawExecutionEventKind::Cancelling => {
            if !next.cancellation_requested
                || !matches!(
                    next.phase,
                    RawExecutionPhase::Queued
                        | RawExecutionPhase::Starting
                        | RawExecutionPhase::Running
                )
            {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            next.phase = RawExecutionPhase::Cancelling;
        }
        RawExecutionEventKind::AttachmentChanged { attachment } => {
            next.attachment = attachment;
        }
        RawExecutionEventKind::Elapsed { elapsed_ms } => {
            if next.phase.is_terminal() || elapsed_ms < next.elapsed_ms {
                return Err(RawExecutionError::InvalidElapsed);
            }
            next.elapsed_ms = elapsed_ms;
        }
        RawExecutionEventKind::Output { chunk } => {
            if !matches!(
                next.phase,
                RawExecutionPhase::Starting
                    | RawExecutionPhase::Running
                    | RawExecutionPhase::Cancelling
            ) {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            match chunk.stream {
                RawOutputStream::Stdout => {
                    if !next.stdout.append(chunk)? {
                        return Ok(false);
                    }
                }
                RawOutputStream::Stderr => {
                    if !next.stderr.append(chunk)? {
                        return Ok(false);
                    }
                }
            }
        }
        RawExecutionEventKind::Finished { result } => {
            result.validate()?;
            if next.phase.is_terminal()
                || (result.outcome == RawExecutionOutcome::Cancelled
                    && !next.cancellation_requested)
                || (result.outcome == RawExecutionOutcome::Succeeded
                    && next.phase != RawExecutionPhase::Running)
            {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            next.elapsed_ms = result.elapsed_ms;
            next.phase = RawExecutionPhase::Terminal(result.outcome);
            next.result = Some(result);
        }
    }
    next.cursor = RawEventCursor {
        sequence: event.sequence,
        generation: event.generation,
    };
    next.validate()?;
    *state = next;
    Ok(true)
}

pub fn replace_raw_execution_snapshot(
    state: &mut Option<RawExecutionState>,
    replacement: RawExecutionState,
) -> Result<bool, RawExecutionError> {
    replacement.validate()?;
    if let Some(current) = state
        && current.request.id == replacement.request.id
        && (replacement.cursor.sequence <= current.cursor.sequence
            || replacement.cursor.generation <= current.cursor.generation)
    {
        return Ok(false);
    }
    *state = Some(replacement);
    Ok(true)
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawExecutionError {
    #[error("invalid Raw {kind} identity: {value:?}")]
    InvalidIdentity { kind: &'static str, value: String },
    #[error("invalid Raw command identity")]
    InvalidCommand,
    #[error("Raw execution authority must use nonzero catalog and capability generations")]
    InvalidAuthority,
    #[error("Raw preview and preview request do not describe the same reviewed work")]
    PreviewRequestMismatch,
    #[error("Raw reviewed preview is invalid: {0}")]
    InvalidReviewedPreview(String),
    #[error("Raw preview digest is not a 32-byte hexadecimal SHA-256 digest")]
    InvalidPreviewDigest,
    #[error("Raw execution contains too many typed parameters")]
    TooManyParameters,
    #[error("Raw execution contains an invalid typed parameter value")]
    InvalidParameterValue,
    #[error("Raw execution build directory is not a bounded absolute normalized identity")]
    InvalidBuildDirectory,
    #[error(transparent)]
    InvalidAdditionalArguments(#[from] RawArgvError),
    #[error("Raw stdout and stderr must use distinct stream identities")]
    DuplicateStreamIdentity,
    #[error("Raw execution owner does not match its interaction class")]
    WrongOwnerKind,
    #[error("Raw output chunk is empty-sequence or exceeds its byte bound")]
    InvalidOutputChunk,
    #[error("Raw output chunk names the wrong typed stream")]
    WrongOutputStream,
    #[error("Raw output sequence gap: expected {expected}, got {actual}")]
    OutputGap { expected: u64, actual: u64 },
    #[error("Raw output snapshot is inconsistent or exceeds retained bounds")]
    InvalidOutputSnapshot,
    #[error("Raw execution event belongs to a different request")]
    WrongRequest,
    #[error(
        "Raw execution event gap: expected sequence/generation {expected_sequence}/{expected_generation}, got {actual_sequence}/{actual_generation}"
    )]
    EventGap {
        expected_sequence: u64,
        actual_sequence: u64,
        expected_generation: u64,
        actual_generation: u64,
    },
    #[error("Raw execution event sequence is exhausted")]
    SequenceExhausted,
    #[error("Raw execution generation is exhausted")]
    GenerationExhausted,
    #[error("invalid Raw execution lifecycle transition")]
    InvalidLifecycle,
    #[error("Raw execution elapsed time moved backwards or changed after termination")]
    InvalidElapsed,
    #[error("Raw execution result is inconsistent with its outcome")]
    InvalidResult,
    #[error("Raw execution result message exceeds its byte bound")]
    ResultMessageTooLong,
    #[error("unsupported Raw history schema version {0}")]
    UnsupportedHistorySchema(u16),
    #[error("only terminal Raw execution replicas may enter history")]
    HistoryRequiresTerminal,
    #[error("invalid Raw history record")]
    InvalidHistoryRecord,
}
