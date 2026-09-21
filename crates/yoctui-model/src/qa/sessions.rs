#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaOperationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaSessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaOperationPreview {
    pub id: QaOperationId,
    pub check: QaCheckId,
    pub family: QaCheckFamily,
    pub scope: QaScope,
    pub request: BuildRequest,
    pub indexed_arguments: Vec<String>,
    pub report_roots: Vec<PathBuf>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QaSessionStatus {
    Starting,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Lost,
}

impl QaSessionStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::TimedOut | Self::Lost
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QaOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaOutputLine {
    pub stream: QaOutputStream,
    pub line: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaSession {
    pub id: QaSessionId,
    pub operation: QaOperationPreview,
    pub status: QaSessionStatus,
    pub background_job_id: Option<BackgroundJobId>,
    pub started_at: SystemTime,
    pub finished_at: Option<SystemTime>,
    pub message: Option<String>,
    pub result_paths: Vec<PathBuf>,
    pub output: VecDeque<QaOutputLine>,
    pub dropped_output: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaLayerOperationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaLayerSessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaLayerOperationPreview {
    pub id: QaLayerOperationId,
    pub check: QaCheckId,
    pub layer: QaLayerIdentity,
    pub executable: QaExecutableIdentity,
    pub arguments: Vec<String>,
    pub indexed_arguments: Vec<String>,
    pub report_roots: Vec<PathBuf>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaLayerSession {
    pub id: QaLayerSessionId,
    pub operation: QaLayerOperationPreview,
    pub status: QaSessionStatus,
    pub started_at: SystemTime,
    pub finished_at: Option<SystemTime>,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
    pub result_paths: Vec<PathBuf>,
    pub output: VecDeque<QaOutputLine>,
    pub dropped_output: usize,
}
