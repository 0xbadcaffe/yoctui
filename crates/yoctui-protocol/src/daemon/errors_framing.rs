#[derive(Debug, Error)]
pub enum DaemonSnapshotError {
    #[error(transparent)]
    Compatibility(#[from] CompatibilityProtocolError),
    #[error(transparent)]
    RawExecution(#[from] RawExecutionProtocolError),
    #[error("invalid daemon snapshot limit for {0}")]
    InvalidLimit(&'static str),
    #[error("daemon snapshot is {actual} bytes, exceeding the {maximum}-byte limit")]
    SnapshotTooLarge { actual: usize, maximum: usize },
    #[error("daemon event is {actual} bytes, exceeding the {maximum}-byte limit")]
    EventTooLarge { actual: usize, maximum: usize },
    #[error("daemon snapshot sequence space is exhausted")]
    SequenceExhausted,
    #[error("daemon snapshot generation space is exhausted")]
    GenerationExhausted,
    #[error(
        "daemon event gap: expected sequence/generation {expected_sequence}/{expected_generation}, got {actual_sequence}/{actual_generation}"
    )]
    EventGap {
        expected_sequence: u64,
        actual_sequence: u64,
        expected_generation: u64,
        actual_generation: u64,
    },
    #[error("stale compatibility generation: current {current}, received {received}")]
    StaleCompatibilityGeneration { current: u64, received: u64 },
    #[error(
        "stale Raw execution {request_id}: current sequence {current_sequence}, received {received_sequence}"
    )]
    StaleRawExecution {
        request_id: String,
        current_sequence: u64,
        received_sequence: u64,
    },
    #[error("daemon snapshot contains too many Raw executions")]
    TooManyRawExecutions,
    #[error("invalid bounded PTY screen snapshot for session {0:?}")]
    InvalidPtyScreen(PtySessionId),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogSeverity {
    Trace,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    pub request_id: RequestId,
    pub outcome: CommandOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandOutcome {
    Accepted,
    Completed,
    RootfsSources {
        sources: Box<crate::rootfs::RootfsSourcesData>,
    },
    ConfirmationRequired {
        confirmation: ConfirmationLease,
        affected_jobs: Vec<JobId>,
        affected_ptys: Vec<PtySessionId>,
    },
    Rejected {
        code: ProtocolErrorCode,
        message: String,
        current_generation: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolFailure {
    pub request_id: Option<RequestId>,
    pub code: ProtocolErrorCode,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolErrorCode {
    IncompatibleVersion,
    UnsupportedCapability,
    AuthenticationFailed,
    MalformedMessage,
    MessageTooLarge,
    LimitExceeded,
    Timeout,
    StaleClient,
    StaleGeneration,
    Conflict,
    NotFound,
    NotWriter,
    ConfirmationRequired,
    ConfirmationExpired,
    Internal,
}

#[derive(Debug, Error)]
pub enum DaemonProtocolError {
    #[error("daemon frame exceeds {MAX_FRAME_BYTES} byte limit")]
    TooLarge,
    #[error("daemon frame has an invalid length prefix")]
    InvalidLength,
    #[error("invalid daemon JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no compatible daemon protocol version")]
    IncompatibleVersion,
    #[error("too many capabilities")]
    TooManyCapabilities,
}

pub fn negotiate_version(
    minimum: ProtocolVersion,
    maximum: ProtocolVersion,
    daemon: ProtocolVersion,
) -> Result<ProtocolVersion, DaemonProtocolError> {
    if minimum.major != maximum.major
        || daemon.major != minimum.major
        || minimum.minor > maximum.minor
        || daemon.minor < minimum.minor
    {
        return Err(DaemonProtocolError::IncompatibleVersion);
    }
    Ok(ProtocolVersion {
        major: daemon.major,
        minor: daemon.minor.min(maximum.minor),
    })
}

pub fn negotiate_capabilities(
    client: &[Capability],
    daemon: &[Capability],
) -> Result<Vec<Capability>, DaemonProtocolError> {
    if client.len() > MAX_CAPABILITIES || daemon.len() > MAX_CAPABILITIES {
        return Err(DaemonProtocolError::TooManyCapabilities);
    }
    let mut common = client
        .iter()
        .copied()
        .filter(|capability| daemon.contains(capability))
        .collect::<Vec<_>>();
    common.sort();
    common.dedup();
    Ok(common)
}

pub fn encode_frame<T: Serialize>(message: &T) -> Result<Vec<u8>, DaemonProtocolError> {
    let payload = serde_json::to_vec(message)?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(DaemonProtocolError::TooLarge);
    }
    let length = u32::try_from(payload.len()).map_err(|_| DaemonProtocolError::TooLarge)?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_frame<T: for<'de> Deserialize<'de>>(frame: &[u8]) -> Result<T, DaemonProtocolError> {
    if frame.len() < 4 {
        return Err(DaemonProtocolError::InvalidLength);
    }
    let length = u32::from_be_bytes(frame[..4].try_into().expect("four-byte prefix")) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(DaemonProtocolError::TooLarge);
    }
    if frame.len() != length + 4 {
        return Err(DaemonProtocolError::InvalidLength);
    }
    Ok(serde_json::from_slice(&frame[4..])?)
}

#[derive(Debug, Default)]
pub struct FrameDecoder {
    pending: Vec<u8>,
}

impl FrameDecoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, DaemonProtocolError> {
        self.pending.extend_from_slice(bytes);
        let mut frames = Vec::new();
        loop {
            if self.pending.len() < 4 {
                break;
            }
            let length = u32::from_be_bytes(
                self.pending[..4]
                    .try_into()
                    .expect("four-byte length prefix"),
            ) as usize;
            if length > MAX_FRAME_BYTES {
                self.pending.clear();
                return Err(DaemonProtocolError::TooLarge);
            }
            let frame_length = 4 + length;
            if self.pending.len() < frame_length {
                break;
            }
            frames.push(self.pending.drain(..frame_length).collect());
        }
        if self.pending.len() > MAX_FRAME_BYTES + 4 {
            self.pending.clear();
            return Err(DaemonProtocolError::TooLarge);
        }
        Ok(frames)
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}
