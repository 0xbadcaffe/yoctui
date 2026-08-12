//! Typed, bounded protocol for the persistent daemon and attachable clients.
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PROTOCOL_MAJOR: u16 = 1;
pub const PROTOCOL_MINOR: u16 = 0;
pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CAPABILITIES: usize = 128;

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceIdentity {
    pub canonical_source: String,
    pub canonical_build: String,
    pub identity_hash: String,
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
    },
    ReleasePtyControl {
        session_id: PtySessionId,
    },
    PrepareShutdown,
    ConfirmShutdown {
        confirmation: ConfirmationLease,
    },
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
    pub jobs: Vec<JobSummary>,
    pub pty_sessions: Vec<PtySessionSummary>,
    pub clients: Vec<ClientSummary>,
    pub recent_logs: Vec<LogRecord>,
    pub recovery_warnings: Vec<String>,
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
    JobChanged(JobSummary),
    JobRemoved {
        job_id: JobId,
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
    #[serde(other)]
    Unknown,
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
            jobs: Vec::new(),
            pty_sessions: Vec::new(),
            clients: vec![ClientSummary {
                id: client_id(1),
                name: "ssh-client".into(),
                attached_unix_ms: 1,
                last_seen_unix_ms: 2,
            }],
            recent_logs: Vec::new(),
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
}
