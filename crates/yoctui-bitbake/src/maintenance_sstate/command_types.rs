#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceSstateCommandKind {
    Readiness,
    CleanupPreview,
    CleanupExecute,
    PrServiceExport,
    PrServiceImport,
    LockedSignatureCache,
    BuildHistoryComparison,
    BuildCompare,
    GitArchiveLocal,
    GitArchivePush,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FilesystemIdentity {
    path: PathBuf,
    byte_size: u64,
    modified_at: std::time::SystemTime,
    directory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MaintenanceFilesystemGuard {
    Existing(FilesystemIdentity),
    Absent {
        path: PathBuf,
        parent: FilesystemIdentity,
    },
}

pub(crate) struct MaintenanceExternalCommand {
    pub session: MaintenanceSessionId,
    pub kind: MaintenanceSstateCommandKind,
    pub executable_identity: MaintenanceFileIdentity,
    pub expected_executable_name: String,
    pub arguments: Vec<OsString>,
    pub current_directory: PathBuf,
    pub timeout: Duration,
    pub preview: MaintenanceOperationPreview,
    pub guards: Vec<MaintenanceFilesystemGuard>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PrServiceFileGuard {
    Export {
        parent: FilesystemIdentity,
        existing: Option<FilesystemIdentity>,
    },
    Import(FilesystemIdentity),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceSstateCommandSpec {
    id: MaintenanceSessionId,
    kind: MaintenanceSstateCommandKind,
    executable_identity: MaintenanceFileIdentity,
    expected_executable_name: String,
    interface: MaintenanceToolInterface,
    arguments: Vec<OsString>,
    environment: BTreeMap<OsString, OsString>,
    current_directory: PathBuf,
    timeout: Duration,
    stdin_payload: Option<Vec<u8>>,
    preview: Option<MaintenanceOperationPreview>,
    cleanup_request: Option<SstateCleanupRequest>,
    cleanup_candidates: Vec<MaintenanceFileIdentity>,
    pr_service_guard: Option<PrServiceFileGuard>,
    external_guards: Vec<MaintenanceFilesystemGuard>,
}
