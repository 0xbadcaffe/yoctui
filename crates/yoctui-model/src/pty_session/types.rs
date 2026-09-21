pub const MIN_PTY_COLUMNS: u16 = 2;
pub const MIN_PTY_ROWS: u16 = 1;
pub const MAX_PTY_COLUMNS: u16 = 1_000;
pub const MAX_PTY_ROWS: u16 = 1_000;
pub const MAX_PTY_NAME_BYTES: usize = 128;
pub const MAX_PTY_ARGUMENTS: usize = 128;
pub const MAX_PTY_ARGUMENT_BYTES: usize = 16 * 1024;
pub const MAX_PTY_SCROLLBACK_LINES: usize = 100_000;
pub const MAX_PTY_SCROLLBACK_CELLS: usize = 10_000_000;
pub const MAX_PTY_SCROLLBACK_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PtySessionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PtyClientId(pub [u8; 16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtyDimensions {
    pub columns: u16,
    pub rows: u16,
}

impl PtyDimensions {
    pub fn validate(self) -> Result<Self, PtySessionError> {
        if !(MIN_PTY_COLUMNS..=MAX_PTY_COLUMNS).contains(&self.columns)
            || !(MIN_PTY_ROWS..=MAX_PTY_ROWS).contains(&self.rows)
        {
            return Err(PtySessionError::InvalidDimensions(self));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtySessionKind {
    BuildShell,
    SourceShell,
    LayerShell,
    RecipeShell,
    DevtoolShell,
    SdkShell,
    NativeShell,
    DeployShell,
    Devshell,
    Menuconfig,
    QemuConsole,
    SshConsole,
    InteractiveTool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyCommandIdentity {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyWorkspaceContext {
    pub source_dir: PathBuf,
    pub build_dir: PathBuf,
    pub authorized_context_roots: Vec<PathBuf>,
    pub owner_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtySessionSpec {
    pub id: PtySessionId,
    pub name: String,
    pub kind: PtySessionKind,
    pub cwd: PathBuf,
    pub command: PtyCommandIdentity,
    pub dimensions: PtyDimensions,
    pub restartable: bool,
    pub workspace: PtyWorkspaceContext,
}

impl PtySessionSpec {
    pub fn validate(&self) -> Result<(), PtySessionError> {
        validate_spec(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtySessionLifecycle {
    Starting,
    Running,
    Terminating,
    Exited,
    Lost,
}

impl PtySessionLifecycle {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Exited | Self::Lost)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyExitStatus {
    Code(i32),
    Signal(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PtyScrollbackMetadata {
    pub first_sequence: u64,
    pub next_sequence: u64,
    pub retained_lines: usize,
    pub retained_cells: usize,
    pub retained_bytes: usize,
    pub dropped_lines: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtyWriterLease {
    pub client: PtyClientId,
    pub epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtySession {
    pub id: PtySessionId,
    pub name: String,
    pub kind: PtySessionKind,
    pub cwd: PathBuf,
    pub command: PtyCommandIdentity,
    pub lifecycle: PtySessionLifecycle,
    pub dimensions: PtyDimensions,
    pub attached_clients: BTreeSet<PtyClientId>,
    pub writer: Option<PtyWriterLease>,
    pub writer_epoch: u64,
    pub process_group: Option<i32>,
    pub scrollback: PtyScrollbackMetadata,
    pub exit_status: Option<PtyExitStatus>,
    pub restartable: bool,
    pub workspace: PtyWorkspaceContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtySessionAction {
    MarkRunning,
    Attach(PtyClientId),
    Detach(PtyClientId),
    TakeControl {
        client: PtyClientId,
        expected_epoch: u64,
    },
    ReleaseControl {
        client: PtyClientId,
        expected_epoch: u64,
    },
    Resize {
        client: PtyClientId,
        writer_epoch: u64,
        dimensions: PtyDimensions,
    },
    AdvanceScrollback(PtyScrollbackMetadata),
    BeginTermination,
    Exit(PtyExitStatus),
    MarkLost,
    Rename(String),
}

