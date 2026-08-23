//! Typed, bounded protocol for the persistent daemon and attachable clients.
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    path::{Component, Path},
};
use thiserror::Error;

use crate::{TaskStatsData, WorkspaceData};

pub const PROTOCOL_MAJOR: u16 = 1;
pub const PROTOCOL_MINOR: u16 = 0;
pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CAPABILITIES: usize = 128;
pub const MAX_RETAINED_EVENTS: usize = 65_536;
pub const MAX_SNAPSHOT_LOGS: usize = 100_000;
pub const MAX_DAEMON_CLIENTS: usize = 32;
pub const MAX_DAEMON_PTY_SESSIONS: usize = 64;
pub const MAX_TERMINAL_SCROLLBACK_LINES: usize = 100_000;
pub const MAX_TERMINAL_ROWS: u16 = 512;
pub const MAX_TERMINAL_COLUMNS: u16 = 512;
pub const MAX_UTILITY_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PTY_OUTPUT_EVENT_BYTES: usize = 64 * 1024;
pub const MAX_DAEMON_BUILD_EVENTS: usize = 2_048;
pub const COMPATIBILITY_SCHEMA_VERSION: u16 = 1;
pub const MAX_COMPATIBILITY_CAPABILITIES: usize = 512;
pub const MAX_COMPATIBILITY_EVIDENCE: usize = 32;
pub const MAX_COMPATIBILITY_ITEMS: usize = 256;
pub const MAX_COMPATIBILITY_TEXT_BYTES: usize = 4_096;
pub const MAX_COMPATIBILITY_ARGV: usize = 64;
pub const RAW_EXECUTION_SCHEMA_VERSION: u16 = 1;
pub const MAX_RAW_EXECUTION_ID_BYTES: usize = 96;
pub const MAX_RAW_EXECUTION_REQUESTS: usize = 64;
pub const MAX_RAW_EXECUTION_PARAMETERS: usize = 32;
pub const MAX_RAW_EXECUTION_PARAMETER_ID_BYTES: usize = 96;
pub const MAX_RAW_EXECUTION_PARAMETER_BYTES: usize = 4_096;
pub const MAX_RAW_EXECUTION_ARGUMENTS: usize = 64;
pub const MAX_RAW_EXECUTION_ARGUMENT_BYTES: usize = 512;
pub const MAX_RAW_EXECUTION_ARGUMENT_AGGREGATE_BYTES: usize = 8_192;
pub const MAX_RAW_EXECUTION_BUILD_DIRECTORY_BYTES: usize = 4_096;
pub const MAX_RAW_EXECUTION_OUTPUT_CHUNK_BYTES: usize = 64 * 1_024;
pub const MAX_RAW_EXECUTION_RETAINED_BYTES: usize = 1_024 * 1_024;
pub const MAX_RAW_EXECUTION_RETAINED_LINES: usize = 10_000;
pub const MAX_RAW_EXECUTION_MESSAGE_BYTES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self {
        major: PROTOCOL_MAJOR,
        minor: PROTOCOL_MINOR,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub [u8; 16]);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DaemonInstanceId(pub [u8; 16]);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PtySessionId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionRequestData {
    pub schema_version: u16,
    pub request_id: String,
    pub catalog_version: u16,
    pub command_id: String,
    pub parameters: Vec<RawExecutionParameterData>,
    pub additional_arguments: Vec<String>,
    pub interaction: RawInteractionData,
    pub safety: RawSafetyData,
    pub capability_generation: u64,
    pub build_directory: String,
    pub preview_digest: String,
}

impl RawExecutionRequestData {
    pub fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        validate_raw_schema(self.schema_version)?;
        validate_raw_identity(&self.request_id, "raw-request:", "request")?;
        validate_raw_catalog_id(&self.command_id, "command")?;
        if self.catalog_version == 0 || self.capability_generation == 0 {
            return Err(RawExecutionProtocolError::InvalidAuthority);
        }
        if self.parameters.len() > MAX_RAW_EXECUTION_PARAMETERS {
            return Err(RawExecutionProtocolError::TooManyParameters);
        }
        let mut parameter_ids = std::collections::BTreeSet::new();
        for parameter in &self.parameters {
            parameter.validate()?;
            if !parameter_ids.insert(&parameter.id) {
                return Err(RawExecutionProtocolError::DuplicateParameter);
            }
        }
        validate_raw_arguments(&self.additional_arguments)?;
        validate_raw_absolute_path(&self.build_directory)?;
        if self.preview_digest.len() != 64
            || !self
                .preview_digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(RawExecutionProtocolError::InvalidPreviewDigest);
        }
        if matches!(self.interaction, RawInteractionData::Unknown)
            || matches!(self.safety, RawSafetyData::Unknown)
        {
            return Err(RawExecutionProtocolError::UnknownRequiredVariant);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionParameterData {
    pub id: String,
    pub value: RawParameterValueData,
}

impl RawExecutionParameterData {
    fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        validate_raw_catalog_id(&self.id, "parameter")?;
        let bytes = match &self.value {
            RawParameterValueData::Recipe(value)
            | RawParameterValueData::Image(value)
            | RawParameterValueData::Target(value)
            | RawParameterValueData::Task(value)
            | RawParameterValueData::UserInterface(value)
            | RawParameterValueData::File(value)
            | RawParameterValueData::Text(value)
            | RawParameterValueData::Multiconfig(value) => value.len(),
            RawParameterValueData::Integer(_) => 1,
            RawParameterValueData::Unknown => {
                return Err(RawExecutionProtocolError::UnknownRequiredVariant);
            }
        };
        if bytes == 0 || bytes > MAX_RAW_EXECUTION_PARAMETER_BYTES {
            return Err(RawExecutionProtocolError::InvalidParameter);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum RawParameterValueData {
    Recipe(String),
    Image(String),
    Target(String),
    Task(String),
    UserInterface(String),
    File(String),
    Integer(u32),
    Text(String),
    Multiconfig(String),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawInteractionData {
    NoninteractiveJob,
    InteractivePty,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawSafetyData {
    Inspection,
    Build,
    MetadataMutation,
    Destructive,
    ServerLifecycle,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum RawExecutionOwnerData {
    Job(String),
    Pty(String),
    #[serde(other)]
    Unknown,
}

impl RawExecutionOwnerData {
    fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        match self {
            Self::Job(id) => validate_raw_identity(id, "raw-job:", "job"),
            Self::Pty(id) => validate_raw_identity(id, "raw-session:", "session"),
            Self::Unknown => Err(RawExecutionProtocolError::UnknownRequiredVariant),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawOutputStreamData {
    Stdout,
    Stderr,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawOutputChunkData {
    pub schema_version: u16,
    pub stream_id: String,
    pub stream: RawOutputStreamData,
    pub sequence: u64,
    pub text: String,
    pub truncated_bytes: u64,
    pub dropped_lines: u64,
}

impl RawOutputChunkData {
    pub fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        validate_raw_schema(self.schema_version)?;
        validate_raw_identity(&self.stream_id, "raw-stream:", "stream")?;
        if self.sequence == 0
            || self.text.len() > MAX_RAW_EXECUTION_OUTPUT_CHUNK_BYTES
            || matches!(self.stream, RawOutputStreamData::Unknown)
        {
            return Err(RawExecutionProtocolError::InvalidOutputChunk);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawExecutionOutcomeData {
    Succeeded,
    Failed,
    Cancelled,
    Lost,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionResultData {
    pub schema_version: u16,
    pub outcome: RawExecutionOutcomeData,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
    pub elapsed_ms: u64,
    pub durable_reference: Option<String>,
}

impl RawExecutionResultData {
    pub fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        validate_raw_schema(self.schema_version)?;
        if matches!(self.outcome, RawExecutionOutcomeData::Unknown) {
            return Err(RawExecutionProtocolError::UnknownRequiredVariant);
        }
        if self
            .message
            .as_ref()
            .is_some_and(|message| message.len() > MAX_RAW_EXECUTION_MESSAGE_BYTES)
        {
            return Err(RawExecutionProtocolError::ResultMessageTooLong);
        }
        if let Some(reference) = &self.durable_reference {
            validate_raw_identity(reference, "raw-durable:", "durable reference")?;
        }
        if matches!(self.outcome, RawExecutionOutcomeData::Lost) && self.exit_code.is_some() {
            return Err(RawExecutionProtocolError::InvalidResult);
        }
        if matches!(self.outcome, RawExecutionOutcomeData::Succeeded) && self.exit_code != Some(0) {
            return Err(RawExecutionProtocolError::InvalidResult);
        }
        Ok(())
    }
}

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
        0
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    StateSnapshots,
    IncrementalEvents,
    EventReplay,
    BackgroundJobs,
    BitBakeLifecycle,
    PtySessions,
    PtyWriterLease,
    PaneAttachments,
    TerminalMouse,
    EnvironmentCompatibility,
    RawExecution,
    GracefulShutdown,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientHello {
    pub minimum_version: ProtocolVersion,
    pub maximum_version: ProtocolVersion,
    pub client_id: ClientId,
    pub client_name: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonHello {
    pub selected_version: ProtocolVersion,
    pub daemon_instance_id: DaemonInstanceId,
    pub boot_id: String,
    pub capabilities: Vec<Capability>,
    pub limits: ProtocolLimits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolLimits {
    pub maximum_frame_bytes: u32,
    pub maximum_snapshot_bytes: u32,
    pub maximum_pending_requests: u16,
    pub maximum_queue_depth: u16,
    pub maximum_terminal_rows: u16,
    pub maximum_terminal_columns: u16,
    pub maximum_clients: u16,
    pub maximum_pty_sessions: u16,
    pub maximum_scrollback_lines: u32,
    pub maximum_utility_output_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceIdentity {
    pub canonical_source: String,
    pub canonical_build: String,
    pub identity_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityIdentityAuthority {
    BackendHandshake,
    BitBakeDatastore,
    BitBakeVersionProbe,
    ConfiguredLayerMetadata,
    ExecutableProbe,
    InitializedEnvironment,
    ProtocolNegotiation,
    ReleaseMetadata,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CompatibilityDetected<T> {
    Unknown,
    Detected {
        value: T,
        authority: CompatibilityIdentityAuthority,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityReleaseIdentity {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityDistroIdentity {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilitySourceRootIdentity {
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityLayerSeriesIdentity {
    pub layer: String,
    pub root: String,
    pub compatible_series: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityToolIdentity {
    pub id: String,
    pub executable: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityBackendIdentity {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityProtocolIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityEnvironmentIdentity {
    pub build_directory: CompatibilityDetected<String>,
    pub source_roots: CompatibilityDetected<Vec<CompatibilitySourceRootIdentity>>,
    pub bitbake_version: CompatibilityDetected<String>,
    pub oe_core: CompatibilityDetected<CompatibilityReleaseIdentity>,
    pub poky: CompatibilityDetected<CompatibilityReleaseIdentity>,
    pub distro: CompatibilityDetected<CompatibilityDistroIdentity>,
    pub machine: CompatibilityDetected<String>,
    pub layer_series: CompatibilityDetected<Vec<CompatibilityLayerSeriesIdentity>>,
    pub available_tools: CompatibilityDetected<Vec<CompatibilityToolIdentity>>,
    pub backend: CompatibilityDetected<CompatibilityBackendIdentity>,
    pub protocol: CompatibilityDetected<CompatibilityProtocolIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityReasonData {
    pub code: String,
    pub message: String,
    pub requirement: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityEvidenceKind {
    DirectProbe,
    BackendNegotiation,
    ProtocolNegotiation,
    Metadata,
    ExecutableIdentity,
    ReleaseVersionFallback,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityEvidenceOutcome {
    Positive,
    Negative,
    Inconclusive,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityEvidenceData {
    pub kind: CompatibilityEvidenceKind,
    pub outcome: CompatibilityEvidenceOutcome,
    pub subject: String,
    pub detail: String,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CompatibilityStateData {
    Available,
    AvailableWithLimitations {
        reason: CompatibilityReasonData,
        limitations: Vec<String>,
    },
    Unavailable {
        reason: CompatibilityReasonData,
    },
    Unknown {
        reason: CompatibilityReasonData,
    },
    Unsupported {
        reason: CompatibilityReasonData,
    },
    #[serde(other)]
    UnknownWireState,
}

impl CompatibilityStateData {
    pub const fn is_enabled(&self) -> bool {
        matches!(
            self,
            Self::Available | Self::AvailableWithLimitations { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityImplementationData {
    pub id: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityCapabilityData {
    pub id: String,
    pub state: CompatibilityStateData,
    pub evidence: Vec<CompatibilityEvidenceData>,
    pub implementation: Option<CompatibilityImplementationData>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilitySnapshotData {
    pub schema_version: u16,
    pub generation: u64,
    pub environment: CompatibilityEnvironmentIdentity,
    pub capabilities: Vec<CompatibilityCapabilityData>,
}

impl CompatibilitySnapshotData {
    pub fn validate(&self) -> Result<(), CompatibilityProtocolError> {
        if self.schema_version != COMPATIBILITY_SCHEMA_VERSION {
            return Err(CompatibilityProtocolError::UnsupportedSchema(
                self.schema_version,
            ));
        }
        if self.generation == 0 {
            return Err(CompatibilityProtocolError::InvalidGeneration);
        }
        validate_environment(&self.environment)?;
        if self.capabilities.len() > MAX_COMPATIBILITY_CAPABILITIES {
            return Err(CompatibilityProtocolError::Oversized("capabilities"));
        }
        let mut ids = std::collections::BTreeSet::new();
        for capability in &self.capabilities {
            if !valid_id(&capability.id) {
                return Err(CompatibilityProtocolError::InvalidText("capability id"));
            }
            if !ids.insert(&capability.id) {
                return Err(CompatibilityProtocolError::DuplicateCapability(
                    capability.id.clone(),
                ));
            }
            validate_capability(capability)?;
        }
        Ok(())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CompatibilityProtocolError {
    #[error("unsupported compatibility schema version: {0}")]
    UnsupportedSchema(u16),
    #[error("compatibility generation must be non-zero")]
    InvalidGeneration,
    #[error("oversized compatibility field: {0}")]
    Oversized(&'static str),
    #[error("invalid compatibility text field: {0}")]
    InvalidText(&'static str),
    #[error("invalid compatibility path field: {0}")]
    InvalidPath(&'static str),
    #[error("duplicate compatibility capability: {0}")]
    DuplicateCapability(String),
    #[error("compatibility capability evidence does not support its state: {0}")]
    EvidenceMismatch(String),
    #[error("unknown compatibility identity authority cannot establish detected identity")]
    UnknownIdentityAuthority,
}

fn validate_environment(
    environment: &CompatibilityEnvironmentIdentity,
) -> Result<(), CompatibilityProtocolError> {
    validate_detected(&environment.build_directory, |path| {
        valid_path(path, "build directory")
    })?;
    validate_detected(&environment.bitbake_version, |value| {
        valid_text(value, "BitBake version")
    })?;
    validate_detected(&environment.machine, |value| {
        valid_id(value)
            .then_some(())
            .ok_or(CompatibilityProtocolError::InvalidText("machine"))
    })?;
    for release in [&environment.oe_core, &environment.poky] {
        validate_detected(release, |release| {
            if release.name.is_none() && release.version.is_none() {
                return Err(CompatibilityProtocolError::InvalidText("release"));
            }
            for value in [release.name.as_deref(), release.version.as_deref()]
                .into_iter()
                .flatten()
            {
                valid_text(value, "release")?;
            }
            Ok(())
        })?;
    }
    validate_detected(&environment.distro, |distro| {
        if !valid_id(&distro.name) {
            return Err(CompatibilityProtocolError::InvalidText("distro"));
        }
        if let Some(version) = &distro.version {
            valid_text(version, "distro version")?;
        }
        Ok(())
    })?;
    validate_detected(&environment.source_roots, |roots| {
        valid_collection(roots, "source roots")?;
        for root in roots {
            valid_text(&root.kind, "source root kind")?;
            valid_path(&root.path, "source root")?;
        }
        Ok(())
    })?;
    validate_detected(&environment.layer_series, |layers| {
        valid_collection(layers, "layer series")?;
        for layer in layers {
            if !valid_id(&layer.layer) {
                return Err(CompatibilityProtocolError::InvalidText("layer"));
            }
            valid_path(&layer.root, "layer root")?;
            valid_collection(&layer.compatible_series, "compatible series")?;
            if layer
                .compatible_series
                .iter()
                .any(|series| !valid_id(series))
            {
                return Err(CompatibilityProtocolError::InvalidText("compatible series"));
            }
        }
        Ok(())
    })?;
    validate_detected(&environment.available_tools, |tools| {
        valid_collection(tools, "available tools")?;
        for tool in tools {
            if !valid_id(&tool.id) {
                return Err(CompatibilityProtocolError::InvalidText("tool id"));
            }
            valid_path(&tool.executable, "tool executable")?;
            if let Some(version) = &tool.version {
                valid_text(version, "tool version")?;
            }
        }
        Ok(())
    })?;
    validate_detected(&environment.backend, |backend| {
        if !valid_id(&backend.name) {
            return Err(CompatibilityProtocolError::InvalidText("backend"));
        }
        if let Some(version) = &backend.version {
            valid_text(version, "backend version")?;
        }
        Ok(())
    })?;
    validate_detected(&environment.protocol, |protocol| {
        if !valid_id(&protocol.name) {
            return Err(CompatibilityProtocolError::InvalidText("protocol"));
        }
        valid_text(&protocol.version, "protocol version")
    })?;
    Ok(())
}

fn validate_detected<T>(
    value: &CompatibilityDetected<T>,
    validate: impl FnOnce(&T) -> Result<(), CompatibilityProtocolError>,
) -> Result<(), CompatibilityProtocolError> {
    match value {
        CompatibilityDetected::Unknown => Ok(()),
        CompatibilityDetected::Detected { value, authority } => {
            if *authority == CompatibilityIdentityAuthority::Unknown {
                return Err(CompatibilityProtocolError::UnknownIdentityAuthority);
            }
            validate(value)
        }
    }
}

fn validate_capability(
    capability: &CompatibilityCapabilityData,
) -> Result<(), CompatibilityProtocolError> {
    if capability.evidence.len() > MAX_COMPATIBILITY_EVIDENCE {
        return Err(CompatibilityProtocolError::Oversized("capability evidence"));
    }
    for evidence in &capability.evidence {
        valid_text(&evidence.subject, "evidence subject")?;
        valid_text(&evidence.detail, "evidence detail")?;
        if evidence.argv.len() > MAX_COMPATIBILITY_ARGV {
            return Err(CompatibilityProtocolError::Oversized("evidence argv"));
        }
        for argument in &evidence.argv {
            valid_text(argument, "evidence argv")?;
        }
    }
    if let Some(implementation) = &capability.implementation
        && (!valid_id(&implementation.id) || !valid_id(&implementation.kind))
    {
        return Err(CompatibilityProtocolError::InvalidText("implementation"));
    }
    let has_positive = capability.evidence.iter().any(|evidence| {
        evidence.outcome == CompatibilityEvidenceOutcome::Positive
            && evidence.kind != CompatibilityEvidenceKind::Unknown
    });
    let has_negative = capability.evidence.iter().any(|evidence| {
        evidence.outcome == CompatibilityEvidenceOutcome::Negative
            && evidence.kind != CompatibilityEvidenceKind::Unknown
    });
    match &capability.state {
        CompatibilityStateData::Available => {
            if !has_positive || capability.implementation.is_none() {
                return Err(CompatibilityProtocolError::EvidenceMismatch(
                    capability.id.clone(),
                ));
            }
        }
        CompatibilityStateData::AvailableWithLimitations {
            reason,
            limitations,
        } => {
            validate_reason(reason)?;
            valid_collection(limitations, "limitations")?;
            for limitation in limitations {
                valid_text(limitation, "limitation")?;
            }
            if !has_positive || capability.implementation.is_none() {
                return Err(CompatibilityProtocolError::EvidenceMismatch(
                    capability.id.clone(),
                ));
            }
        }
        CompatibilityStateData::Unavailable { reason } => {
            validate_reason(reason)?;
            if !has_negative || capability.implementation.is_some() {
                return Err(CompatibilityProtocolError::EvidenceMismatch(
                    capability.id.clone(),
                ));
            }
        }
        CompatibilityStateData::Unknown { reason }
        | CompatibilityStateData::Unsupported { reason } => {
            validate_reason(reason)?;
            if capability.implementation.is_some() {
                return Err(CompatibilityProtocolError::EvidenceMismatch(
                    capability.id.clone(),
                ));
            }
        }
        CompatibilityStateData::UnknownWireState => {
            if capability.implementation.is_some() {
                return Err(CompatibilityProtocolError::EvidenceMismatch(
                    capability.id.clone(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_reason(reason: &CompatibilityReasonData) -> Result<(), CompatibilityProtocolError> {
    if !valid_id(&reason.code) {
        return Err(CompatibilityProtocolError::InvalidText("reason code"));
    }
    valid_text(&reason.message, "reason message")?;
    if let Some(requirement) = &reason.requirement {
        valid_text(requirement, "reason requirement")?;
    }
    Ok(())
}

fn valid_collection<T>(
    values: &[T],
    field: &'static str,
) -> Result<(), CompatibilityProtocolError> {
    if values.is_empty() || values.len() > MAX_COMPATIBILITY_ITEMS {
        return Err(CompatibilityProtocolError::Oversized(field));
    }
    Ok(())
}

fn valid_text(value: &str, field: &'static str) -> Result<(), CompatibilityProtocolError> {
    if value.is_empty()
        || value.len() > MAX_COMPATIBILITY_TEXT_BYTES
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(CompatibilityProtocolError::InvalidText(field));
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_path(value: &str, field: &'static str) -> Result<(), CompatibilityProtocolError> {
    let path = Path::new(value);
    if !path.is_absolute()
        || path == Path::new("/")
        || value.len() > MAX_COMPATIBILITY_TEXT_BYTES
        || !path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
    {
        return Err(CompatibilityProtocolError::InvalidPath(field));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeCursor {
    pub daemon_instance_id: DaemonInstanceId,
    pub last_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub state: bool,
    pub jobs: bool,
    pub logs: bool,
    pub pty_sessions: Vec<PtySessionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum ClientMessage {
    Hello(ClientHello),
    Attach {
        workspace: Option<WorkspaceIdentity>,
        subscription: Subscription,
        resume: Option<ResumeCursor>,
    },
    Subscribe {
        subscription: Subscription,
    },
    Unsubscribe {
        subscription: Subscription,
    },
    Command(CommandRequest),
    PtyInput(PtyInput),
    PtyResize(PtyResize),
    Layout {
        event: ClientLayoutEvent,
    },
    Mouse {
        event: ServerMouseEvent,
    },
    Detach,
    Pong {
        nonce: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRequest {
    pub request_id: RequestId,
    pub expected_generation: Option<u64>,
    pub command: DaemonCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonCommand {
    StartBuild {
        targets: Vec<String>,
        task: Option<String>,
        force: bool,
    },
    CancelJob {
        job_id: JobId,
    },
    StartRaw {
        request: RawExecutionRequestData,
    },
    CancelRaw {
        request_id: String,
    },
    StartDevtool {
        operation: DaemonDevtoolOperation,
        build_directory: String,
    },
    StartSdk {
        session_id: u64,
        operation: DaemonSdkOperation,
        context: DaemonSdkContext,
    },
    CancelSdk {
        session_id: u64,
    },
    StartQemu {
        session_id: u64,
        request: DaemonQemuRequest,
        build_directory: String,
        executable: String,
    },
    CancelQemu {
        session_id: u64,
    },
    StartWicCreate {
        session_id: u64,
        request: DaemonWicCreateRequest,
        build_directory: String,
        executable: String,
    },
    StartWicWrite {
        session_id: u64,
        executable: String,
        image_path: String,
        device_path: String,
        device_major_minor: String,
        device_size_bytes: u64,
        device_model: Option<String>,
        device_serial: Option<String>,
        device_transport: Option<String>,
        build_directory: String,
    },
    CancelWic {
        session_id: u64,
    },
    StartTestSession {
        session_id: u64,
        request: DaemonTestSelftestRequest,
        build_directory: String,
        path_directories: Vec<String>,
    },
    CancelTestSession {
        session_id: u64,
    },
    ImportTestResults {
        generation: u64,
        roots: Vec<String>,
    },
    CompareTestResults {
        generation: u64,
        baseline_identity: String,
        candidate_identity: String,
    },
    ExportTestJunit {
        generation: u64,
        result_identity: String,
        destination: String,
    },
    InspectTestResultTool {
        path_directories: Vec<String>,
    },
    InspectQaCapability {
        request: DaemonQaCapabilityRequest,
    },
    StartQaLayerCheck {
        session_id: u64,
        operation_id: u64,
        check_id: String,
        layer_name: String,
        layer_root: String,
        executable: String,
        arguments: Vec<String>,
        report_roots: Vec<String>,
    },
    CancelQaLayerCheck {
        session_id: u64,
    },
    StartQaReportScan {
        generation: u64,
        build_directory: String,
        paths: Vec<String>,
    },
    CancelQaReportScan {
        generation: u64,
    },
    StartSecurityReportScan {
        generation: u64,
        paths: Vec<String>,
    },
    CancelSecurityReportScan {
        generation: u64,
    },
    StartSecurityPackageMap {
        session_id: u64,
        executable: String,
        arguments: Vec<String>,
        report_roots: Vec<String>,
    },
    CancelSecurityPackageMap {
        session_id: u64,
    },
    InspectMaintenanceCapability {
        request: u64,
        build_directory: String,
        sstate_directory: Option<String>,
        tmp_directory: Option<String>,
        stamps_directories: Vec<String>,
        executable_search_path: Vec<String>,
    },
    StartMaintenanceSstateReadiness {
        session_id: u64,
        capability_request: u64,
        operation_id: u64,
        build_directory: String,
        sstate_directory: Option<String>,
        tmp_directory: Option<String>,
        stamps_directories: Vec<String>,
        executable_search_path: Vec<String>,
        targets: Vec<String>,
        mode: String,
        output: Option<String>,
        log: Option<String>,
        timeout_seconds: u64,
    },
    CancelMaintenance {
        session_id: u64,
    },
    StartMaintenanceExternal {
        session_id: u64,
        executable: String,
        expected_name: String,
        arguments: Vec<String>,
        current_directory: String,
    },
    InspectMaintenanceServices {
        request: u64,
        build_directory: String,
        prserv_host: Option<String>,
        hashserve: Option<String>,
        hashserve_upstream: Option<String>,
        signature_handler: Option<String>,
        executable_search_path: Vec<String>,
        process_root: String,
    },
    BitBakeLifecycle {
        operation: BitBakeOperation,
        confirmation: Option<ConfirmationLease>,
    },
    CreatePty {
        name: String,
        kind: PtyKind,
        cwd: String,
        command: PtyCommand,
        dimensions: TerminalDimensions,
    },
    RenamePty {
        session_id: PtySessionId,
        name: String,
    },
    TerminatePty {
        session_id: PtySessionId,
        force: bool,
        confirmation: Option<ConfirmationLease>,
    },
    TakePtyControl {
        session_id: PtySessionId,
        expected_epoch: u64,
    },
    ReleasePtyControl {
        session_id: PtySessionId,
        expected_epoch: u64,
    },
    PrepareShutdown,
    ConfirmShutdown {
        confirmation: ConfirmationLease,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonDevtoolOperation {
    Modify { recipe: String },
    UpdateRecipe { recipe: String },
    Finish { recipe: String, destination: String },
    DeployTarget { recipe: String, target: String },
    UndeployTarget { recipe: String, target: String },
    Reset { recipe: String },
    Upgrade { recipe: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSdkContext {
    pub build_directory: String,
    pub sdk_deploy_root: String,
    pub workspace_roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSdkArtifactIdentity {
    pub path: String,
    pub size_bytes: u64,
    pub modified_unix_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonSdkNativeMode {
    FindSysroot,
    RunNative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonSdkOperation {
    Publish {
        executable: String,
        artifact: DaemonSdkArtifactIdentity,
        destination: String,
    },
    Native {
        executable: String,
        mode: DaemonSdkNativeMode,
        extracted_root: Option<String>,
        recipe: String,
        tool: Option<String>,
        arguments: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonQemuRequest {
    pub machine: String,
    pub image_machine: String,
    pub image: String,
    pub image_path: String,
    pub artifact_kind: String,
    pub kernel: Option<String>,
    pub rootfs: Option<String>,
    pub networking: String,
    pub display: String,
    pub serial: String,
    pub memory_mib: u32,
    pub extra_arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonWicCreateRequest {
    pub machine: String,
    pub image: String,
    pub kickstart_name: String,
    pub kickstart_path: Option<String>,
    pub output_directory: String,
    pub generate_bmap: bool,
    pub compression: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestSelftestRequest {
    pub executable: String,
    pub family: String,
    pub selector: Option<String>,
    pub parallelism: u16,
    pub verbose: bool,
    pub skip_network: bool,
}

pub const MAX_TEST_RESULT_RECORDS: usize = 4096;
pub const MAX_TEST_RESULT_LIMITATIONS: usize = 256;
pub const MAX_QA_RECORDS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestResultRecord {
    pub identity: String,
    pub outcome: String,
    pub duration_ms: Option<u64>,
    pub log_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonQaSnapshot {
    pub generation: u64,
    pub capability: String,
    pub task_bindings: Vec<String>,
    pub reports: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonQaCapabilityInput {
    pub generation: u64,
    pub build_directory: String,
    pub source_directory: Option<String>,
    pub layer_directories: Vec<String>,
    pub recipe_names: Vec<String>,
    pub report_roots: Vec<String>,
    pub selected_recipe_name: String,
    pub selected_recipe_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonQaCapabilityRequest {
    pub request_id: RequestId,
    pub input: DaemonQaCapabilityInput,
}

impl DaemonQaCapabilityInput {
    pub fn bounded(mut self) -> Self {
        self.layer_directories.truncate(MAX_QA_RECORDS);
        self.recipe_names.truncate(MAX_QA_RECORDS);
        self.report_roots.truncate(MAX_QA_RECORDS);
        self
    }
}

impl DaemonQaSnapshot {
    pub fn bounded(mut self) -> Self {
        self.task_bindings.truncate(MAX_QA_RECORDS);
        self.reports.truncate(MAX_QA_RECORDS);
        self.limitations.truncate(MAX_TEST_RESULT_LIMITATIONS);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestResultSnapshot {
    pub generation: u64,
    pub records: Vec<DaemonTestResultRecord>,
    pub limitations: Vec<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestComparisonDiff {
    pub generation: u64,
    pub baseline: String,
    pub candidate: String,
    pub transitions: Vec<DaemonTestComparisonTransition>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonTestResultToolCapability {
    NotInspected,
    Missing,
    Available { executable: String },
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestComparisonTransition {
    pub identity: String,
    pub baseline: Option<String>,
    pub candidate: Option<String>,
    pub category: String,
}

impl DaemonTestComparisonDiff {
    pub fn bounded(mut self) -> Self {
        self.transitions.truncate(MAX_TEST_RESULT_RECORDS);
        self.limitations.truncate(MAX_TEST_RESULT_LIMITATIONS);
        self
    }
}

#[cfg(test)]
mod daemon_test_snapshot_tests {
    use super::*;
    #[test]
    fn daemon_test_snapshot_is_bounded_and_round_trips() {
        let snapshot = DaemonTestResultSnapshot {
            generation: 4,
            records: (0..(MAX_TEST_RESULT_RECORDS + 2))
                .map(|index| DaemonTestResultRecord {
                    identity: index.to_string(),
                    outcome: "pass".into(),
                    duration_ms: None,
                    log_path: None,
                })
                .collect(),
            limitations: (0..(MAX_TEST_RESULT_LIMITATIONS + 2))
                .map(|index| index.to_string())
                .collect(),
            complete: true,
        }
        .bounded();
        assert_eq!(snapshot.records.len(), MAX_TEST_RESULT_RECORDS);
        assert_eq!(snapshot.limitations.len(), MAX_TEST_RESULT_LIMITATIONS);
        let encoded = serde_json::to_vec(&snapshot).unwrap();
        let decoded: DaemonTestResultSnapshot = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, snapshot);
    }

    #[test]
    fn daemon_test_compare_diff_is_bounded_and_round_trips() {
        let diff = DaemonTestComparisonDiff {
            generation: 2,
            baseline: "a".into(),
            candidate: "b".into(),
            transitions: Vec::new(),
            limitations: vec!["limited".into()],
        }
        .bounded();
        let bytes = serde_json::to_vec(&diff).unwrap();
        assert_eq!(
            serde_json::from_slice::<DaemonTestComparisonDiff>(&bytes).unwrap(),
            diff
        );
    }

    #[test]
    fn daemon_qa_snapshot_is_bounded() {
        let snapshot = DaemonQaSnapshot {
            generation: 1,
            capability: "available".into(),
            task_bindings: (0..MAX_QA_RECORDS + 1).map(|i| i.to_string()).collect(),
            reports: (0..MAX_QA_RECORDS + 1).map(|i| i.to_string()).collect(),
            limitations: Vec::new(),
        }
        .bounded();
        assert_eq!(snapshot.task_bindings.len(), MAX_QA_RECORDS);
        assert_eq!(snapshot.reports.len(), MAX_QA_RECORDS);
    }

    #[test]
    fn daemon_qa_input_is_bounded() {
        let input = DaemonQaCapabilityInput {
            generation: 1,
            build_directory: "/build".into(),
            source_directory: None,
            layer_directories: (0..MAX_QA_RECORDS + 1).map(|i| i.to_string()).collect(),
            recipe_names: Vec::new(),
            report_roots: Vec::new(),
            selected_recipe_name: "recipe".into(),
            selected_recipe_file: "/build/recipe.bb".into(),
        }
        .bounded();
        assert_eq!(input.layer_directories.len(), MAX_QA_RECORDS);
    }
}

impl DaemonTestResultSnapshot {
    pub fn bounded(mut self) -> Self {
        self.records.truncate(MAX_TEST_RESULT_RECORDS);
        self.limitations.truncate(MAX_TEST_RESULT_LIMITATIONS);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BitBakeOperation {
    Connect,
    Disconnect,
    Start,
    Stop,
    Restart,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmationLease {
    pub token: [u8; 32],
    pub preview_hash: [u8; 32],
    pub expires_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyCommand {
    pub program: String,
    pub arguments: Vec<String>,
    pub environment_profile_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PtyKind {
    BuildShell,
    SourceShell,
    LayerShell,
    RecipeShell,
    DevtoolShell,
    Devshell,
    Menuconfig,
    SdkShell,
    NativeShell,
    Utility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalDimensions {
    pub columns: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyInput {
    pub request_id: RequestId,
    pub session_id: PtySessionId,
    pub writer_epoch: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyResize {
    pub request_id: RequestId,
    pub session_id: PtySessionId,
    pub writer_epoch: u64,
    pub dimensions: TerminalDimensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientLayoutEvent {
    AttachSession {
        pane_id: PaneId,
        session_id: PtySessionId,
    },
    DetachSession {
        pane_id: PaneId,
        session_id: PtySessionId,
    },
    FocusWriter {
        pane_id: PaneId,
        session_id: PtySessionId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseEventKind {
    Down,
    Up,
    Drag,
    ScrollUp,
    ScrollDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMouseEvent {
    pub session_id: PtySessionId,
    pub writer_epoch: u64,
    pub kind: MouseEventKind,
    pub button: u8,
    pub column: u16,
    pub row: u16,
    pub modifiers: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Hello(DaemonHello),
    Attached {
        snapshot: DaemonSnapshot,
        replayed_through: u64,
    },
    Snapshot(DaemonSnapshot),
    Event(SequencedEvent),
    CommandResult(CommandResult),
    ResyncRequired {
        reason: String,
        current_sequence: u64,
    },
    Error(ProtocolFailure),
    Ping {
        nonce: u64,
        deadline_unix_ms: u64,
    },
    Detaching,
    ShuttingDown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSnapshot {
    pub daemon_instance_id: DaemonInstanceId,
    pub sequence: u64,
    pub generation: u64,
    pub workspace: Option<WorkspaceIdentity>,
    pub project_profile: ProjectProfileSummary,
    pub bitbake: BitBakeState,
    #[serde(default)]
    pub compatibility: Option<CompatibilitySnapshotData>,
    pub jobs: Vec<JobSummary>,
    #[serde(default)]
    pub raw_executions: Vec<RawExecutionSnapshotData>,
    pub pty_sessions: Vec<PtySessionSummary>,
    #[serde(default)]
    pub pty_screens: Vec<PtyScreenSnapshot>,
    pub clients: Vec<ClientSummary>,
    pub recent_logs: Vec<LogRecord>,
    #[serde(default)]
    pub build_events: Vec<DaemonBuildEvent>,
    pub recovery_warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonBuildEvent {
    Reset {
        targets: Vec<String>,
    },
    Workspace {
        data: WorkspaceData,
    },
    Started,
    ParseProgress {
        current: Option<u64>,
        total: Option<u64>,
    },
    TaskQueued {
        recipe: String,
        task: String,
        worker: Option<String>,
        stats: Option<TaskStatsData>,
    },
    TaskStarted {
        recipe: String,
        task: String,
        pid: Option<u32>,
        worker: Option<String>,
        log_path: Option<String>,
        stats: Option<TaskStatsData>,
    },
    TaskProgress {
        recipe: String,
        task: String,
        progress: Option<u8>,
    },
    TaskCompleted {
        recipe: String,
        task: String,
        success: bool,
    },
    Completed {
        success: bool,
        exit_code: Option<i32>,
    },
    CommandFailed {
        code: String,
        message: String,
    },
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequencedEvent {
    pub sequence: u64,
    pub generation: u64,
    pub event: DaemonEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonEvent {
    BitBakeChanged(BitBakeState),
    CompatibilityChanged(Box<CompatibilitySnapshotData>),
    JobChanged(JobSummary),
    JobRemoved {
        job_id: JobId,
    },
    RawExecutionChanged(Box<RawExecutionSnapshotData>),
    RawExecutionRemoved {
        request_id: String,
    },
    PtyChanged(PtySessionSummary),
    PtyOutput {
        session_id: PtySessionId,
        bytes: Vec<u8>,
    },
    PtyScreen(PtyScreenSnapshot),
    ClientChanged(ClientSummary),
    ClientRemoved {
        client_id: ClientId,
    },
    RecoveryWarning {
        message: String,
    },
    Log(LogRecord),
    Build(DaemonBuildEvent),
    TestResults(DaemonTestResultSnapshot),
    TestComparison(DaemonTestComparisonDiff),
    TestResultTool(DaemonTestResultToolCapability),
    QaSnapshot(DaemonQaSnapshot),
    QaCapability(DaemonQaSnapshot),
    SecuritySnapshot(DaemonSecuritySnapshot),
    MaintenanceSnapshot(DaemonMaintenanceSnapshot),
    Telemetry(DaemonTelemetry),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSecuritySnapshot {
    pub generation: u64,
    pub reports: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonMaintenanceSnapshot {
    pub request: u64,
    pub tools: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProjectProfileSummary {
    NotLoaded,
    Absent,
    Loaded { schema_version: u32 },
    Invalid { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Disconnected,
    Connecting,
    Running,
    Stopping,
    Exited,
    Failed,
    Lost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitBakeState {
    pub lifecycle: LifecycleState,
    pub version: Option<String>,
    pub capabilities: Vec<BitBakeCapability>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BitBakeCapability {
    WorkspaceInspection,
    RecipeInventory,
    LayerInventory,
    BuildControl,
    Cancellation,
    ServerRestart,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSummary {
    pub id: JobId,
    pub kind: JobKind,
    pub label: String,
    pub lifecycle: LifecycleState,
    pub progress_current: Option<u64>,
    pub progress_total: Option<u64>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    BitBakeBuild,
    Devtool,
    Qemu,
    Wic,
    Sdk,
    Testing,
    Qa,
    Security,
    Maintenance,
    Utility,
    Raw,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtySessionSummary {
    pub id: PtySessionId,
    pub name: String,
    pub kind: PtyKind,
    pub cwd: String,
    pub lifecycle: LifecycleState,
    pub dimensions: TerminalDimensions,
    pub writer: Option<ClientId>,
    pub writer_epoch: u64,
    pub viewers: u16,
    pub exit_code: Option<i32>,
    pub restartable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientSummary {
    pub id: ClientId,
    pub name: String,
    pub attached_unix_ms: u64,
    pub last_seen_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonRecoveryState {
    CleanStart,
    Recovering,
    Recovered,
    Degraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTelemetry {
    pub uptime_seconds: u64,
    pub bitbake: LifecycleState,
    pub connected_clients: u16,
    pub active_jobs: u16,
    pub pty_sessions: u16,
    pub queue_depth: u16,
    pub memory_bytes: Option<u64>,
    pub recovery: DaemonRecoveryState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyScreenSnapshot {
    pub session_id: PtySessionId,
    pub dimensions: TerminalDimensions,
    pub cursor_column: u16,
    pub cursor_row: u16,
    pub rows: Vec<String>,
    pub scrollback_lines: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogRecord {
    pub source: String,
    pub severity: LogSeverity,
    pub message: String,
    pub unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonSnapshotLimits {
    pub retained_events: usize,
    pub recent_logs: usize,
    pub snapshot_bytes: usize,
}

impl Default for DaemonSnapshotLimits {
    fn default() -> Self {
        Self {
            retained_events: 4_096,
            // Keep attach/status snapshots responsive during high-volume
            // BitBake output. The journal remains sequence/event bounded;
            // clients can still follow incremental log events after attach.
            recent_logs: 512,
            snapshot_bytes: MAX_FRAME_BYTES,
        }
    }
}

impl DaemonSnapshotLimits {
    fn validate(self) -> Result<Self, DaemonSnapshotError> {
        if self.retained_events == 0 || self.retained_events > MAX_RETAINED_EVENTS {
            return Err(DaemonSnapshotError::InvalidLimit("retained events"));
        }
        if self.recent_logs == 0 || self.recent_logs > MAX_SNAPSHOT_LOGS {
            return Err(DaemonSnapshotError::InvalidLimit("recent logs"));
        }
        if self.snapshot_bytes == 0 || self.snapshot_bytes > MAX_FRAME_BYTES {
            return Err(DaemonSnapshotError::InvalidLimit("snapshot bytes"));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonSnapshotSync {
    Replace {
        snapshot: Box<DaemonSnapshot>,
        reason: SnapshotReplacementReason,
    },
    Replay {
        events: Vec<SequencedEvent>,
        replayed_through: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotReplacementReason {
    InitialAttach,
    DaemonInstanceChanged,
    HistoryExpired,
    CursorAhead,
}

/// Single-owner snapshot/event journal. Calling `synchronize` while holding the
/// daemon state's lock establishes the snapshot/replay watermark before a
/// client is added to the live subscriber set.
#[derive(Debug, Clone)]
pub struct DaemonSnapshotJournal {
    snapshot: DaemonSnapshot,
    events: VecDeque<SequencedEvent>,
    limits: DaemonSnapshotLimits,
}

impl DaemonSnapshotJournal {
    pub fn new(
        snapshot: DaemonSnapshot,
        limits: DaemonSnapshotLimits,
    ) -> Result<Self, DaemonSnapshotError> {
        let limits = limits.validate()?;
        if let Some(compatibility) = &snapshot.compatibility {
            compatibility.validate()?;
        }
        if snapshot.raw_executions.len() > MAX_RAW_EXECUTION_REQUESTS {
            return Err(DaemonSnapshotError::TooManyRawExecutions);
        }
        let mut raw_request_ids = std::collections::BTreeSet::new();
        for execution in &snapshot.raw_executions {
            execution.validate()?;
            if !raw_request_ids.insert(&execution.request.request_id) {
                return Err(DaemonSnapshotError::RawExecution(
                    RawExecutionProtocolError::DuplicateRequest,
                ));
            }
        }
        for screen in &snapshot.pty_screens {
            validate_pty_screen(screen)?;
        }
        ensure_snapshot_bound(&snapshot, limits.snapshot_bytes)?;
        Ok(Self {
            snapshot,
            events: VecDeque::new(),
            limits,
        })
    }

    pub fn snapshot(&self) -> &DaemonSnapshot {
        &self.snapshot
    }

    pub fn publish(&mut self, event: DaemonEvent) -> Result<SequencedEvent, DaemonSnapshotError> {
        let sequence = self
            .snapshot
            .sequence
            .checked_add(1)
            .ok_or(DaemonSnapshotError::SequenceExhausted)?;
        let generation = self
            .snapshot
            .generation
            .checked_add(1)
            .ok_or(DaemonSnapshotError::GenerationExhausted)?;
        let sequenced = SequencedEvent {
            sequence,
            generation,
            event,
        };
        let event_bytes = serde_json::to_vec(&sequenced)?.len();
        if event_bytes > MAX_FRAME_BYTES {
            return Err(DaemonSnapshotError::EventTooLarge {
                actual: event_bytes,
                maximum: MAX_FRAME_BYTES,
            });
        }
        let mut candidate = self.snapshot.clone();
        apply_sequenced_event(&mut candidate, &sequenced)?;
        while candidate.recent_logs.len() > self.limits.recent_logs {
            candidate.recent_logs.remove(0);
        }
        while serde_json::to_vec(&candidate)?.len() > self.limits.snapshot_bytes
            && !candidate.recent_logs.is_empty()
        {
            candidate.recent_logs.remove(0);
        }
        while serde_json::to_vec(&candidate)?.len() > self.limits.snapshot_bytes
            && candidate.pty_screens.len() > 1
        {
            candidate.pty_screens.remove(0);
        }
        ensure_snapshot_bound(&candidate, self.limits.snapshot_bytes)?;
        self.snapshot = candidate;
        self.events.push_back(sequenced.clone());
        while self.events.len() > self.limits.retained_events {
            self.events.pop_front();
        }
        Ok(sequenced)
    }

    pub fn synchronize(&self, resume: Option<ResumeCursor>) -> DaemonSnapshotSync {
        let Some(cursor) = resume else {
            return DaemonSnapshotSync::Replace {
                snapshot: Box::new(self.snapshot.clone()),
                reason: SnapshotReplacementReason::InitialAttach,
            };
        };
        if cursor.daemon_instance_id != self.snapshot.daemon_instance_id {
            return DaemonSnapshotSync::Replace {
                snapshot: Box::new(self.snapshot.clone()),
                reason: SnapshotReplacementReason::DaemonInstanceChanged,
            };
        }
        if cursor.last_sequence > self.snapshot.sequence {
            return DaemonSnapshotSync::Replace {
                snapshot: Box::new(self.snapshot.clone()),
                reason: SnapshotReplacementReason::CursorAhead,
            };
        }
        let first_retained = self
            .events
            .front()
            .map(|event| event.sequence)
            .unwrap_or_else(|| self.snapshot.sequence.saturating_add(1));
        if cursor.last_sequence.saturating_add(1) < first_retained {
            return DaemonSnapshotSync::Replace {
                snapshot: Box::new(self.snapshot.clone()),
                reason: SnapshotReplacementReason::HistoryExpired,
            };
        }
        DaemonSnapshotSync::Replay {
            events: self
                .events
                .iter()
                .filter(|event| event.sequence > cursor.last_sequence)
                .cloned()
                .collect(),
            replayed_through: self.snapshot.sequence,
        }
    }
}

pub fn apply_sequenced_event(
    snapshot: &mut DaemonSnapshot,
    sequenced: &SequencedEvent,
) -> Result<(), DaemonSnapshotError> {
    let expected_sequence = snapshot
        .sequence
        .checked_add(1)
        .ok_or(DaemonSnapshotError::SequenceExhausted)?;
    let expected_generation = snapshot
        .generation
        .checked_add(1)
        .ok_or(DaemonSnapshotError::GenerationExhausted)?;
    if sequenced.sequence != expected_sequence || sequenced.generation != expected_generation {
        return Err(DaemonSnapshotError::EventGap {
            expected_sequence,
            actual_sequence: sequenced.sequence,
            expected_generation,
            actual_generation: sequenced.generation,
        });
    }
    match &sequenced.event {
        DaemonEvent::BitBakeChanged(bitbake) => snapshot.bitbake = bitbake.clone(),
        DaemonEvent::CompatibilityChanged(compatibility) => {
            compatibility.validate()?;
            if snapshot
                .compatibility
                .as_ref()
                .is_some_and(|current| current.generation >= compatibility.generation)
            {
                return Err(DaemonSnapshotError::StaleCompatibilityGeneration {
                    current: snapshot
                        .compatibility
                        .as_ref()
                        .map(|current| current.generation)
                        .unwrap_or(0),
                    received: compatibility.generation,
                });
            }
            snapshot.compatibility = Some((**compatibility).clone());
        }
        DaemonEvent::JobChanged(job) => replace_by(&mut snapshot.jobs, job.clone(), |item| item.id),
        DaemonEvent::JobRemoved { job_id } => snapshot.jobs.retain(|job| job.id != *job_id),
        DaemonEvent::RawExecutionChanged(execution) => {
            execution.validate()?;
            let request_id = execution.request.request_id.clone();
            if let Some(current) = snapshot
                .raw_executions
                .iter()
                .find(|current| current.request.request_id == request_id)
                && (execution.sequence <= current.sequence
                    || execution.generation <= current.generation)
            {
                return Err(DaemonSnapshotError::StaleRawExecution {
                    request_id,
                    current_sequence: current.sequence,
                    received_sequence: execution.sequence,
                });
            }
            replace_by(
                &mut snapshot.raw_executions,
                (**execution).clone(),
                |item| item.request.request_id.clone(),
            );
            if snapshot.raw_executions.len() > MAX_RAW_EXECUTION_REQUESTS {
                return Err(DaemonSnapshotError::TooManyRawExecutions);
            }
        }
        DaemonEvent::RawExecutionRemoved { request_id } => {
            validate_raw_identity(request_id, "raw-request:", "request")?;
            snapshot
                .raw_executions
                .retain(|execution| execution.request.request_id != *request_id);
        }
        DaemonEvent::PtyChanged(pty) => {
            replace_by(&mut snapshot.pty_sessions, pty.clone(), |item| item.id);
        }
        DaemonEvent::PtyScreen(screen) => {
            validate_pty_screen(screen)?;
            snapshot
                .pty_screens
                .retain(|item| item.session_id != screen.session_id);
            snapshot.pty_screens.push(screen.clone());
        }
        DaemonEvent::PtyOutput { .. }
        | DaemonEvent::TestResults(_)
        | DaemonEvent::TestComparison(_)
        | DaemonEvent::TestResultTool(_)
        | DaemonEvent::QaSnapshot(_)
        | DaemonEvent::QaCapability(_)
        | DaemonEvent::SecuritySnapshot(_)
        | DaemonEvent::MaintenanceSnapshot(_)
        | DaemonEvent::Telemetry(_)
        | DaemonEvent::Unknown => {}
        DaemonEvent::ClientChanged(client) => {
            replace_by(&mut snapshot.clients, client.clone(), |item| item.id);
        }
        DaemonEvent::ClientRemoved { client_id } => {
            snapshot.clients.retain(|client| client.id != *client_id);
        }
        DaemonEvent::RecoveryWarning { message } => {
            snapshot.recovery_warnings.push(message.clone());
        }
        DaemonEvent::Log(record) => snapshot.recent_logs.push(record.clone()),
        DaemonEvent::Build(event) => apply_build_event(snapshot, event.clone()),
    }
    snapshot.sequence = sequenced.sequence;
    snapshot.generation = sequenced.generation;
    Ok(())
}

fn apply_build_event(snapshot: &mut DaemonSnapshot, event: DaemonBuildEvent) {
    if matches!(event, DaemonBuildEvent::Reset { .. }) {
        snapshot.build_events.clear();
    }

    match &event {
        DaemonBuildEvent::Workspace { .. } => snapshot
            .build_events
            .retain(|item| !matches!(item, DaemonBuildEvent::Workspace { .. })),
        DaemonBuildEvent::ParseProgress { .. } => snapshot
            .build_events
            .retain(|item| !matches!(item, DaemonBuildEvent::ParseProgress { .. })),
        DaemonBuildEvent::TaskQueued { recipe, task, .. }
        | DaemonBuildEvent::TaskStarted { recipe, task, .. } => snapshot.build_events.retain(
            |item| {
                !matches!(item,
                    DaemonBuildEvent::TaskQueued { recipe: old_recipe, task: old_task, .. }
                    | DaemonBuildEvent::TaskStarted { recipe: old_recipe, task: old_task, .. }
                    | DaemonBuildEvent::TaskProgress { recipe: old_recipe, task: old_task, .. }
                    if old_recipe == recipe && old_task == task)
            },
        ),
        DaemonBuildEvent::TaskProgress { recipe, task, .. } => snapshot.build_events.retain(
            |item| {
                !matches!(item, DaemonBuildEvent::TaskProgress { recipe: old_recipe, task: old_task, .. } if old_recipe == recipe && old_task == task)
            },
        ),
        DaemonBuildEvent::TaskCompleted { recipe, task, .. } => snapshot.build_events.retain(
            |item| {
                !matches!(item,
                    DaemonBuildEvent::TaskQueued { recipe: old_recipe, task: old_task, .. }
                    | DaemonBuildEvent::TaskStarted { recipe: old_recipe, task: old_task, .. }
                    | DaemonBuildEvent::TaskProgress { recipe: old_recipe, task: old_task, .. }
                    if old_recipe == recipe && old_task == task)
            },
        ),
        _ => {}
    }
    snapshot.build_events.push(event);
    while snapshot.build_events.len() > MAX_DAEMON_BUILD_EVENTS {
        let removable = snapshot
            .build_events
            .iter()
            .position(|item| matches!(item, DaemonBuildEvent::TaskCompleted { .. }));
        let removable = removable.or_else(|| {
            snapshot.build_events.iter().position(|item| {
                !matches!(
                    item,
                    DaemonBuildEvent::Reset { .. }
                        | DaemonBuildEvent::Workspace { .. }
                        | DaemonBuildEvent::Started
                )
            })
        });
        snapshot.build_events.remove(removable.unwrap_or(0));
    }
}

fn replace_by<T, K: PartialEq>(items: &mut Vec<T>, replacement: T, key: impl Fn(&T) -> K) {
    let replacement_key = key(&replacement);
    if let Some(index) = items.iter().position(|item| key(item) == replacement_key) {
        items[index] = replacement;
    } else {
        items.push(replacement);
    }
}

fn validate_pty_screen(screen: &PtyScreenSnapshot) -> Result<(), DaemonSnapshotError> {
    let dimensions = screen.dimensions;
    let valid_dimensions = dimensions.columns > 0
        && dimensions.columns <= MAX_TERMINAL_COLUMNS
        && dimensions.rows > 0
        && dimensions.rows <= MAX_TERMINAL_ROWS;
    let valid_cursor =
        screen.cursor_column < dimensions.columns && screen.cursor_row < dimensions.rows;
    let valid_rows = screen.rows.len() <= usize::from(dimensions.rows);
    if !valid_dimensions
        || !valid_cursor
        || !valid_rows
        || screen.scrollback_lines as usize > MAX_TERMINAL_SCROLLBACK_LINES
    {
        return Err(DaemonSnapshotError::InvalidPtyScreen(screen.session_id));
    }
    Ok(())
}

fn ensure_snapshot_bound(
    snapshot: &DaemonSnapshot,
    maximum_bytes: usize,
) -> Result<(), DaemonSnapshotError> {
    let encoded = serde_json::to_vec(snapshot)?;
    if encoded.len() > maximum_bytes {
        return Err(DaemonSnapshotError::SnapshotTooLarge {
            actual: encoded.len(),
            maximum: maximum_bytes,
        });
    }
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn client_id(byte: u8) -> ClientId {
        ClientId([byte; 16])
    }

    fn daemon_snapshot_fixture() -> DaemonSnapshot {
        DaemonSnapshot {
            daemon_instance_id: DaemonInstanceId([7; 16]),
            sequence: 0,
            generation: 0,
            workspace: None,
            project_profile: ProjectProfileSummary::Absent,
            bitbake: BitBakeState {
                lifecycle: LifecycleState::Disconnected,
                version: None,
                capabilities: Vec::new(),
                diagnostic: None,
            },
            compatibility: None,
            jobs: Vec::new(),
            raw_executions: Vec::new(),
            pty_sessions: Vec::new(),
            pty_screens: Vec::new(),
            clients: Vec::new(),
            recent_logs: Vec::new(),
            build_events: Vec::new(),
            recovery_warnings: Vec::new(),
        }
    }

    fn compatibility_environment_fixture() -> CompatibilityEnvironmentIdentity {
        CompatibilityEnvironmentIdentity {
            build_directory: CompatibilityDetected::Detected {
                value: "/work/poky/build".into(),
                authority: CompatibilityIdentityAuthority::InitializedEnvironment,
            },
            source_roots: CompatibilityDetected::Unknown,
            bitbake_version: CompatibilityDetected::Detected {
                value: "2.18.0".into(),
                authority: CompatibilityIdentityAuthority::BitBakeVersionProbe,
            },
            oe_core: CompatibilityDetected::Unknown,
            poky: CompatibilityDetected::Unknown,
            distro: CompatibilityDetected::Unknown,
            machine: CompatibilityDetected::Unknown,
            layer_series: CompatibilityDetected::Unknown,
            available_tools: CompatibilityDetected::Unknown,
            backend: CompatibilityDetected::Unknown,
            protocol: CompatibilityDetected::Detected {
                value: CompatibilityProtocolIdentity {
                    name: "yoctui-daemon".into(),
                    version: "1.0".into(),
                },
                authority: CompatibilityIdentityAuthority::ProtocolNegotiation,
            },
        }
    }

    fn compatibility_snapshot_fixture(generation: u64) -> CompatibilitySnapshotData {
        CompatibilitySnapshotData {
            schema_version: COMPATIBILITY_SCHEMA_VERSION,
            generation,
            environment: compatibility_environment_fixture(),
            capabilities: vec![CompatibilityCapabilityData {
                id: "bitbake.getvar".into(),
                state: CompatibilityStateData::Available,
                evidence: vec![CompatibilityEvidenceData {
                    kind: CompatibilityEvidenceKind::DirectProbe,
                    outcome: CompatibilityEvidenceOutcome::Positive,
                    subject: "bitbake --help".into(),
                    detail: "getvar option present".into(),
                    argv: vec!["bitbake".into(), "--help".into()],
                }],
                implementation: Some(CompatibilityImplementationData {
                    id: "bitbake.getvar.native".into(),
                    kind: "native".into(),
                }),
            }],
        }
    }

    #[test]
    fn compatibility_snapshot_round_trips_bounded_identity_state_and_evidence() {
        let snapshot = compatibility_snapshot_fixture(7);
        snapshot.validate().unwrap();

        let encoded = serde_json::to_vec(&snapshot).unwrap();
        let decoded: CompatibilitySnapshotData = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, snapshot);
        assert!(decoded.capabilities[0].state.is_enabled());
    }

    #[test]
    fn compatibility_validation_rejects_duplicate_oversized_and_unsupported_evidence() {
        let mut duplicate = compatibility_snapshot_fixture(1);
        duplicate
            .capabilities
            .push(duplicate.capabilities[0].clone());
        assert!(matches!(
            duplicate.validate(),
            Err(CompatibilityProtocolError::DuplicateCapability(_))
        ));

        let mut oversized = compatibility_snapshot_fixture(1);
        oversized.capabilities[0].evidence[0].argv =
            vec!["argument".into(); MAX_COMPATIBILITY_ARGV + 1];
        assert_eq!(
            oversized.validate(),
            Err(CompatibilityProtocolError::Oversized("evidence argv"))
        );

        let mut contradicted = compatibility_snapshot_fixture(1);
        contradicted.capabilities[0].state = CompatibilityStateData::Unavailable {
            reason: CompatibilityReasonData {
                code: "command_missing".into(),
                message: "The command is unavailable.".into(),
                requirement: Some("bitbake-getvar --value".into()),
            },
        };
        contradicted.capabilities[0].implementation = None;
        assert!(matches!(
            contradicted.validate(),
            Err(CompatibilityProtocolError::EvidenceMismatch(_))
        ));
    }

    #[test]
    fn compatibility_unknown_wire_values_fail_closed() {
        let state: CompatibilityStateData =
            serde_json::from_str(r#"{"state":"available_in_a_future_protocol"}"#).unwrap();
        assert_eq!(state, CompatibilityStateData::UnknownWireState);
        assert!(!state.is_enabled());

        let evidence_kind: CompatibilityEvidenceKind =
            serde_json::from_str(r#""future_probe""#).unwrap();
        let evidence_outcome: CompatibilityEvidenceOutcome =
            serde_json::from_str(r#""future_outcome""#).unwrap();
        assert_eq!(evidence_kind, CompatibilityEvidenceKind::Unknown);
        assert_eq!(evidence_outcome, CompatibilityEvidenceOutcome::Unknown);
    }

    #[test]
    fn compatibility_events_replace_newer_snapshots_and_reject_stale_generations() {
        let mut initial = daemon_snapshot_fixture();
        initial.compatibility = Some(compatibility_snapshot_fixture(1));
        let mut journal = DaemonSnapshotJournal::new(initial, DaemonSnapshotLimits::default())
            .expect("valid compatibility snapshot");

        journal
            .publish(DaemonEvent::CompatibilityChanged(Box::new(
                compatibility_snapshot_fixture(2),
            )))
            .unwrap();
        assert_eq!(
            journal
                .snapshot()
                .compatibility
                .as_ref()
                .unwrap()
                .generation,
            2
        );

        assert!(matches!(
            journal.publish(DaemonEvent::CompatibilityChanged(Box::new(
                compatibility_snapshot_fixture(2)
            ))),
            Err(DaemonSnapshotError::StaleCompatibilityGeneration {
                current: 2,
                received: 2
            })
        ));
        assert_eq!(journal.snapshot().sequence, 1);
    }

    #[test]
    fn daemon_protocol_negotiates_versions_and_capabilities_explicitly() {
        assert_eq!(
            negotiate_version(
                ProtocolVersion { major: 1, minor: 0 },
                ProtocolVersion { major: 1, minor: 4 },
                ProtocolVersion { major: 1, minor: 2 },
            )
            .unwrap(),
            ProtocolVersion { major: 1, minor: 2 }
        );
        assert!(matches!(
            negotiate_version(
                ProtocolVersion { major: 2, minor: 0 },
                ProtocolVersion { major: 2, minor: 0 },
                ProtocolVersion::CURRENT,
            ),
            Err(DaemonProtocolError::IncompatibleVersion)
        ));
        assert_eq!(
            negotiate_capabilities(
                &[Capability::PtySessions, Capability::StateSnapshots],
                &[Capability::StateSnapshots, Capability::BackgroundJobs],
            )
            .unwrap(),
            vec![Capability::StateSnapshots]
        );

        let future: Capability = serde_json::from_str("\"future_capability\"").unwrap();
        assert_eq!(future, Capability::Unknown);
        let future_event: DaemonEvent =
            serde_json::from_str(r#"{"type":"future_optional_event"}"#).unwrap();
        assert_eq!(future_event, DaemonEvent::Unknown);
    }

    #[test]
    fn daemon_protocol_round_trips_snapshot_event_and_correlated_command() {
        let snapshot = DaemonSnapshot {
            daemon_instance_id: DaemonInstanceId([7; 16]),
            sequence: 42,
            generation: 9,
            workspace: None,
            project_profile: ProjectProfileSummary::Absent,
            bitbake: BitBakeState {
                lifecycle: LifecycleState::Running,
                version: Some("2.8.1".into()),
                capabilities: vec![BitBakeCapability::WorkspaceInspection],
                diagnostic: None,
            },
            compatibility: None,
            jobs: Vec::new(),
            raw_executions: Vec::new(),
            pty_sessions: Vec::new(),
            pty_screens: Vec::new(),
            clients: vec![ClientSummary {
                id: client_id(1),
                name: "ssh-client".into(),
                attached_unix_ms: 1,
                last_seen_unix_ms: 2,
            }],
            recent_logs: Vec::new(),
            build_events: Vec::new(),
            recovery_warnings: Vec::new(),
        };
        let message = ServerMessage::Attached {
            snapshot,
            replayed_through: 42,
        };
        assert_eq!(
            decode_frame::<ServerMessage>(&encode_frame(&message).unwrap()).unwrap(),
            message
        );

        let command = ClientMessage::Command(CommandRequest {
            request_id: RequestId(81),
            expected_generation: Some(9),
            command: DaemonCommand::TakePtyControl {
                session_id: PtySessionId(3),
                expected_epoch: 0,
            },
        });
        assert_eq!(
            decode_frame::<ClientMessage>(&encode_frame(&command).unwrap()).unwrap(),
            command
        );
    }

    #[test]
    fn daemon_protocol_frames_partial_messages_and_rejects_oversize() {
        let first = encode_frame(&ClientMessage::Detach).unwrap();
        let second = encode_frame(&ClientMessage::Pong { nonce: 4 }).unwrap();
        let mut decoder = FrameDecoder::default();
        assert!(decoder.push(&first[..3]).unwrap().is_empty());
        let mut remainder = first[3..].to_vec();
        remainder.extend_from_slice(&second);
        let frames = decoder.push(&remainder).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(
            decode_frame::<ClientMessage>(&frames[0]).unwrap(),
            ClientMessage::Detach
        );
        assert_eq!(
            decode_frame::<ClientMessage>(&frames[1]).unwrap(),
            ClientMessage::Pong { nonce: 4 }
        );
        assert_eq!(decoder.pending_len(), 0);

        let mut oversized = FrameDecoder::default();
        assert!(matches!(
            oversized.push(&((MAX_FRAME_BYTES as u32 + 1).to_be_bytes())),
            Err(DaemonProtocolError::TooLarge)
        ));
    }

    #[test]
    fn daemon_protocol_covers_reconnect_stale_writer_layout_and_mouse() {
        let attach = ClientMessage::Attach {
            workspace: None,
            subscription: Subscription {
                state: true,
                jobs: true,
                logs: false,
                pty_sessions: vec![PtySessionId(8)],
            },
            resume: Some(ResumeCursor {
                daemon_instance_id: DaemonInstanceId([9; 16]),
                last_sequence: 77,
            }),
        };
        assert_eq!(
            decode_frame::<ClientMessage>(&encode_frame(&attach).unwrap()).unwrap(),
            attach
        );

        let stale = ServerMessage::CommandResult(CommandResult {
            request_id: RequestId(5),
            outcome: CommandOutcome::Rejected {
                code: ProtocolErrorCode::StaleGeneration,
                message: "snapshot replaced".into(),
                current_generation: 12,
            },
        });
        assert_eq!(
            decode_frame::<ServerMessage>(&encode_frame(&stale).unwrap()).unwrap(),
            stale
        );

        for message in [
            ClientMessage::Layout {
                event: ClientLayoutEvent::AttachSession {
                    pane_id: PaneId(2),
                    session_id: PtySessionId(8),
                },
            },
            ClientMessage::Mouse {
                event: ServerMouseEvent {
                    session_id: PtySessionId(8),
                    writer_epoch: 3,
                    kind: MouseEventKind::Drag,
                    button: 1,
                    column: 40,
                    row: 12,
                    modifiers: 0,
                },
            },
        ] {
            assert_eq!(
                decode_frame::<ClientMessage>(&encode_frame(&message).unwrap()).unwrap(),
                message
            );
        }
    }

    #[test]
    fn multi_client_state_keeps_identity_global_and_layout_local() {
        let mut snapshot = daemon_snapshot_fixture();
        snapshot.clients = vec![
            ClientSummary {
                id: ClientId([1; 16]),
                name: "left".into(),
                attached_unix_ms: 1,
                last_seen_unix_ms: 2,
            },
            ClientSummary {
                id: ClientId([2; 16]),
                name: "right".into(),
                attached_unix_ms: 1,
                last_seen_unix_ms: 2,
            },
        ];
        let event = DaemonEvent::ClientChanged(snapshot.clients[1].clone());
        let sequenced = SequencedEvent {
            sequence: 1,
            generation: 1,
            event,
        };
        apply_sequenced_event(&mut snapshot, &sequenced).unwrap();
        assert_eq!(snapshot.clients.len(), 2);
        assert_ne!(snapshot.clients[0].id, snapshot.clients[1].id);
    }

    #[test]
    fn multi_client_fanout_replays_global_events_from_independent_cursors() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        journal
            .publish(DaemonEvent::RecoveryWarning {
                message: "shared".into(),
            })
            .unwrap();
        let cursor = ResumeCursor {
            daemon_instance_id: journal.snapshot().daemon_instance_id,
            last_sequence: 0,
        };
        let first = journal.synchronize(Some(cursor));
        let second = journal.synchronize(Some(cursor));
        let events = |sync| match sync {
            DaemonSnapshotSync::Replay { events, .. } => events,
            _ => panic!("expected replay"),
        };
        assert_eq!(events(first), events(second));
    }

    #[test]
    fn next_generation_pty_screen_is_bounded_and_retained_for_reattach() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        let screen = PtyScreenSnapshot {
            session_id: PtySessionId(9),
            dimensions: TerminalDimensions {
                columns: 20,
                rows: 4,
            },
            cursor_column: 3,
            cursor_row: 1,
            rows: vec!["ready".into(), "prompt".into()],
            scrollback_lines: 7,
        };
        journal
            .publish(DaemonEvent::PtyScreen(screen.clone()))
            .unwrap();
        assert_eq!(journal.snapshot().pty_screens, vec![screen.clone()]);
        let mut replacement = screen;
        replacement.rows = vec!["updated".into()];
        journal
            .publish(DaemonEvent::PtyScreen(replacement.clone()))
            .unwrap();
        assert_eq!(journal.snapshot().pty_screens, vec![replacement]);

        let invalid = PtyScreenSnapshot {
            session_id: PtySessionId(10),
            dimensions: TerminalDimensions {
                columns: 2,
                rows: 1,
            },
            cursor_column: 0,
            cursor_row: 0,
            rows: vec!["first".into(), "extra".into()],
            scrollback_lines: 0,
        };
        assert!(matches!(
            journal.publish(DaemonEvent::PtyScreen(invalid)),
            Err(DaemonSnapshotError::InvalidPtyScreen(PtySessionId(10)))
        ));
    }

    #[test]
    fn daemon_snapshot_is_gap_free_bounded_and_replays_only_retained_events() {
        let mut journal = DaemonSnapshotJournal::new(
            daemon_snapshot_fixture(),
            DaemonSnapshotLimits {
                retained_events: 2,
                recent_logs: 2,
                snapshot_bytes: MAX_FRAME_BYTES,
            },
        )
        .unwrap();
        for index in 1..=3 {
            let event = journal
                .publish(DaemonEvent::Log(LogRecord {
                    source: "test".into(),
                    severity: LogSeverity::Info,
                    message: format!("event-{index}"),
                    unix_ms: index,
                }))
                .unwrap();
            assert_eq!(event.sequence, index);
            assert_eq!(event.generation, index);
        }
        assert_eq!(journal.snapshot().sequence, 3);
        assert_eq!(journal.snapshot().recent_logs.len(), 2);
        assert_eq!(journal.snapshot().recent_logs[0].message, "event-2");

        let replay = journal.synchronize(Some(ResumeCursor {
            daemon_instance_id: DaemonInstanceId([7; 16]),
            last_sequence: 1,
        }));
        assert!(matches!(
            replay,
            DaemonSnapshotSync::Replay {
                ref events,
                replayed_through: 3
            } if events.iter().map(|event| event.sequence).collect::<Vec<_>>() == vec![2, 3]
        ));
        assert!(matches!(
            journal.synchronize(Some(ResumeCursor {
                daemon_instance_id: DaemonInstanceId([7; 16]),
                last_sequence: 0,
            })),
            DaemonSnapshotSync::Replace {
                reason: SnapshotReplacementReason::HistoryExpired,
                ..
            }
        ));
        assert!(matches!(
            journal.synchronize(Some(ResumeCursor {
                daemon_instance_id: DaemonInstanceId([8; 16]),
                last_sequence: 3,
            })),
            DaemonSnapshotSync::Replace {
                reason: SnapshotReplacementReason::DaemonInstanceChanged,
                ..
            }
        ));

        let mut client = daemon_snapshot_fixture();
        let gap = SequencedEvent {
            sequence: 2,
            generation: 2,
            event: DaemonEvent::Unknown,
        };
        assert!(matches!(
            apply_sequenced_event(&mut client, &gap),
            Err(DaemonSnapshotError::EventGap { .. })
        ));
        assert_eq!(client.sequence, 0);
    }

    #[test]
    fn daemon_snapshot_evicts_oldest_logs_to_preserve_byte_bound() {
        let mut journal = DaemonSnapshotJournal::new(
            daemon_snapshot_fixture(),
            DaemonSnapshotLimits {
                retained_events: 16,
                recent_logs: 16,
                snapshot_bytes: 2_048,
            },
        )
        .unwrap();
        for index in 1..=4 {
            journal
                .publish(DaemonEvent::Log(LogRecord {
                    source: "bitbake".into(),
                    severity: LogSeverity::Info,
                    message: "x".repeat(900),
                    unix_ms: index,
                }))
                .unwrap();
        }
        let encoded = serde_json::to_vec(journal.snapshot()).unwrap();
        assert!(encoded.len() <= 2_048);
        assert!(journal.snapshot().recent_logs.len() < 4);
        assert_eq!(journal.snapshot().sequence, 4);
    }

    #[test]
    fn daemon_snapshot_rejects_invalid_limits_and_oversized_snapshots() {
        assert!(matches!(
            DaemonSnapshotJournal::new(
                daemon_snapshot_fixture(),
                DaemonSnapshotLimits {
                    retained_events: 0,
                    ..DaemonSnapshotLimits::default()
                },
            ),
            Err(DaemonSnapshotError::InvalidLimit("retained events"))
        ));
        assert!(matches!(
            DaemonSnapshotJournal::new(
                daemon_snapshot_fixture(),
                DaemonSnapshotLimits {
                    snapshot_bytes: 1,
                    ..DaemonSnapshotLimits::default()
                },
            ),
            Err(DaemonSnapshotError::SnapshotTooLarge { .. })
        ));

        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        assert!(matches!(
            journal.publish(DaemonEvent::Log(LogRecord {
                source: "test".into(),
                severity: LogSeverity::Error,
                message: "x".repeat(MAX_FRAME_BYTES),
                unix_ms: 0,
            })),
            Err(DaemonSnapshotError::EventTooLarge { .. })
        ));
        assert_eq!(journal.snapshot().sequence, 0);
    }

    #[test]
    fn daemon_build_snapshot_retains_typed_attach_progress() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::Reset {
                targets: vec!["core-image-minimal".into()],
            }))
            .unwrap();
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::TaskStarted {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                pid: Some(42),
                worker: Some("worker-1".into()),
                log_path: Some("/build/temp/log.do_compile".into()),
                stats: Some(TaskStatsData {
                    completed: 102,
                    total: 4090,
                    active: 8,
                    failed: 0,
                }),
            }))
            .unwrap();
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::TaskProgress {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(77),
            }))
            .unwrap();
        assert_eq!(journal.snapshot().build_events.len(), 3);
        assert!(matches!(
            journal.snapshot().build_events.last(),
            Some(DaemonBuildEvent::TaskProgress {
                progress: Some(77),
                ..
            })
        ));
        let encoded = encode_frame(&ServerMessage::Snapshot(journal.snapshot().clone())).unwrap();
        assert!(encoded.len() < MAX_FRAME_BYTES);
    }

    fn raw_execution_request_fixture() -> RawExecutionRequestData {
        RawExecutionRequestData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            request_id: "raw-request:protocol-1".into(),
            catalog_version: 1,
            command_id: "build.target".into(),
            parameters: vec![RawExecutionParameterData {
                id: "target".into(),
                value: RawParameterValueData::Target("core-image-minimal".into()),
            }],
            additional_arguments: vec!["--dry-run".into()],
            interaction: RawInteractionData::NoninteractiveJob,
            safety: RawSafetyData::Build,
            capability_generation: 7,
            build_directory: "/work/build".into(),
            preview_digest: "ab".repeat(32),
        }
    }

    fn raw_execution_chunk_fixture() -> RawOutputChunkData {
        RawOutputChunkData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            stream_id: "raw-stream:stdout-1".into(),
            stream: RawOutputStreamData::Stdout,
            sequence: 1,
            text: "héllo\n".into(),
            truncated_bytes: 2,
            dropped_lines: 1,
        }
    }

    fn raw_execution_snapshot_fixture(sequence: u64) -> RawExecutionSnapshotData {
        let chunk = raw_execution_chunk_fixture();
        RawExecutionSnapshotData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            request: raw_execution_request_fixture(),
            phase: RawExecutionPhaseData::Running,
            attachment: RawAttachmentData::Detached,
            owner: Some(RawExecutionOwnerData::Job("raw-job:protocol-1".into())),
            cancellation_requested: false,
            queued_unix_ms: 10,
            started_unix_ms: Some(20),
            elapsed_ms: 30,
            result: None,
            stdout: RawRetainedOutputData {
                stream_id: chunk.stream_id.clone(),
                stream: RawOutputStreamData::Stdout,
                retained_bytes: chunk.text.len() as u64,
                retained_lines: raw_protocol_line_count(&chunk.text) as u64,
                chunks: vec![chunk],
                next_sequence: 2,
                dropped_bytes: 2,
                dropped_lines: 1,
                truncated_chunks: 1,
            },
            stderr: RawRetainedOutputData {
                stream_id: "raw-stream:stderr-1".into(),
                stream: RawOutputStreamData::Stderr,
                chunks: Vec::new(),
                next_sequence: 1,
                retained_bytes: 0,
                retained_lines: 0,
                dropped_bytes: 0,
                dropped_lines: 0,
                truncated_chunks: 0,
            },
            sequence,
            generation: sequence,
        }
    }

    #[test]
    fn raw_execution_protocol_round_trips_request_event_chunk_snapshot_and_result() {
        let request = raw_execution_request_fixture();
        request.validate().unwrap();
        let request_round_trip: RawExecutionRequestData =
            serde_json::from_slice(&serde_json::to_vec(&request).unwrap()).unwrap();
        assert_eq!(request_round_trip, request);

        let chunk = raw_execution_chunk_fixture();
        chunk.validate().unwrap();
        let chunk_round_trip: RawOutputChunkData =
            serde_json::from_slice(&serde_json::to_vec(&chunk).unwrap()).unwrap();
        assert_eq!(chunk_round_trip, chunk);

        let result = RawExecutionResultData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            outcome: RawExecutionOutcomeData::Cancelled,
            exit_code: None,
            message: Some("cancelled by client".into()),
            elapsed_ms: 40,
            durable_reference: Some("raw-durable:history-1".into()),
        };
        result.validate().unwrap();
        let result_round_trip: RawExecutionResultData =
            serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
        assert_eq!(result_round_trip, result);

        let event = RawExecutionEventData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            request_id: request.request_id.clone(),
            sequence: 2,
            generation: 9,
            event: RawExecutionEventKindData::Finished { result },
        };
        event.validate().unwrap();
        let event_round_trip: RawExecutionEventData =
            serde_json::from_slice(&serde_json::to_vec(&event).unwrap()).unwrap();
        assert_eq!(event_round_trip, event);

        let snapshot = raw_execution_snapshot_fixture(4);
        snapshot.validate().unwrap();
        let snapshot_round_trip: RawExecutionSnapshotData =
            serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
        assert_eq!(snapshot_round_trip, snapshot);

        let command = ClientMessage::Command(CommandRequest {
            request_id: RequestId(4),
            expected_generation: Some(8),
            command: DaemonCommand::StartRaw { request },
        });
        assert_eq!(
            decode_frame::<ClientMessage>(&encode_frame(&command).unwrap()).unwrap(),
            command
        );
    }

    #[test]
    fn raw_execution_protocol_rejects_unknown_cross_kind_and_unicode_byte_overflow() {
        let mut request = raw_execution_request_fixture();
        request.request_id = "raw-job:protocol-1".into();
        assert_eq!(
            request.validate(),
            Err(RawExecutionProtocolError::InvalidIdentity("request"))
        );

        let mut request = raw_execution_request_fixture();
        request.interaction = serde_json::from_str("\"future_mode\"").unwrap();
        assert_eq!(
            request.validate(),
            Err(RawExecutionProtocolError::UnknownRequiredVariant)
        );

        let mut request = raw_execution_request_fixture();
        request.additional_arguments = vec!["界".repeat(MAX_RAW_EXECUTION_ARGUMENT_BYTES / 3 + 1)];
        assert_eq!(
            request.validate(),
            Err(RawExecutionProtocolError::InvalidArguments)
        );

        let future: RawExecutionEventKindData =
            serde_json::from_str(r#"{"type":"future_required"}"#).unwrap();
        assert_eq!(future, RawExecutionEventKindData::Unknown);
        let event = RawExecutionEventData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            request_id: "raw-request:future".into(),
            sequence: 1,
            generation: 1,
            event: future,
        };
        assert_eq!(
            event.validate(),
            Err(RawExecutionProtocolError::UnknownRequiredVariant)
        );
    }

    #[test]
    fn raw_execution_protocol_snapshot_journal_replaces_newer_and_rejects_stale() {
        let mut base = daemon_snapshot_fixture();
        base.raw_executions = vec![raw_execution_snapshot_fixture(1)];
        let mut journal =
            DaemonSnapshotJournal::new(base, DaemonSnapshotLimits::default()).unwrap();
        journal
            .publish(DaemonEvent::RawExecutionChanged(Box::new(
                raw_execution_snapshot_fixture(2),
            )))
            .unwrap();
        assert_eq!(journal.snapshot().raw_executions[0].sequence, 2);
        let before = journal.snapshot().clone();
        assert!(matches!(
            journal.publish(DaemonEvent::RawExecutionChanged(Box::new(
                raw_execution_snapshot_fixture(2)
            ))),
            Err(DaemonSnapshotError::StaleRawExecution { .. })
        ));
        assert_eq!(journal.snapshot(), &before);
    }

    #[test]
    fn resource_limits_are_explicit_and_bounded() {
        const {
            assert!(MAX_DAEMON_CLIENTS < u16::MAX as usize);
            assert!(MAX_DAEMON_PTY_SESSIONS < u16::MAX as usize);
            assert!(MAX_TERMINAL_SCROLLBACK_LINES <= MAX_SNAPSHOT_LOGS);
            assert!(MAX_PTY_OUTPUT_EVENT_BYTES <= MAX_FRAME_BYTES);
            assert!(MAX_UTILITY_OUTPUT_BYTES <= MAX_FRAME_BYTES);
        }
        let limits = ProtocolLimits {
            maximum_frame_bytes: MAX_FRAME_BYTES as u32,
            maximum_snapshot_bytes: MAX_FRAME_BYTES as u32,
            maximum_pending_requests: 64,
            maximum_queue_depth: 256,
            maximum_terminal_rows: 512,
            maximum_terminal_columns: 512,
            maximum_clients: MAX_DAEMON_CLIENTS as u16,
            maximum_pty_sessions: MAX_DAEMON_PTY_SESSIONS as u16,
            maximum_scrollback_lines: MAX_TERMINAL_SCROLLBACK_LINES as u32,
            maximum_utility_output_bytes: MAX_UTILITY_OUTPUT_BYTES as u32,
        };
        let round_trip: ProtocolLimits =
            serde_json::from_slice(&serde_json::to_vec(&limits).unwrap()).unwrap();
        assert_eq!(round_trip, limits);
    }
}
