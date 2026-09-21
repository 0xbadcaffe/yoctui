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
    QemuConsole,
    SshConsole,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyViewport {
    pub request_id: RequestId,
    pub session_id: PtySessionId,
    pub scrollback_offset: u32,
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
