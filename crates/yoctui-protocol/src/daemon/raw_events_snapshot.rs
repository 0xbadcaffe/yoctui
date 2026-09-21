#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawAttachmentData {
    Attached,
    Detached,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RawExecutionEventKindData {
    Starting {
        owner: RawExecutionOwnerData,
    },
    Running {
        started_unix_ms: u64,
    },
    CancellationRequested,
    Cancelling,
    AttachmentChanged {
        attachment: RawAttachmentData,
    },
    Elapsed {
        elapsed_ms: u64,
    },
    Output {
        chunk: RawOutputChunkData,
    },
    Finished {
        result: RawExecutionResultData,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionEventData {
    pub schema_version: u16,
    pub request_id: String,
    pub sequence: u64,
    pub generation: u64,
    pub event: RawExecutionEventKindData,
}

impl RawExecutionEventData {
    pub fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        validate_raw_schema(self.schema_version)?;
        validate_raw_identity(&self.request_id, "raw-request:", "request")?;
        if self.sequence == 0 || self.generation == 0 {
            return Err(RawExecutionProtocolError::InvalidCorrelation);
        }
        match &self.event {
            RawExecutionEventKindData::Starting { owner } => owner.validate(),
            RawExecutionEventKindData::Output { chunk } => chunk.validate(),
            RawExecutionEventKindData::Finished { result } => result.validate(),
            RawExecutionEventKindData::AttachmentChanged {
                attachment: RawAttachmentData::Unknown,
            } => Err(RawExecutionProtocolError::UnknownRequiredVariant),
            RawExecutionEventKindData::Unknown => {
                Err(RawExecutionProtocolError::UnknownRequiredVariant)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "outcome", rename_all = "snake_case")]
pub enum RawExecutionPhaseData {
    Queued,
    Starting,
    Running,
    Cancelling,
    Terminal(RawExecutionOutcomeData),
    #[serde(other)]
    Unknown,
}

impl RawExecutionPhaseData {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawRetainedOutputData {
    pub stream_id: String,
    pub stream: RawOutputStreamData,
    pub chunks: Vec<RawOutputChunkData>,
    pub next_sequence: u64,
    pub retained_bytes: u64,
    pub retained_lines: u64,
    pub dropped_bytes: u64,
    pub dropped_lines: u64,
    pub truncated_chunks: u64,
}

impl RawRetainedOutputData {
    fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        validate_raw_identity(&self.stream_id, "raw-stream:", "stream")?;
        if matches!(self.stream, RawOutputStreamData::Unknown)
            || self.chunks.len() > MAX_RAW_EXECUTION_RETAINED_LINES
            || self.retained_bytes as usize > MAX_RAW_EXECUTION_RETAINED_BYTES
            || self.retained_lines as usize > MAX_RAW_EXECUTION_RETAINED_LINES
            || self.next_sequence == 0
        {
            return Err(RawExecutionProtocolError::InvalidOutputSnapshot);
        }
        let bytes = self
            .chunks
            .iter()
            .map(|chunk| chunk.text.len())
            .sum::<usize>();
        let lines = self
            .chunks
            .iter()
            .map(|chunk| raw_protocol_line_count(&chunk.text))
            .sum::<usize>();
        if bytes != self.retained_bytes as usize || lines != self.retained_lines as usize {
            return Err(RawExecutionProtocolError::InvalidOutputSnapshot);
        }
        let mut expected = self
            .chunks
            .first()
            .map(|chunk| chunk.sequence)
            .unwrap_or(self.next_sequence);
        for chunk in &self.chunks {
            chunk.validate()?;
            if chunk.stream_id != self.stream_id
                || chunk.stream != self.stream
                || chunk.sequence != expected
            {
                return Err(RawExecutionProtocolError::InvalidOutputSnapshot);
            }
            expected = expected
                .checked_add(1)
                .ok_or(RawExecutionProtocolError::InvalidOutputSnapshot)?;
        }
        if expected != self.next_sequence {
            return Err(RawExecutionProtocolError::InvalidOutputSnapshot);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionSnapshotData {
    pub schema_version: u16,
    pub request: RawExecutionRequestData,
    pub phase: RawExecutionPhaseData,
    pub attachment: RawAttachmentData,
    pub owner: Option<RawExecutionOwnerData>,
    pub cancellation_requested: bool,
    pub queued_unix_ms: u64,
    pub started_unix_ms: Option<u64>,
    pub elapsed_ms: u64,
    pub result: Option<RawExecutionResultData>,
    pub stdout: RawRetainedOutputData,
    pub stderr: RawRetainedOutputData,
    pub sequence: u64,
    pub generation: u64,
}

impl RawExecutionSnapshotData {
    pub fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        validate_raw_schema(self.schema_version)?;
        self.request.validate()?;
        self.stdout.validate()?;
        self.stderr.validate()?;
        if self.stdout.stream_id == self.stderr.stream_id
            || self.stdout.stream != RawOutputStreamData::Stdout
            || self.stderr.stream != RawOutputStreamData::Stderr
            || matches!(self.phase, RawExecutionPhaseData::Unknown)
            || matches!(self.attachment, RawAttachmentData::Unknown)
        {
            return Err(RawExecutionProtocolError::InvalidSnapshot);
        }
        if let Some(owner) = &self.owner {
            owner.validate()?;
            if matches!(
                (&self.request.interaction, owner),
                (
                    RawInteractionData::NoninteractiveJob,
                    RawExecutionOwnerData::Pty(_)
                ) | (
                    RawInteractionData::InteractivePty,
                    RawExecutionOwnerData::Job(_)
                )
            ) {
                return Err(RawExecutionProtocolError::InvalidSnapshot);
            }
        }
        match (&self.phase, &self.result) {
            (RawExecutionPhaseData::Terminal(outcome), Some(result))
                if *outcome == result.outcome && self.elapsed_ms == result.elapsed_ms =>
            {
                result.validate()?;
            }
            (RawExecutionPhaseData::Terminal(_), _) | (_, Some(_)) => {
                return Err(RawExecutionProtocolError::InvalidSnapshot);
            }
            _ => {}
        }
        if matches!(
            self.phase,
            RawExecutionPhaseData::Terminal(RawExecutionOutcomeData::Unknown)
        ) {
            return Err(RawExecutionProtocolError::UnknownRequiredVariant);
        }
        if (self.sequence == 0) != (self.generation == 0) {
            return Err(RawExecutionProtocolError::InvalidCorrelation);
        }
        match self.phase {
            RawExecutionPhaseData::Queued
                if self.owner.is_some() || self.started_unix_ms.is_some() =>
            {
                return Err(RawExecutionProtocolError::InvalidSnapshot);
            }
            RawExecutionPhaseData::Starting
                if self.owner.is_none() || self.started_unix_ms.is_some() =>
            {
                return Err(RawExecutionProtocolError::InvalidSnapshot);
            }
            RawExecutionPhaseData::Running
                if self.owner.is_none() || self.started_unix_ms.is_none() =>
            {
                return Err(RawExecutionProtocolError::InvalidSnapshot);
            }
            RawExecutionPhaseData::Cancelling
                if !self.cancellation_requested
                    || self.started_unix_ms.is_some() && self.owner.is_none() =>
            {
                return Err(RawExecutionProtocolError::InvalidSnapshot);
            }
            RawExecutionPhaseData::Terminal(RawExecutionOutcomeData::Cancelled)
                if !self.cancellation_requested =>
            {
                return Err(RawExecutionProtocolError::InvalidSnapshot);
            }
            _ => {}
        }
        Ok(())
    }
}

fn validate_raw_schema(version: u16) -> Result<(), RawExecutionProtocolError> {
    if version == RAW_EXECUTION_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(RawExecutionProtocolError::UnsupportedSchema(version))
    }
}

fn validate_raw_identity(
    value: &str,
    prefix: &'static str,
    kind: &'static str,
) -> Result<(), RawExecutionProtocolError> {
    let token = value
        .strip_prefix(prefix)
        .ok_or(RawExecutionProtocolError::InvalidIdentity(kind))?;
    if value.len() > MAX_RAW_EXECUTION_ID_BYTES
        || token.is_empty()
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(RawExecutionProtocolError::InvalidIdentity(kind));
    }
    Ok(())
}

fn validate_raw_catalog_id(
    value: &str,
    kind: &'static str,
) -> Result<(), RawExecutionProtocolError> {
    if value.is_empty()
        || value.len() > MAX_RAW_EXECUTION_PARAMETER_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(RawExecutionProtocolError::InvalidIdentity(kind));
    }
    Ok(())
}

fn validate_raw_arguments(arguments: &[String]) -> Result<(), RawExecutionProtocolError> {
    if arguments.len() > MAX_RAW_EXECUTION_ARGUMENTS
        || arguments
            .iter()
            .any(|argument| argument.len() > MAX_RAW_EXECUTION_ARGUMENT_BYTES)
        || arguments.iter().map(String::len).sum::<usize>()
            > MAX_RAW_EXECUTION_ARGUMENT_AGGREGATE_BYTES
    {
        return Err(RawExecutionProtocolError::InvalidArguments);
    }
    Ok(())
}

fn validate_raw_absolute_path(path: &str) -> Result<(), RawExecutionProtocolError> {
    let path = Path::new(path);
    if !path.is_absolute()
        || path.as_os_str().as_encoded_bytes().len() > MAX_RAW_EXECUTION_BUILD_DIRECTORY_BYTES
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(RawExecutionProtocolError::InvalidBuildDirectory);
    }
    Ok(())
}

fn raw_protocol_line_count(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.bytes().filter(|byte| *byte == b'\n').count() + usize::from(!text.ends_with('\n'))
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawExecutionProtocolError {
    #[error("unsupported Raw execution schema version {0}")]
    UnsupportedSchema(u16),
    #[error("invalid Raw execution {0} identity")]
    InvalidIdentity(&'static str),
    #[error("Raw execution authority generation is zero")]
    InvalidAuthority,
    #[error("Raw execution has too many parameters")]
    TooManyParameters,
    #[error("Raw execution repeats a parameter identity")]
    DuplicateParameter,
    #[error("Raw execution snapshot repeats a request identity")]
    DuplicateRequest,
    #[error("Raw execution contains an invalid parameter")]
    InvalidParameter,
    #[error("Raw execution contains invalid bounded arguments")]
    InvalidArguments,
    #[error("Raw execution build directory is not a bounded absolute normalized path")]
    InvalidBuildDirectory,
    #[error("Raw execution preview digest is invalid")]
    InvalidPreviewDigest,
    #[error("Raw execution contains an unknown required enum variant")]
    UnknownRequiredVariant,
    #[error("Raw execution event correlation must be nonzero")]
    InvalidCorrelation,
    #[error("Raw execution output chunk is invalid")]
    InvalidOutputChunk,
    #[error("Raw execution retained output snapshot is invalid")]
    InvalidOutputSnapshot,
    #[error("Raw execution result message exceeds its byte bound")]
    ResultMessageTooLong,
    #[error("Raw execution result is inconsistent")]
    InvalidResult,
    #[error("Raw execution snapshot is inconsistent")]
    InvalidSnapshot,
    #[error("unsupported Raw history schema version {0}")]
    UnsupportedHistorySchema(u16),
    #[error("only terminal Raw execution replicas may enter history")]
    HistoryRequiresTerminal,
    #[error("invalid Raw history record")]
    InvalidHistoryRecord,
    #[error("Raw history has too many records")]
    TooManyHistoryRecords,
    #[error("Raw history is not uniquely ordered newest first")]
    InvalidHistoryOrder,
    #[error("Raw history exceeds its aggregate byte bound")]
    HistoryTooLarge,
}
