#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientReplicaStatus {
    Disconnected,
    Synchronizing,
    Current,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientDaemonLifecycle {
    Disconnected,
    Connecting,
    Running,
    Stopping,
    Exited,
    Failed,
    Lost,
}

impl ClientDaemonLifecycle {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Disconnected | Self::Exited | Self::Failed | Self::Lost
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientDaemonJobKind {
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
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDaemonJobSummary {
    pub id: u64,
    pub kind: ClientDaemonJobKind,
    pub label: String,
    pub lifecycle: ClientDaemonLifecycle,
    pub progress_current: Option<u64>,
    pub progress_total: Option<u64>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDaemonPtySummary {
    pub id: u64,
    pub name: String,
    pub lifecycle: ClientDaemonLifecycle,
    pub viewers: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDaemonPtyDetails {
    pub id: u64,
    pub kind: ClientDaemonPtyKind,
    pub cwd: String,
    pub columns: u16,
    pub rows: u16,
    pub writer: Option<[u8; 16]>,
    pub writer_epoch: u64,
    pub exit_code: Option<i32>,
    pub restartable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientDaemonPtyKind {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDaemonPtyScreen {
    pub session_id: u64,
    pub columns: u16,
    pub rows_count: u16,
    pub cursor_column: u16,
    pub cursor_row: u16,
    pub cursor_hidden: bool,
    pub scrollback_offset: u32,
    pub rows: Vec<String>,
    pub cells: Vec<ClientDaemonTerminalCell>,
    pub scrollback_lines: u32,
    pub dropped_line_feeds_lower_bound: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClientDaemonTerminalColor {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClientDaemonTerminalCell {
    pub contents: String,
    pub foreground: ClientDaemonTerminalColor,
    pub background: ClientDaemonTerminalColor,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub wide: bool,
    pub wide_continuation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientDaemonTelemetry {
    pub uptime_seconds: u64,
    pub active_jobs: usize,
    pub pty_sessions: usize,
    pub queue_depth: usize,
    pub pressure: ClientDaemonPressureCounters,
    pub memory_bytes: Option<u64>,
    pub recovery: DaemonRecoveryState,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClientDaemonPressureCounters {
    pub current_queue_depth: usize,
    pub maximum_queue_depth: usize,
    pub cosmetic_coalesced: u64,
    pub cosmetic_dropped: u64,
    pub reliable_waits: u64,
    pub forced_resynchronizations: u64,
    pub slow_client_disconnects: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDaemonView {
    pub status: ClientReplicaStatus,
    pub instance_id: Option<DaemonModelInstanceId>,
    pub instance_identity: Option<String>,
    pub sequence: u64,
    pub generation: u64,
    pub bitbake: ClientDaemonLifecycle,
    pub jobs: Vec<ClientDaemonJobSummary>,
    pub pty_sessions: Vec<ClientDaemonPtySummary>,
    pub pty_details: Vec<ClientDaemonPtyDetails>,
    pub pty_screens: Vec<ClientDaemonPtyScreen>,
    pub connected_clients: usize,
    pub recent_logs: Vec<String>,
    pub recovery_warnings: Vec<String>,
    pub telemetry: Option<ClientDaemonTelemetry>,
}

impl Default for ClientDaemonView {
    fn default() -> Self {
        Self {
            status: ClientReplicaStatus::Disconnected,
            instance_id: None,
            instance_identity: None,
            sequence: 0,
            generation: 0,
            bitbake: ClientDaemonLifecycle::Disconnected,
            jobs: Vec::new(),
            pty_sessions: Vec::new(),
            pty_details: Vec::new(),
            pty_screens: Vec::new(),
            connected_clients: 0,
            recent_logs: Vec::new(),
            recovery_warnings: Vec::new(),
            telemetry: None,
        }
    }
}

