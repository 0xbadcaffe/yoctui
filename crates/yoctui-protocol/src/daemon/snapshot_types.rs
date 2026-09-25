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
    #[serde(default)]
    pub raw_history: Vec<RawHistoryRecordData>,
    pub pty_sessions: Vec<PtySessionSummary>,
    #[serde(default)]
    pub pty_screens: Vec<PtyScreenSnapshot>,
    pub clients: Vec<ClientSummary>,
    pub recent_logs: Vec<LogRecord>,
    #[serde(default)]
    pub build_events: Vec<DaemonBuildEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_progress: Option<DaemonBuildProgress>,
    pub recovery_warnings: Vec<String>,
}

/// Aggregate build authority is independent of the retained per-task rows.
/// Absence supports legacy snapshots; it is not a zero-completed checkpoint.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonBuildProgress {
    #[serde(default)]
    pub cache: yoctui_model::BuildCacheState,
    pub completed: usize,
    pub total: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonBuildEvent {
    SstateSummary {
        summary: yoctui_model::SstateSummary,
    },
    Reset {
        targets: Vec<String>,
    },
    Workspace {
        data: WorkspaceData,
    },
    Started {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        started_unix_ms: Option<u64>,
    },
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        started_unix_ms: Option<u64>,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        started_unix_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        finished_unix_ms: Option<u64>,
    },
    Completed {
        success: bool,
        exit_code: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        finished_unix_ms: Option<u64>,
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
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
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
    DevtoolStatusChanged(Box<DaemonDevtoolStatusData>),
    PtyChanged(PtySessionSummary),
    PtyRemoved {
        session_id: PtySessionId,
    },
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
    #[serde(default)]
    pub pressure: DaemonPressureCounters,
    pub memory_bytes: Option<u64>,
    pub recovery: DaemonRecoveryState,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonPressureCounters {
    pub current_queue_depth: u32,
    pub maximum_queue_depth: u32,
    pub cosmetic_coalesced: u64,
    pub cosmetic_dropped: u64,
    pub reliable_waits: u64,
    pub forced_resynchronizations: u64,
    pub slow_client_disconnects: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyScreenSnapshot {
    pub session_id: PtySessionId,
    pub dimensions: TerminalDimensions,
    pub cursor_column: u16,
    pub cursor_row: u16,
    pub cursor_hidden: bool,
    pub scrollback_offset: u32,
    pub cells: Vec<PtyScreenCell>,
    pub scrollback_lines: u32,
    pub dropped_line_feeds_lower_bound: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PtyTerminalColor {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyScreenCell {
    pub index: u32,
    pub contents: String,
    pub foreground: PtyTerminalColor,
    pub background: PtyTerminalColor,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub wide: bool,
    pub wide_continuation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogRecord {
    pub source: String,
    pub severity: LogSeverity,
    pub message: String,
    pub unix_ms: u64,
    #[serde(default)]
    pub recipe: Option<String>,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub build: Option<String>,
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
