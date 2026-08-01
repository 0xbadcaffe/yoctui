use crate::Screen;
use std::{
    collections::VecDeque,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

pub const MAX_MAINTENANCE_TOOLS: usize = 32;
pub const MAX_MAINTENANCE_TARGETS: usize = 128;
pub const MAX_MAINTENANCE_PATHS: usize = 4_096;
pub const MAX_MAINTENANCE_ARGUMENTS: usize = 256;
pub const MAX_MAINTENANCE_OUTPUT: usize = 512;
pub const MAX_MAINTENANCE_EVIDENCE: usize = 256;
pub const MAX_MAINTENANCE_SESSIONS: usize = 32;
pub const MAX_MAINTENANCE_LIMITATIONS: usize = 128;
pub const MAX_MAINTENANCE_TEXT_BYTES: usize = 4_096;

fn bounded_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_MAINTENANCE_TEXT_BYTES
        && !value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
}

fn bounded_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !matches!(value, "." | "..")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+' | ':')
        })
}

fn absolute_normal_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.as_os_str().len() <= MAX_MAINTENANCE_TEXT_BYTES
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir | Component::CurDir))
}

fn normalize_text(mut values: Vec<String>, maximum: usize) -> Vec<String> {
    values.retain(|value| bounded_text(value));
    values.sort();
    values.dedup();
    values.truncate(maximum);
    values
}

fn normalize_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.retain(|path| absolute_normal_path(path));
    paths.sort();
    paths.dedup();
    paths.truncate(MAX_MAINTENANCE_PATHS);
    paths
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MaintenanceView {
    #[default]
    Sstate,
    Services,
    Release,
    Integrations,
}

impl MaintenanceView {
    pub fn cycle(self, backwards: bool) -> Self {
        match (self, backwards) {
            (Self::Sstate, false) | (Self::Release, true) => Self::Services,
            (Self::Services, false) | (Self::Integrations, true) => Self::Release,
            (Self::Release, false) | (Self::Sstate, true) => Self::Integrations,
            (Self::Integrations, false) | (Self::Services, true) => Self::Sstate,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Sstate => 0,
            Self::Services => 1,
            Self::Release => 2,
            Self::Integrations => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaintenanceTool {
    OeCheckSstate,
    SstateCacheManagement,
    PrServiceTool,
    LockedSignatureCache,
    BuildHistoryDiff,
    BuildCompare,
    GitArchive,
    CreatePullRequest,
    SendPullRequest,
    SendErrorReport,
    Toaster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceToolInterface {
    Native,
    SstatePython,
    SstateLegacyShell,
    DetectionOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceFileIdentity {
    pub path: PathBuf,
    pub byte_size: u64,
    pub modified_at: SystemTime,
}

impl MaintenanceFileIdentity {
    pub fn new(
        path: PathBuf,
        byte_size: u64,
        modified_at: SystemTime,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&path) {
            return Err("Maintenance file identity must be a canonical absolute non-root path");
        }
        Ok(Self {
            path,
            byte_size,
            modified_at,
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.path.clone(), self.byte_size, self.modified_at).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceToolCapability {
    Available {
        tool: MaintenanceTool,
        executable: MaintenanceFileIdentity,
        interface: MaintenanceToolInterface,
    },
    Unavailable {
        tool: MaintenanceTool,
        reason: String,
    },
}

impl MaintenanceToolCapability {
    pub fn tool(&self) -> MaintenanceTool {
        match self {
            Self::Available { tool, .. } | Self::Unavailable { tool, .. } => *tool,
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            Self::Available { executable, .. } => executable.is_valid(),
            Self::Unavailable { reason, .. } => bounded_text(reason),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MaintenanceMetadata {
    pub build_dir: Option<PathBuf>,
    pub sstate_dir: Option<PathBuf>,
    pub tmp_dir: Option<PathBuf>,
    pub stamps_dirs: Vec<PathBuf>,
    pub buildhistory_dir: Option<PathBuf>,
    pub prserv_host: Option<String>,
    pub hashserve: Option<String>,
    pub hashserve_upstream: Option<String>,
    pub signature_handler: Option<String>,
    pub native_lsb: Option<String>,
    pub machine: Option<String>,
    pub distro: Option<String>,
}

impl MaintenanceMetadata {
    pub fn new(mut value: Self) -> Result<Self, &'static str> {
        for path in [
            value.build_dir.as_ref(),
            value.sstate_dir.as_ref(),
            value.tmp_dir.as_ref(),
            value.buildhistory_dir.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if !absolute_normal_path(path) {
                return Err("Maintenance metadata path is invalid");
            }
        }
        value.stamps_dirs = normalize_paths(value.stamps_dirs);
        for text in [
            value.prserv_host.as_deref(),
            value.hashserve.as_deref(),
            value.hashserve_upstream.as_deref(),
            value.signature_handler.as_deref(),
            value.native_lsb.as_deref(),
            value.machine.as_deref(),
            value.distro.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if !bounded_text(text) {
                return Err("Maintenance metadata text is invalid");
            }
        }
        Ok(value)
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.clone()).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceCapabilitySnapshot {
    pub metadata: MaintenanceMetadata,
    pub tools: Vec<MaintenanceToolCapability>,
    pub limitations: Vec<String>,
}

impl MaintenanceCapabilitySnapshot {
    pub fn new(
        metadata: MaintenanceMetadata,
        mut tools: Vec<MaintenanceToolCapability>,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if !metadata.is_valid() || tools.iter().any(|tool| !tool.is_valid()) {
            return Err("Maintenance capability snapshot is invalid");
        }
        tools.sort_by_key(MaintenanceToolCapability::tool);
        tools.dedup_by_key(|tool| tool.tool());
        tools.truncate(MAX_MAINTENANCE_TOOLS);
        Ok(Self {
            metadata,
            tools,
            limitations: normalize_text(limitations, MAX_MAINTENANCE_LIMITATIONS),
        })
    }

    pub fn capability(&self, tool: MaintenanceTool) -> Option<&MaintenanceToolCapability> {
        self.tools
            .iter()
            .find(|capability| capability.tool() == tool)
    }

    pub fn supports(&self, tool: MaintenanceTool) -> bool {
        matches!(
            self.capability(tool),
            Some(MaintenanceToolCapability::Available { .. })
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum MaintenanceCapability {
    #[default]
    NotInspected,
    Loading(u64),
    Available {
        request: u64,
        snapshot: MaintenanceCapabilitySnapshot,
    },
    Partial {
        request: u64,
        snapshot: MaintenanceCapabilitySnapshot,
        limitations: Vec<String>,
    },
    Failed {
        request: u64,
        message: String,
    },
}

impl MaintenanceCapability {
    pub fn request(&self) -> Option<u64> {
        match self {
            Self::Loading(request)
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Failed { request, .. } => Some(*request),
            Self::NotInspected => None,
        }
    }

    pub fn snapshot(&self) -> Option<&MaintenanceCapabilitySnapshot> {
        match self {
            Self::Available { snapshot, .. } | Self::Partial { snapshot, .. } => Some(snapshot),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ServiceKind {
    Pr,
    Hash,
    Worker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Disabled,
    Configured,
    Reachable,
    Unreachable,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceLocation {
    Local,
    Remote,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceEndpointRole {
    Primary,
    Upstream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceReachability {
    NotProbed,
    Reachable,
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceEndpointDiagnostic {
    pub role: ServiceEndpointRole,
    pub value: String,
    pub location: ServiceLocation,
    pub reachability: ServiceReachability,
    pub limitation: Option<String>,
}

impl ServiceEndpointDiagnostic {
    pub fn new(
        role: ServiceEndpointRole,
        value: String,
        location: ServiceLocation,
        reachability: ServiceReachability,
        limitation: Option<String>,
    ) -> Result<Self, &'static str> {
        if !bounded_text(&value)
            || limitation
                .as_deref()
                .is_some_and(|message| !bounded_text(message))
        {
            return Err("Maintenance service endpoint diagnostic is invalid");
        }
        Ok(Self {
            role,
            value,
            location,
            reachability,
            limitation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServiceProcessEvidence {
    pub pid: u32,
    pub executable: String,
}

impl ServiceProcessEvidence {
    pub fn new(pid: u32, executable: String) -> Result<Self, &'static str> {
        if pid == 0 || !bounded_token(&executable) {
            return Err("Maintenance service process evidence is invalid");
        }
        Ok(Self { pid, executable })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDiagnostic {
    pub kind: ServiceKind,
    pub state: ServiceState,
    pub endpoints: Vec<ServiceEndpointDiagnostic>,
    pub process_evidence: Vec<ServiceProcessEvidence>,
    pub limitations: Vec<String>,
}

impl ServiceDiagnostic {
    pub fn new(
        kind: ServiceKind,
        state: ServiceState,
        mut endpoints: Vec<ServiceEndpointDiagnostic>,
        mut process_evidence: Vec<ServiceProcessEvidence>,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if endpoints.iter().any(|endpoint| {
            ServiceEndpointDiagnostic::new(
                endpoint.role,
                endpoint.value.clone(),
                endpoint.location,
                endpoint.reachability,
                endpoint.limitation.clone(),
            )
            .as_ref()
                != Ok(endpoint)
        }) || process_evidence.iter().any(|process| {
            ServiceProcessEvidence::new(process.pid, process.executable.clone()).as_ref()
                != Ok(process)
        }) {
            return Err("Maintenance service diagnostic is invalid");
        }
        endpoints.sort_by(|left, right| {
            left.role
                .cmp(&right.role)
                .then(left.value.cmp(&right.value))
        });
        endpoints.dedup_by(|left, right| left.role == right.role && left.value == right.value);
        endpoints.truncate(MAX_MAINTENANCE_PATHS);
        process_evidence.sort();
        process_evidence.dedup();
        process_evidence.truncate(MAX_MAINTENANCE_OUTPUT);
        Ok(Self {
            kind,
            state,
            endpoints,
            process_evidence,
            limitations: normalize_text(limitations, MAX_MAINTENANCE_LIMITATIONS),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum MaintenanceServiceDiagnostics {
    #[default]
    NotInspected,
    Loading(u64),
    Available {
        request: u64,
        services: Vec<ServiceDiagnostic>,
    },
    Partial {
        request: u64,
        services: Vec<ServiceDiagnostic>,
        limitations: Vec<String>,
    },
    Failed {
        request: u64,
        message: String,
    },
}

impl MaintenanceServiceDiagnostics {
    pub fn request(&self) -> Option<u64> {
        match self {
            Self::Loading(request)
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Failed { request, .. } => Some(*request),
            Self::NotInspected => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceDirectoryIdentity {
    pub path: PathBuf,
    pub modified_at: SystemTime,
}

impl MaintenanceDirectoryIdentity {
    pub fn new(path: PathBuf, modified_at: SystemTime) -> Result<Self, &'static str> {
        if !absolute_normal_path(&path) {
            return Err(
                "Maintenance directory identity must be a canonical absolute non-root path",
            );
        }
        Ok(Self { path, modified_at })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.path.clone(), self.modified_at).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionalIntegrationState {
    Available,
    Partial,
    Unavailable,
}

fn optional_state(parts: &[bool]) -> OptionalIntegrationState {
    if parts.iter().all(|present| *present) {
        OptionalIntegrationState::Available
    } else if parts.iter().any(|present| *present) {
        OptionalIntegrationState::Partial
    } else {
        OptionalIntegrationState::Unavailable
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceGitWorktreeIdentity {
    pub root: MaintenanceDirectoryIdentity,
    pub head: MaintenanceFileIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalPullRequestIntegration {
    pub state: OptionalIntegrationState,
    pub create_helper: Option<MaintenanceFileIdentity>,
    pub send_helper: Option<MaintenanceFileIdentity>,
    pub worktree: Option<MaintenanceGitWorktreeIdentity>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalErrorReportIntegration {
    pub state: OptionalIntegrationState,
    pub helper: Option<MaintenanceFileIdentity>,
    pub candidate_report: Option<MaintenanceFileIdentity>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalRepoManifestIntegration {
    pub state: OptionalIntegrationState,
    pub repo_executable: Option<MaintenanceFileIdentity>,
    pub workspace: Option<MaintenanceDirectoryIdentity>,
    pub manifest: Option<MaintenanceFileIdentity>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalToasterIntegration {
    pub state: OptionalIntegrationState,
    pub executable: Option<MaintenanceFileIdentity>,
    pub configurations: Vec<MaintenanceFileIdentity>,
    pub observed_processes: Vec<ServiceProcessEvidence>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceIntegrationsSnapshot {
    pub pull_request: OptionalPullRequestIntegration,
    pub error_report: OptionalErrorReportIntegration,
    pub repo_manifest: OptionalRepoManifestIntegration,
    pub toaster: OptionalToasterIntegration,
    pub limitations: Vec<String>,
}

impl MaintenanceIntegrationsSnapshot {
    pub fn new(mut value: Self) -> Result<Self, &'static str> {
        let file_valid = |identity: &Option<MaintenanceFileIdentity>| {
            identity
                .as_ref()
                .is_none_or(MaintenanceFileIdentity::is_valid)
        };
        if !file_valid(&value.pull_request.create_helper)
            || !file_valid(&value.pull_request.send_helper)
            || value
                .pull_request
                .worktree
                .as_ref()
                .is_some_and(|worktree| !worktree.root.is_valid() || !worktree.head.is_valid())
            || value.pull_request.state
                != optional_state(&[
                    value.pull_request.create_helper.is_some(),
                    value.pull_request.send_helper.is_some(),
                    value.pull_request.worktree.is_some(),
                ])
            || !file_valid(&value.error_report.helper)
            || !file_valid(&value.error_report.candidate_report)
            || value.error_report.state
                != optional_state(&[
                    value.error_report.helper.is_some(),
                    value.error_report.candidate_report.is_some(),
                ])
            || !file_valid(&value.repo_manifest.repo_executable)
            || value
                .repo_manifest
                .workspace
                .as_ref()
                .is_some_and(|identity| !identity.is_valid())
            || !file_valid(&value.repo_manifest.manifest)
            || value.repo_manifest.state
                != optional_state(&[
                    value.repo_manifest.repo_executable.is_some(),
                    value.repo_manifest.workspace.is_some(),
                    value.repo_manifest.manifest.is_some(),
                ])
            || !file_valid(&value.toaster.executable)
            || value
                .toaster
                .configurations
                .iter()
                .any(|identity| !identity.is_valid())
            || value.toaster.observed_processes.iter().any(|process| {
                ServiceProcessEvidence::new(process.pid, process.executable.clone()).as_ref()
                    != Ok(process)
            })
            || value.toaster.state
                != optional_state(&[
                    value.toaster.executable.is_some(),
                    !value.toaster.configurations.is_empty(),
                ])
        {
            return Err("Maintenance integration snapshot is invalid");
        }
        value
            .toaster
            .configurations
            .sort_by(|left, right| left.path.cmp(&right.path));
        value
            .toaster
            .configurations
            .dedup_by(|left, right| left.path == right.path);
        value.toaster.configurations.truncate(MAX_MAINTENANCE_PATHS);
        value.toaster.observed_processes.sort();
        value.toaster.observed_processes.dedup();
        value
            .toaster
            .observed_processes
            .truncate(MAX_MAINTENANCE_OUTPUT);
        value.pull_request.limitations =
            normalize_text(value.pull_request.limitations, MAX_MAINTENANCE_LIMITATIONS);
        value.error_report.limitations =
            normalize_text(value.error_report.limitations, MAX_MAINTENANCE_LIMITATIONS);
        value.repo_manifest.limitations =
            normalize_text(value.repo_manifest.limitations, MAX_MAINTENANCE_LIMITATIONS);
        value.toaster.limitations =
            normalize_text(value.toaster.limitations, MAX_MAINTENANCE_LIMITATIONS);
        value.limitations = normalize_text(value.limitations, MAX_MAINTENANCE_LIMITATIONS);
        Ok(value)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum MaintenanceIntegrationDiagnostics {
    #[default]
    NotInspected,
    Loading(u64),
    Available {
        request: u64,
        snapshot: MaintenanceIntegrationsSnapshot,
    },
    Partial {
        request: u64,
        snapshot: MaintenanceIntegrationsSnapshot,
        limitations: Vec<String>,
    },
    Failed {
        request: u64,
        message: String,
    },
}

impl MaintenanceIntegrationDiagnostics {
    pub fn request(&self) -> Option<u64> {
        match self {
            Self::Loading(request)
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Failed { request, .. } => Some(*request),
            Self::NotInspected => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SstateReadinessMode {
    IsolatedTmpdir,
    SameTmpdir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SstateReadinessRequest {
    pub targets: Vec<String>,
    pub mode: SstateReadinessMode,
    pub output: Option<PathBuf>,
    pub log: Option<PathBuf>,
    pub timeout_seconds: u64,
}

impl SstateReadinessRequest {
    pub fn new(
        mut targets: Vec<String>,
        mode: SstateReadinessMode,
        output: Option<PathBuf>,
        log: Option<PathBuf>,
        timeout_seconds: u64,
    ) -> Result<Self, &'static str> {
        targets.retain(|target| bounded_token(target));
        targets.sort();
        targets.dedup();
        targets.truncate(MAX_MAINTENANCE_TARGETS);
        if targets.is_empty()
            || timeout_seconds == 0
            || [output.as_ref(), log.as_ref()]
                .into_iter()
                .flatten()
                .any(|path| !absolute_normal_path(path))
        {
            return Err("sstate readiness request is invalid");
        }
        Ok(Self {
            targets,
            mode,
            output,
            log,
            timeout_seconds,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SstateCleanupMode {
    Duplicates,
    Orphans,
    UnreferencedByStamps,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SstateCleanupRequest {
    pub cache_dir: PathBuf,
    pub stamps_dirs: Vec<PathBuf>,
    pub modes: Vec<SstateCleanupMode>,
    pub jobs: u16,
}

impl SstateCleanupRequest {
    pub fn new(
        cache_dir: PathBuf,
        stamps_dirs: Vec<PathBuf>,
        mut modes: Vec<SstateCleanupMode>,
        jobs: u16,
    ) -> Result<Self, &'static str> {
        let stamps_dirs = normalize_paths(stamps_dirs);
        modes.sort_by_key(|mode| match mode {
            SstateCleanupMode::Duplicates => 0,
            SstateCleanupMode::Orphans => 1,
            SstateCleanupMode::UnreferencedByStamps => 2,
        });
        modes.dedup();
        if !absolute_normal_path(&cache_dir)
            || modes.is_empty()
            || jobs == 0
            || (modes.contains(&SstateCleanupMode::UnreferencedByStamps) && stamps_dirs.is_empty())
        {
            return Err("sstate cleanup request is invalid");
        }
        Ok(Self {
            cache_dir,
            stamps_dirs,
            modes,
            jobs,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SstateCleanupPreview {
    pub request: SstateCleanupRequest,
    pub candidates: Vec<MaintenanceFileIdentity>,
}

impl SstateCleanupPreview {
    pub fn new(
        request: SstateCleanupRequest,
        mut candidates: Vec<MaintenanceFileIdentity>,
    ) -> Result<Self, &'static str> {
        if candidates.iter().any(|candidate| {
            !candidate.is_valid() || !candidate.path.starts_with(&request.cache_dir)
        }) {
            return Err("sstate cleanup candidate escapes the cache root");
        }
        candidates.sort_by(|left, right| left.path.cmp(&right.path));
        candidates.dedup_by(|left, right| left.path == right.path);
        candidates.truncate(MAX_MAINTENANCE_PATHS);
        Ok(Self {
            request,
            candidates,
        })
    }

    pub fn required_phrase(&self) -> String {
        format!(
            "DELETE {} FROM {}",
            self.candidates.len(),
            self.request.cache_dir.display()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrServiceOperation {
    Export,
    Import,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrServiceRequest {
    pub operation: PrServiceOperation,
    pub file: PathBuf,
    pub build_dir: PathBuf,
    pub endpoint: String,
}

impl PrServiceRequest {
    pub fn new(
        operation: PrServiceOperation,
        file: PathBuf,
        build_dir: PathBuf,
        endpoint: String,
    ) -> Result<Self, &'static str> {
        let extension = file.extension().and_then(|value| value.to_str());
        if !absolute_normal_path(&file)
            || !matches!(extension, Some("conf" | "inc"))
            || !absolute_normal_path(&build_dir)
            || !bounded_text(&endpoint)
        {
            return Err("PR service file must be an absolute .conf or .inc path");
        }
        Ok(Self {
            operation,
            file,
            build_dir,
            endpoint,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedSignatureCacheRequest {
    pub locked_signatures: PathBuf,
    pub input_cache: PathBuf,
    pub output_cache: PathBuf,
    pub native_lsb: String,
    pub filter: Option<PathBuf>,
}

impl LockedSignatureCacheRequest {
    pub fn new(
        locked_signatures: PathBuf,
        input_cache: PathBuf,
        output_cache: PathBuf,
        native_lsb: String,
        filter: Option<PathBuf>,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&locked_signatures)
            || !absolute_normal_path(&input_cache)
            || !absolute_normal_path(&output_cache)
            || input_cache == output_cache
            || !bounded_token(&native_lsb)
            || filter
                .as_ref()
                .is_some_and(|path| !absolute_normal_path(path))
        {
            return Err("locked signature cache request is invalid");
        }
        Ok(Self {
            locked_signatures,
            input_cache,
            output_cache,
            native_lsb,
            filter,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildComparisonRequest {
    pub repository: PathBuf,
    pub from_revision: Option<String>,
    pub to_revision: Option<String>,
    pub report_version: bool,
    pub report_all: bool,
    pub signatures: bool,
    pub signature_diff: bool,
    pub exclude_paths: Vec<String>,
    pub no_colour: bool,
}

impl BuildComparisonRequest {
    pub fn new(mut value: Self) -> Result<Self, &'static str> {
        if !absolute_normal_path(&value.repository)
            || [value.from_revision.as_deref(), value.to_revision.as_deref()]
                .into_iter()
                .flatten()
                .any(|revision| !bounded_text(revision))
            || value.exclude_paths.iter().any(|path| !bounded_text(path))
        {
            return Err("build comparison request is invalid");
        }
        value.exclude_paths = normalize_text(value.exclude_paths, MAX_MAINTENANCE_ARGUMENTS);
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitArchiveRequest {
    pub data_dir: PathBuf,
    pub git_dir: PathBuf,
    pub create: bool,
    pub bare: bool,
    pub create_tag: bool,
    pub branch_name: String,
    pub tag_name: Option<String>,
    pub commit_subject: String,
    pub commit_body: String,
    pub tag_subject: String,
    pub tag_body: String,
    pub exclusions: Vec<String>,
    pub notes: Vec<(String, PathBuf)>,
    pub push_remote: Option<String>,
}

impl GitArchiveRequest {
    pub fn new(mut value: Self) -> Result<Self, &'static str> {
        if !absolute_normal_path(&value.data_dir)
            || !absolute_normal_path(&value.git_dir)
            || !bounded_text(&value.branch_name)
            || !bounded_text(&value.commit_subject)
            || (!value.commit_body.is_empty() && !bounded_text(&value.commit_body))
            || !bounded_text(&value.tag_subject)
            || (!value.tag_body.is_empty() && !bounded_text(&value.tag_body))
            || value
                .tag_name
                .as_deref()
                .is_some_and(|name| !bounded_text(name))
            || value
                .push_remote
                .as_deref()
                .is_some_and(|remote| !bounded_token(remote))
            || value
                .notes
                .iter()
                .any(|(reference, path)| !bounded_text(reference) || !absolute_normal_path(path))
        {
            return Err("Git archive request is invalid");
        }
        value.exclusions = normalize_text(value.exclusions, MAX_MAINTENANCE_ARGUMENTS);
        value
            .notes
            .sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        value.notes.dedup();
        value.notes.truncate(MAX_MAINTENANCE_ARGUMENTS);
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceOperation {
    SstateReadiness(SstateReadinessRequest),
    SstateCleanup(SstateCleanupPreview),
    PrService(PrServiceRequest),
    LockedSignatureCache(LockedSignatureCacheRequest),
    BuildHistoryComparison(BuildComparisonRequest),
    BuildCompare(BuildComparisonRequest),
    GitArchive(GitArchiveRequest),
}

impl MaintenanceOperation {
    pub fn tool(&self) -> MaintenanceTool {
        match self {
            Self::SstateReadiness(_) => MaintenanceTool::OeCheckSstate,
            Self::SstateCleanup(_) => MaintenanceTool::SstateCacheManagement,
            Self::PrService(_) => MaintenanceTool::PrServiceTool,
            Self::LockedSignatureCache(_) => MaintenanceTool::LockedSignatureCache,
            Self::BuildHistoryComparison(_) => MaintenanceTool::BuildHistoryDiff,
            Self::BuildCompare(_) => MaintenanceTool::BuildCompare,
            Self::GitArchive(_) => MaintenanceTool::GitArchive,
        }
    }

    pub fn destructive(&self) -> bool {
        matches!(
            self,
            Self::SstateCleanup(_)
                | Self::PrService(_)
                | Self::LockedSignatureCache(_)
                | Self::GitArchive(_)
        )
    }

    pub fn network_side_effect(&self) -> bool {
        matches!(
            self,
            Self::GitArchive(GitArchiveRequest {
                push_remote: Some(_),
                ..
            })
        )
    }

    pub fn cleanup_phrase(&self) -> Option<String> {
        match self {
            Self::SstateCleanup(preview) => Some(preview.required_phrase()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceOperationPreview {
    pub id: u64,
    pub capability_request: u64,
    pub operation: MaintenanceOperation,
    pub arguments: Vec<String>,
    pub limitations: Vec<String>,
}

impl MaintenanceOperationPreview {
    pub fn new(
        id: u64,
        capability_request: u64,
        operation: MaintenanceOperation,
        arguments: Vec<String>,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if id == 0
            || capability_request == 0
            || arguments.is_empty()
            || arguments.len() > MAX_MAINTENANCE_ARGUMENTS
            || arguments.iter().any(|argument| !bounded_text(argument))
        {
            return Err("Maintenance operation preview is invalid");
        }
        Ok(Self {
            id,
            capability_request,
            operation,
            arguments,
            limitations: normalize_text(limitations, MAX_MAINTENANCE_LIMITATIONS),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaintenanceSessionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceOutputLine {
    pub stream: MaintenanceOutputStream,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MaintenanceReadinessField {
    #[default]
    Targets,
    Mode,
    Output,
    Log,
    Timeout,
}

impl MaintenanceReadinessField {
    pub fn cycle(self, backwards: bool) -> Self {
        match (self, backwards) {
            (Self::Targets, false) | (Self::Output, true) => Self::Mode,
            (Self::Mode, false) | (Self::Log, true) => Self::Output,
            (Self::Output, false) | (Self::Timeout, true) => Self::Log,
            (Self::Log, false) | (Self::Targets, true) => Self::Timeout,
            (Self::Timeout, false) | (Self::Mode, true) => Self::Targets,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceReadinessDraft {
    pub field: MaintenanceReadinessField,
    pub targets: String,
    pub mode: SstateReadinessMode,
    pub output: String,
    pub log: String,
    pub timeout: String,
    pub validation: Option<String>,
}

impl Default for MaintenanceReadinessDraft {
    fn default() -> Self {
        Self {
            field: MaintenanceReadinessField::Targets,
            targets: String::new(),
            mode: SstateReadinessMode::IsolatedTmpdir,
            output: String::new(),
            log: String::new(),
            timeout: "3600".into(),
            validation: None,
        }
    }
}

impl MaintenanceReadinessDraft {
    pub fn request(&self) -> Result<SstateReadinessRequest, &'static str> {
        let targets = self
            .targets
            .split(|character: char| character.is_whitespace() || character == ',')
            .filter(|target| !target.is_empty())
            .map(str::to_owned)
            .collect();
        let optional_path = |value: &str| {
            if value.is_empty() {
                Ok(None)
            } else {
                let path = PathBuf::from(value);
                absolute_normal_path(&path)
                    .then_some(Some(path))
                    .ok_or("output and log paths must be absolute normalized paths")
            }
        };
        let timeout_seconds = self
            .timeout
            .parse::<u64>()
            .map_err(|_| "timeout must be a positive integer")?;
        SstateReadinessRequest::new(
            targets,
            self.mode,
            optional_path(&self.output)?,
            optional_path(&self.log)?,
            timeout_seconds,
        )
    }

    pub fn is_bounded(&self) -> bool {
        [&self.targets, &self.output, &self.log, &self.timeout]
            .into_iter()
            .all(|value| value.len() <= MAX_MAINTENANCE_TEXT_BYTES && !value.contains('\n'))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MaintenanceCleanupField {
    #[default]
    Duplicates,
    Orphans,
    UnreferencedByStamps,
    Jobs,
}

impl MaintenanceCleanupField {
    pub fn cycle(self, backwards: bool) -> Self {
        match (self, backwards) {
            (Self::Duplicates, false) | (Self::UnreferencedByStamps, true) => Self::Orphans,
            (Self::Orphans, false) | (Self::Jobs, true) => Self::UnreferencedByStamps,
            (Self::UnreferencedByStamps, false) | (Self::Duplicates, true) => Self::Jobs,
            (Self::Jobs, false) | (Self::Orphans, true) => Self::Duplicates,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceCleanupDraft {
    pub field: MaintenanceCleanupField,
    pub cache_dir: PathBuf,
    pub stamps_dirs: Vec<PathBuf>,
    pub duplicates: bool,
    pub orphans: bool,
    pub unreferenced_by_stamps: bool,
    pub jobs: String,
    pub validation: Option<String>,
}

impl MaintenanceCleanupDraft {
    pub fn from_metadata(metadata: &MaintenanceMetadata) -> Result<Self, &'static str> {
        let cache_dir = metadata
            .sstate_dir
            .clone()
            .ok_or("SSTATE_DIR is unavailable")?;
        Ok(Self {
            field: MaintenanceCleanupField::Duplicates,
            cache_dir,
            stamps_dirs: metadata.stamps_dirs.clone(),
            duplicates: true,
            orphans: false,
            unreferenced_by_stamps: false,
            jobs: "1".into(),
            validation: None,
        })
    }

    pub fn request(&self) -> Result<SstateCleanupRequest, &'static str> {
        let modes = [
            (self.duplicates, SstateCleanupMode::Duplicates),
            (self.orphans, SstateCleanupMode::Orphans),
            (
                self.unreferenced_by_stamps,
                SstateCleanupMode::UnreferencedByStamps,
            ),
        ]
        .into_iter()
        .filter_map(|(selected, mode)| selected.then_some(mode))
        .collect();
        let jobs = self
            .jobs
            .parse::<u16>()
            .map_err(|_| "jobs must be a positive integer")?;
        SstateCleanupRequest::new(
            self.cache_dir.clone(),
            self.stamps_dirs.clone(),
            modes,
            jobs,
        )
    }

    pub fn is_bounded(&self) -> bool {
        self.jobs.len() <= 5
            && self
                .jobs
                .chars()
                .all(|character| character.is_ascii_digit())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceSessionStatus {
    Queued,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Lost,
}

impl MaintenanceSessionStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::TimedOut | Self::Lost
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceEvidence {
    pub identity: MaintenanceFileIdentity,
    pub label: String,
}

impl MaintenanceEvidence {
    pub fn new(identity: MaintenanceFileIdentity, label: String) -> Result<Self, &'static str> {
        if !identity.is_valid() || !bounded_text(&label) {
            return Err("Maintenance evidence is invalid");
        }
        Ok(Self { identity, label })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceSession {
    pub id: MaintenanceSessionId,
    pub preview: MaintenanceOperationPreview,
    pub status: MaintenanceSessionStatus,
    pub started_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
    pub output: VecDeque<MaintenanceOutputLine>,
    pub dropped_lines: usize,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
}

impl MaintenanceSession {
    fn append_output(&mut self, stream: MaintenanceOutputStream, text: String) {
        if !bounded_text(&text) {
            return;
        }
        if self.output.len() == MAX_MAINTENANCE_OUTPUT {
            self.output.pop_front();
            self.dropped_lines = self.dropped_lines.saturating_add(1);
        }
        self.output
            .push_back(MaintenanceOutputLine { stream, text });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceDialog {
    ReadinessForm(Box<MaintenanceReadinessDraft>),
    CleanupForm(Box<MaintenanceCleanupDraft>),
    Confirm(MaintenanceOperationPreview),
    CleanupPhrase {
        preview: MaintenanceOperationPreview,
        input: String,
    },
    ConfirmNetworkPush(MaintenanceOperationPreview),
    ConfirmCancellation(MaintenanceSessionId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceState {
    pub view: MaintenanceView,
    pub selections: [usize; 4],
    pub capability: MaintenanceCapability,
    pub services: MaintenanceServiceDiagnostics,
    pub integrations: MaintenanceIntegrationDiagnostics,
    pub pending: Option<MaintenanceOperationPreview>,
    pub sessions: VecDeque<MaintenanceSession>,
    pub evidence: Vec<MaintenanceEvidence>,
    pub evidence_selection: usize,
    pub capability_generation: u64,
    pub service_generation: u64,
}

impl Default for MaintenanceState {
    fn default() -> Self {
        Self {
            view: MaintenanceView::Sstate,
            selections: [0; 4],
            capability: MaintenanceCapability::NotInspected,
            services: MaintenanceServiceDiagnostics::NotInspected,
            integrations: MaintenanceIntegrationDiagnostics::NotInspected,
            pending: None,
            sessions: VecDeque::new(),
            evidence: Vec::new(),
            evidence_selection: 0,
            capability_generation: 0,
            service_generation: 0,
        }
    }
}

impl MaintenanceState {
    pub fn selection(&self) -> usize {
        self.selections[self.view.index()]
    }

    pub fn active_session(&self) -> Option<&MaintenanceSession> {
        self.sessions
            .iter()
            .rev()
            .find(|session| !session.status.is_terminal())
    }

    pub fn selected_evidence(&self) -> Option<&MaintenanceEvidence> {
        self.evidence.get(self.evidence_selection)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceAction {
    CycleView {
        backwards: bool,
    },
    Select {
        delta: isize,
        row_count: usize,
    },
    InspectCapability,
    CapabilityLoaded {
        request: u64,
        snapshot: MaintenanceCapabilitySnapshot,
        partial: bool,
    },
    CapabilityFailed {
        request: u64,
        message: String,
    },
    IntegrationsLoaded {
        request: u64,
        snapshot: Box<MaintenanceIntegrationsSnapshot>,
        partial: bool,
    },
    IntegrationsFailed {
        request: u64,
        message: String,
    },
    InspectServices,
    ServicesLoaded {
        request: u64,
        services: Vec<ServiceDiagnostic>,
        limitations: Vec<String>,
    },
    ServicesFailed {
        request: u64,
        message: String,
    },
    OpenReadinessForm,
    UpdateReadinessForm(Box<MaintenanceReadinessDraft>),
    ConfirmReadinessForm(Box<MaintenanceReadinessDraft>),
    OpenCleanupForm,
    UpdateCleanupForm(Box<MaintenanceCleanupDraft>),
    ConfirmCleanupForm(Box<MaintenanceCleanupDraft>),
    BeginOperation(MaintenanceOperationPreview),
    UpdateCleanupPhrase {
        preview: MaintenanceOperationPreview,
        input: String,
    },
    ConfirmCleanupPhrase {
        preview: MaintenanceOperationPreview,
        input: String,
    },
    ConfirmOperation(MaintenanceOperationPreview),
    ConfirmNetworkPush(MaintenanceOperationPreview),
    CancelDialog,
    SessionRunning {
        id: MaintenanceSessionId,
        started_at: SystemTime,
    },
    SessionOutput {
        id: MaintenanceSessionId,
        stream: MaintenanceOutputStream,
        text: String,
    },
    CompleteSession {
        id: MaintenanceSessionId,
        exit_code: i32,
        evidence: Vec<MaintenanceEvidence>,
        finished_at: SystemTime,
    },
    FailSession {
        id: MaintenanceSessionId,
        message: String,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    TimeoutSession {
        id: MaintenanceSessionId,
        finished_at: SystemTime,
    },
    LoseSession {
        id: MaintenanceSessionId,
        message: String,
        finished_at: SystemTime,
    },
    BeginCancellation,
    ConfirmCancellation(MaintenanceSessionId),
    RejectCancellation {
        id: MaintenanceSessionId,
        message: String,
    },
    CancelSession {
        id: MaintenanceSessionId,
        finished_at: SystemTime,
    },
    SelectEvidence(isize),
    OpenSelectedEvidence,
    OpenSignatures,
    OpenSecurity,
    OpenQa,
    OpenRecipes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceEffect {
    InspectCapability {
        request: u64,
    },
    InspectServices {
        request: u64,
    },
    PreviewReadiness {
        capability_request: u64,
        request: SstateReadinessRequest,
    },
    PreviewCleanup {
        capability_request: u64,
        request: SstateCleanupRequest,
    },
    StartOperation {
        id: MaintenanceSessionId,
        preview: Box<MaintenanceOperationPreview>,
    },
    CancelOperation(MaintenanceSessionId),
    OpenEvidence(MaintenanceFileIdentity),
    Navigate(Screen),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceDialogUpdate {
    None,
    Open(Box<MaintenanceDialog>),
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceTransition {
    pub effect: Option<MaintenanceEffect>,
    pub dialog: MaintenanceDialogUpdate,
    pub notification: Option<String>,
}

impl MaintenanceTransition {
    fn none() -> Self {
        Self {
            effect: None,
            dialog: MaintenanceDialogUpdate::None,
            notification: None,
        }
    }

    fn effect(effect: MaintenanceEffect) -> Self {
        Self {
            effect: Some(effect),
            ..Self::none()
        }
    }
}

fn next_id(generation: &mut u64) -> u64 {
    *generation = generation.wrapping_add(1).max(1);
    *generation
}

fn exact_preview(state: &MaintenanceState, preview: &MaintenanceOperationPreview) -> bool {
    state.pending.as_ref() == Some(preview)
        && state.capability.request() == Some(preview.capability_request)
        && state
            .capability
            .snapshot()
            .is_some_and(|snapshot| snapshot.supports(preview.operation.tool()))
}

fn begin_session(
    state: &mut MaintenanceState,
    preview: MaintenanceOperationPreview,
) -> MaintenanceEffect {
    let id = MaintenanceSessionId(preview.id);
    if state.sessions.len() == MAX_MAINTENANCE_SESSIONS {
        state.sessions.pop_front();
    }
    state.sessions.push_back(MaintenanceSession {
        id,
        preview: preview.clone(),
        status: MaintenanceSessionStatus::Queued,
        started_at: None,
        finished_at: None,
        output: VecDeque::new(),
        dropped_lines: 0,
        exit_code: None,
        message: None,
    });
    state.pending = None;
    MaintenanceEffect::StartOperation {
        id,
        preview: Box::new(preview),
    }
}

fn session_mut(
    state: &mut MaintenanceState,
    id: MaintenanceSessionId,
) -> Option<&mut MaintenanceSession> {
    state.sessions.iter_mut().find(|session| session.id == id)
}

fn terminal_session(
    state: &mut MaintenanceState,
    id: MaintenanceSessionId,
    status: MaintenanceSessionStatus,
    exit_code: Option<i32>,
    message: Option<String>,
    finished_at: SystemTime,
) -> bool {
    let Some(session) = session_mut(state, id) else {
        return false;
    };
    if session.status.is_terminal() {
        return false;
    }
    session.status = status;
    session.exit_code = exit_code;
    session.message = message.filter(|message| bounded_text(message));
    session.finished_at = Some(finished_at);
    true
}

pub fn update_maintenance(
    state: &mut MaintenanceState,
    action: MaintenanceAction,
) -> MaintenanceTransition {
    match action {
        MaintenanceAction::CycleView { backwards } => {
            state.view = state.view.cycle(backwards);
        }
        MaintenanceAction::Select { delta, row_count } => {
            let selection = &mut state.selections[state.view.index()];
            *selection = if delta.is_negative() {
                selection.saturating_sub(delta.unsigned_abs())
            } else {
                selection
                    .saturating_add(delta as usize)
                    .min(row_count.saturating_sub(1))
            };
        }
        MaintenanceAction::InspectCapability => {
            let request = next_id(&mut state.capability_generation);
            state.capability = MaintenanceCapability::Loading(request);
            state.services = MaintenanceServiceDiagnostics::Loading(request);
            state.integrations = MaintenanceIntegrationDiagnostics::Loading(request);
            return MaintenanceTransition::effect(MaintenanceEffect::InspectCapability { request });
        }
        MaintenanceAction::CapabilityLoaded {
            request,
            snapshot,
            partial,
        } if state.capability.request() == Some(request) => {
            state.capability = if partial {
                MaintenanceCapability::Partial {
                    request,
                    limitations: snapshot.limitations.clone(),
                    snapshot,
                }
            } else {
                MaintenanceCapability::Available { request, snapshot }
            };
        }
        MaintenanceAction::CapabilityFailed { request, message }
            if state.capability.request() == Some(request) && bounded_text(&message) =>
        {
            state.capability = MaintenanceCapability::Failed { request, message };
        }
        MaintenanceAction::IntegrationsLoaded {
            request,
            snapshot,
            partial,
        } if state.integrations.request() == Some(request) => {
            let snapshot = *snapshot;
            state.integrations = if partial {
                MaintenanceIntegrationDiagnostics::Partial {
                    request,
                    limitations: snapshot.limitations.clone(),
                    snapshot,
                }
            } else {
                MaintenanceIntegrationDiagnostics::Available { request, snapshot }
            };
        }
        MaintenanceAction::IntegrationsFailed { request, message }
            if state.integrations.request() == Some(request) && bounded_text(&message) =>
        {
            state.integrations = MaintenanceIntegrationDiagnostics::Failed { request, message };
        }
        MaintenanceAction::InspectServices => {
            let request = next_id(&mut state.service_generation);
            state.services = MaintenanceServiceDiagnostics::Loading(request);
            return MaintenanceTransition::effect(MaintenanceEffect::InspectServices { request });
        }
        MaintenanceAction::ServicesLoaded {
            request,
            mut services,
            limitations,
        } if state.services.request() == Some(request) => {
            services.truncate(3);
            let limitations = normalize_text(limitations, MAX_MAINTENANCE_LIMITATIONS);
            state.services = if limitations.is_empty() {
                MaintenanceServiceDiagnostics::Available { request, services }
            } else {
                MaintenanceServiceDiagnostics::Partial {
                    request,
                    services,
                    limitations,
                }
            };
        }
        MaintenanceAction::ServicesFailed { request, message }
            if state.services.request() == Some(request) && bounded_text(&message) =>
        {
            state.services = MaintenanceServiceDiagnostics::Failed { request, message };
        }
        MaintenanceAction::OpenReadinessForm
            if state.view == MaintenanceView::Sstate
                && state
                    .capability
                    .snapshot()
                    .is_some_and(|snapshot| snapshot.supports(MaintenanceTool::OeCheckSstate)) =>
        {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(MaintenanceDialog::ReadinessForm(
                    Box::default(),
                ))),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::UpdateReadinessForm(draft) if draft.is_bounded() => {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(MaintenanceDialog::ReadinessForm(
                    draft,
                ))),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmReadinessForm(mut draft) if draft.is_bounded() => {
            match draft.request() {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewReadiness {
                            capability_request,
                            request,
                        }),
                        dialog: MaintenanceDialogUpdate::Close,
                        notification: None,
                    };
                }
                Err(message) => {
                    draft.validation = Some(message.into());
                    return MaintenanceTransition {
                        dialog: MaintenanceDialogUpdate::Open(Box::new(
                            MaintenanceDialog::ReadinessForm(draft),
                        )),
                        ..MaintenanceTransition::none()
                    };
                }
            }
        }
        MaintenanceAction::OpenCleanupForm
            if state.view == MaintenanceView::Sstate
                && state.capability.snapshot().is_some_and(|snapshot| {
                    snapshot.supports(MaintenanceTool::SstateCacheManagement)
                }) =>
        {
            if let Some(snapshot) = state.capability.snapshot()
                && let Ok(draft) = MaintenanceCleanupDraft::from_metadata(&snapshot.metadata)
            {
                return MaintenanceTransition {
                    dialog: MaintenanceDialogUpdate::Open(Box::new(
                        MaintenanceDialog::CleanupForm(Box::new(draft)),
                    )),
                    ..MaintenanceTransition::none()
                };
            }
        }
        MaintenanceAction::UpdateCleanupForm(draft) if draft.is_bounded() => {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(MaintenanceDialog::CleanupForm(
                    draft,
                ))),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmCleanupForm(mut draft) if draft.is_bounded() => {
            match draft.request() {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewCleanup {
                            capability_request,
                            request,
                        }),
                        dialog: MaintenanceDialogUpdate::Close,
                        notification: None,
                    };
                }
                Err(message) => {
                    draft.validation = Some(message.into());
                    return MaintenanceTransition {
                        dialog: MaintenanceDialogUpdate::Open(Box::new(
                            MaintenanceDialog::CleanupForm(draft),
                        )),
                        ..MaintenanceTransition::none()
                    };
                }
            }
        }
        MaintenanceAction::BeginOperation(preview)
            if state.active_session().is_none()
                && !state
                    .sessions
                    .iter()
                    .any(|session| session.id.0 == preview.id)
                && state.capability.request() == Some(preview.capability_request)
                && state
                    .capability
                    .snapshot()
                    .is_some_and(|snapshot| snapshot.supports(preview.operation.tool())) =>
        {
            state.pending = Some(preview.clone());
            let dialog = if preview.operation.cleanup_phrase().is_some() {
                MaintenanceDialog::CleanupPhrase {
                    preview,
                    input: String::new(),
                }
            } else {
                MaintenanceDialog::Confirm(preview)
            };
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(dialog)),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::UpdateCleanupPhrase { preview, input }
            if exact_preview(state, &preview)
                && input.len() <= MAX_MAINTENANCE_TEXT_BYTES
                && !input.chars().any(char::is_control) =>
        {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(MaintenanceDialog::CleanupPhrase {
                    preview,
                    input,
                })),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmCleanupPhrase { preview, input }
            if exact_preview(state, &preview)
                && preview.operation.cleanup_phrase().as_deref() == Some(input.as_str()) =>
        {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(MaintenanceDialog::Confirm(
                    preview,
                ))),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmOperation(preview) if exact_preview(state, &preview) => {
            if preview.operation.network_side_effect() {
                return MaintenanceTransition {
                    dialog: MaintenanceDialogUpdate::Open(Box::new(
                        MaintenanceDialog::ConfirmNetworkPush(preview),
                    )),
                    ..MaintenanceTransition::none()
                };
            }
            return MaintenanceTransition {
                effect: Some(begin_session(state, preview)),
                dialog: MaintenanceDialogUpdate::Close,
                notification: None,
            };
        }
        MaintenanceAction::ConfirmNetworkPush(preview)
            if exact_preview(state, &preview) && preview.operation.network_side_effect() =>
        {
            return MaintenanceTransition {
                effect: Some(begin_session(state, preview)),
                dialog: MaintenanceDialogUpdate::Close,
                notification: None,
            };
        }
        MaintenanceAction::CancelDialog => {
            state.pending = None;
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Close,
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::SessionRunning { id, started_at } => {
            if let Some(session) = session_mut(state, id)
                && session.status == MaintenanceSessionStatus::Queued
            {
                session.status = MaintenanceSessionStatus::Running;
                session.started_at = Some(started_at);
            }
        }
        MaintenanceAction::SessionOutput { id, stream, text } => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.append_output(stream, text);
            }
        }
        MaintenanceAction::CompleteSession {
            id,
            exit_code,
            mut evidence,
            finished_at,
        } => {
            if exit_code != 0 {
                let _ = terminal_session(
                    state,
                    id,
                    MaintenanceSessionStatus::Failed,
                    Some(exit_code),
                    Some(format!(
                        "Maintenance operation exited with status {exit_code}"
                    )),
                    finished_at,
                );
            } else if terminal_session(
                state,
                id,
                MaintenanceSessionStatus::Succeeded,
                Some(0),
                None,
                finished_at,
            ) {
                evidence.retain(|item| item.identity.is_valid() && bounded_text(&item.label));
                evidence.sort_by(|left, right| left.identity.path.cmp(&right.identity.path));
                evidence.dedup_by(|left, right| left.identity.path == right.identity.path);
                evidence.truncate(MAX_MAINTENANCE_EVIDENCE);
                state.evidence = evidence;
                state.evidence_selection = 0;
            }
        }
        MaintenanceAction::FailSession {
            id,
            message,
            exit_code,
            finished_at,
        } => {
            let _ = terminal_session(
                state,
                id,
                MaintenanceSessionStatus::Failed,
                exit_code,
                Some(message),
                finished_at,
            );
        }
        MaintenanceAction::TimeoutSession { id, finished_at } => {
            let _ = terminal_session(
                state,
                id,
                MaintenanceSessionStatus::TimedOut,
                None,
                Some("Maintenance operation timed out".into()),
                finished_at,
            );
        }
        MaintenanceAction::LoseSession {
            id,
            message,
            finished_at,
        } => {
            let _ = terminal_session(
                state,
                id,
                MaintenanceSessionStatus::Lost,
                None,
                Some(message),
                finished_at,
            );
        }
        MaintenanceAction::BeginCancellation => {
            if let Some(session) = state.active_session() {
                return MaintenanceTransition {
                    dialog: MaintenanceDialogUpdate::Open(Box::new(
                        MaintenanceDialog::ConfirmCancellation(session.id),
                    )),
                    ..MaintenanceTransition::none()
                };
            }
        }
        MaintenanceAction::ConfirmCancellation(id) => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = MaintenanceSessionStatus::Cancelling;
                return MaintenanceTransition {
                    effect: Some(MaintenanceEffect::CancelOperation(id)),
                    dialog: MaintenanceDialogUpdate::Close,
                    notification: None,
                };
            }
        }
        MaintenanceAction::RejectCancellation { id, message } => {
            if let Some(session) = session_mut(state, id)
                && session.status == MaintenanceSessionStatus::Cancelling
            {
                session.status = MaintenanceSessionStatus::Running;
                session.message = Some(message);
            }
        }
        MaintenanceAction::CancelSession { id, finished_at } => {
            let _ = terminal_session(
                state,
                id,
                MaintenanceSessionStatus::Cancelled,
                None,
                None,
                finished_at,
            );
        }
        MaintenanceAction::SelectEvidence(delta) => {
            state.evidence_selection = if delta.is_negative() {
                state
                    .evidence_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                state
                    .evidence_selection
                    .saturating_add(delta as usize)
                    .min(state.evidence.len().saturating_sub(1))
            };
        }
        MaintenanceAction::OpenSelectedEvidence => {
            if let Some(evidence) = state.selected_evidence() {
                return MaintenanceTransition::effect(MaintenanceEffect::OpenEvidence(
                    evidence.identity.clone(),
                ));
            }
        }
        MaintenanceAction::OpenSignatures => {
            return MaintenanceTransition::effect(MaintenanceEffect::Navigate(Screen::Signatures));
        }
        MaintenanceAction::OpenSecurity => {
            return MaintenanceTransition::effect(MaintenanceEffect::Navigate(Screen::Security));
        }
        MaintenanceAction::OpenQa => {
            return MaintenanceTransition::effect(MaintenanceEffect::Navigate(Screen::Qa));
        }
        MaintenanceAction::OpenRecipes => {
            return MaintenanceTransition::effect(MaintenanceEffect::Navigate(Screen::Recipes));
        }
        _ => {}
    }
    MaintenanceTransition::none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn identity(path: &str) -> MaintenanceFileIdentity {
        MaintenanceFileIdentity::new(PathBuf::from(path), 10, UNIX_EPOCH).unwrap()
    }

    fn capability() -> MaintenanceCapabilitySnapshot {
        MaintenanceCapabilitySnapshot::new(
            MaintenanceMetadata::new(MaintenanceMetadata {
                build_dir: Some("/build".into()),
                sstate_dir: Some("/cache".into()),
                stamps_dirs: vec!["/build/tmp/stamps".into()],
                native_lsb: Some("ubuntu".into()),
                machine: Some("qemux86-64".into()),
                ..MaintenanceMetadata::default()
            })
            .unwrap(),
            [
                MaintenanceTool::OeCheckSstate,
                MaintenanceTool::SstateCacheManagement,
                MaintenanceTool::PrServiceTool,
                MaintenanceTool::LockedSignatureCache,
                MaintenanceTool::BuildHistoryDiff,
                MaintenanceTool::BuildCompare,
                MaintenanceTool::GitArchive,
            ]
            .into_iter()
            .map(|tool| MaintenanceToolCapability::Available {
                tool,
                executable: identity(&format!("/tools/{tool:?}")),
                interface: MaintenanceToolInterface::Native,
            })
            .collect(),
            vec![],
        )
        .unwrap()
    }

    fn ready_state() -> MaintenanceState {
        let mut state = MaintenanceState::default();
        let transition = update_maintenance(&mut state, MaintenanceAction::InspectCapability);
        assert_eq!(
            transition.effect,
            Some(MaintenanceEffect::InspectCapability { request: 1 })
        );
        update_maintenance(
            &mut state,
            MaintenanceAction::CapabilityLoaded {
                request: 1,
                snapshot: capability(),
                partial: false,
            },
        );
        state
    }

    fn readiness_preview(id: u64) -> MaintenanceOperationPreview {
        MaintenanceOperationPreview::new(
            id,
            1,
            MaintenanceOperation::SstateReadiness(
                SstateReadinessRequest::new(
                    vec!["core-image-minimal".into()],
                    SstateReadinessMode::IsolatedTmpdir,
                    None,
                    None,
                    60,
                )
                .unwrap(),
            ),
            vec![
                "0: /tools/oe-check-sstate".into(),
                "1: core-image-minimal".into(),
            ],
            vec![],
        )
        .unwrap()
    }

    #[test]
    fn maintenance_workflow_preserves_selection_across_fixed_views() {
        let mut state = MaintenanceState::default();
        update_maintenance(
            &mut state,
            MaintenanceAction::Select {
                delta: 3,
                row_count: 8,
            },
        );
        update_maintenance(
            &mut state,
            MaintenanceAction::CycleView { backwards: false },
        );
        update_maintenance(
            &mut state,
            MaintenanceAction::Select {
                delta: 1,
                row_count: 3,
            },
        );
        update_maintenance(&mut state, MaintenanceAction::CycleView { backwards: true });
        assert_eq!(state.view, MaintenanceView::Sstate);
        assert_eq!(state.selection(), 3);
        assert_eq!(state.selections[MaintenanceView::Services.index()], 1);
    }

    #[test]
    fn maintenance_workflow_rejects_stale_capability_and_preview() {
        let mut state = MaintenanceState::default();
        update_maintenance(&mut state, MaintenanceAction::InspectCapability);
        update_maintenance(&mut state, MaintenanceAction::InspectCapability);
        update_maintenance(
            &mut state,
            MaintenanceAction::CapabilityLoaded {
                request: 1,
                snapshot: capability(),
                partial: false,
            },
        );
        assert!(matches!(
            state.capability,
            MaintenanceCapability::Loading(2)
        ));

        update_maintenance(
            &mut state,
            MaintenanceAction::CapabilityLoaded {
                request: 2,
                snapshot: capability(),
                partial: false,
            },
        );
        let stale = readiness_preview(3);
        assert_eq!(
            update_maintenance(&mut state, MaintenanceAction::BeginOperation(stale)),
            MaintenanceTransition::none()
        );
    }

    #[test]
    fn maintenance_workflow_requires_exact_cleanup_phrase() {
        let mut state = ready_state();
        let cleanup = SstateCleanupPreview::new(
            SstateCleanupRequest::new(
                "/cache".into(),
                vec![],
                vec![SstateCleanupMode::Duplicates],
                4,
            )
            .unwrap(),
            vec![identity("/cache/a"), identity("/cache/b")],
        )
        .unwrap();
        assert_eq!(cleanup.required_phrase(), "DELETE 2 FROM /cache");
        assert!(
            SstateCleanupPreview::new(cleanup.request.clone(), vec![identity("/outside/a")])
                .is_err()
        );

        let preview = MaintenanceOperationPreview::new(
            9,
            1,
            MaintenanceOperation::SstateCleanup(cleanup),
            vec!["0: /tools/sstate".into()],
            vec![],
        )
        .unwrap();
        let transition = update_maintenance(
            &mut state,
            MaintenanceAction::BeginOperation(preview.clone()),
        );
        assert!(matches!(
            transition.dialog,
            MaintenanceDialogUpdate::Open(dialog)
                if matches!(*dialog, MaintenanceDialog::CleanupPhrase { .. })
        ));
        assert!(
            state
                .pending
                .as_ref()
                .is_some_and(|value| value == &preview)
        );
        assert_eq!(
            update_maintenance(
                &mut state,
                MaintenanceAction::ConfirmCleanupPhrase {
                    preview: preview.clone(),
                    input: "DELETE 1 FROM /cache".into(),
                },
            ),
            MaintenanceTransition::none()
        );
        assert!(matches!(
            update_maintenance(
                &mut state,
                MaintenanceAction::ConfirmCleanupPhrase {
                    preview,
                    input: "DELETE 2 FROM /cache".into(),
                },
            )
            .dialog,
            MaintenanceDialogUpdate::Open(dialog)
                if matches!(*dialog, MaintenanceDialog::Confirm(_))
        ));
    }

    #[test]
    fn maintenance_workflow_covers_success_failure_timeout_cancel_and_loss() {
        let terminal = [
            MaintenanceSessionStatus::Succeeded,
            MaintenanceSessionStatus::Failed,
            MaintenanceSessionStatus::TimedOut,
            MaintenanceSessionStatus::Cancelled,
            MaintenanceSessionStatus::Lost,
        ];
        for (index, expected) in terminal.into_iter().enumerate() {
            let mut state = ready_state();
            let preview = readiness_preview(index as u64 + 1);
            update_maintenance(
                &mut state,
                MaintenanceAction::BeginOperation(preview.clone()),
            );
            let transition = update_maintenance(
                &mut state,
                MaintenanceAction::ConfirmOperation(preview.clone()),
            );
            let id = MaintenanceSessionId(preview.id);
            assert!(matches!(
                transition.effect,
                Some(MaintenanceEffect::StartOperation { .. })
            ));
            update_maintenance(
                &mut state,
                MaintenanceAction::SessionRunning {
                    id,
                    started_at: UNIX_EPOCH,
                },
            );
            match expected {
                MaintenanceSessionStatus::Succeeded => {
                    update_maintenance(
                        &mut state,
                        MaintenanceAction::CompleteSession {
                            id,
                            exit_code: 0,
                            evidence: vec![
                                MaintenanceEvidence::new(
                                    identity("/reports/result"),
                                    "result".into(),
                                )
                                .unwrap(),
                            ],
                            finished_at: UNIX_EPOCH + Duration::from_secs(1),
                        },
                    );
                }
                MaintenanceSessionStatus::Failed => {
                    update_maintenance(
                        &mut state,
                        MaintenanceAction::FailSession {
                            id,
                            message: "failed".into(),
                            exit_code: Some(2),
                            finished_at: UNIX_EPOCH,
                        },
                    );
                }
                MaintenanceSessionStatus::TimedOut => {
                    update_maintenance(
                        &mut state,
                        MaintenanceAction::TimeoutSession {
                            id,
                            finished_at: UNIX_EPOCH,
                        },
                    );
                }
                MaintenanceSessionStatus::Cancelled => {
                    update_maintenance(
                        &mut state,
                        MaintenanceAction::CancelSession {
                            id,
                            finished_at: UNIX_EPOCH,
                        },
                    );
                }
                MaintenanceSessionStatus::Lost => {
                    update_maintenance(
                        &mut state,
                        MaintenanceAction::LoseSession {
                            id,
                            message: "runner lost".into(),
                            finished_at: UNIX_EPOCH,
                        },
                    );
                }
                _ => unreachable!(),
            }
            assert_eq!(state.sessions.back().unwrap().status, expected);
        }
    }

    #[test]
    fn maintenance_workflow_bounds_output_and_replaces_only_successful_evidence() {
        let mut state = ready_state();
        state.evidence =
            vec![MaintenanceEvidence::new(identity("/reports/old"), "old".into()).unwrap()];
        let preview = readiness_preview(1);
        update_maintenance(
            &mut state,
            MaintenanceAction::BeginOperation(preview.clone()),
        );
        update_maintenance(&mut state, MaintenanceAction::ConfirmOperation(preview));
        let id = MaintenanceSessionId(1);
        for index in 0..MAX_MAINTENANCE_OUTPUT + 10 {
            update_maintenance(
                &mut state,
                MaintenanceAction::SessionOutput {
                    id,
                    stream: MaintenanceOutputStream::Stdout,
                    text: format!("line {index}"),
                },
            );
        }
        assert_eq!(
            state.sessions.back().unwrap().output.len(),
            MAX_MAINTENANCE_OUTPUT
        );
        assert_eq!(state.sessions.back().unwrap().dropped_lines, 10);
        update_maintenance(
            &mut state,
            MaintenanceAction::FailSession {
                id,
                message: "failed".into(),
                exit_code: Some(1),
                finished_at: UNIX_EPOCH,
            },
        );
        assert_eq!(state.evidence[0].identity.path, Path::new("/reports/old"));
    }

    #[test]
    fn maintenance_workflow_routes_existing_owned_destinations() {
        let mut state = MaintenanceState::default();
        for (action, screen) in [
            (MaintenanceAction::OpenSignatures, Screen::Signatures),
            (MaintenanceAction::OpenSecurity, Screen::Security),
            (MaintenanceAction::OpenQa, Screen::Qa),
            (MaintenanceAction::OpenRecipes, Screen::Recipes),
        ] {
            assert_eq!(
                update_maintenance(&mut state, action).effect,
                Some(MaintenanceEffect::Navigate(screen))
            );
        }
    }

    #[test]
    fn maintenance_service_model_retains_typed_endpoint_process_and_pr_context() {
        let endpoint = ServiceEndpointDiagnostic::new(
            ServiceEndpointRole::Primary,
            "localhost:8585".into(),
            ServiceLocation::Local,
            ServiceReachability::Reachable,
            None,
        )
        .unwrap();
        let process = ServiceProcessEvidence::new(42, "bitbake-prserv".into()).unwrap();
        let diagnostic = ServiceDiagnostic::new(
            ServiceKind::Pr,
            ServiceState::Reachable,
            vec![endpoint.clone(), endpoint],
            vec![process.clone(), process],
            vec!["observational only".into(), "observational only".into()],
        )
        .unwrap();
        assert_eq!(diagnostic.endpoints.len(), 1);
        assert_eq!(diagnostic.process_evidence.len(), 1);
        assert_eq!(diagnostic.limitations, vec!["observational only"]);

        let request = PrServiceRequest::new(
            PrServiceOperation::Import,
            "/evidence/pr.inc".into(),
            "/build".into(),
            "localhost:8585".into(),
        )
        .unwrap();
        assert_eq!(request.build_dir, Path::new("/build"));
        assert_eq!(request.endpoint, "localhost:8585");
        assert!(
            PrServiceRequest::new(
                PrServiceOperation::Export,
                "/evidence/pr.txt".into(),
                "/build".into(),
                "localhost:8585".into(),
            )
            .is_err()
        );
    }

    #[test]
    fn maintenance_release_model_retains_no_colour_and_separate_archive_push_intent() {
        let comparison = BuildComparisonRequest::new(BuildComparisonRequest {
            repository: "/build/buildhistory".into(),
            from_revision: Some("HEAD^".into()),
            to_revision: Some("HEAD".into()),
            report_version: true,
            report_all: false,
            signatures: true,
            signature_diff: false,
            exclude_paths: vec!["images/*".into(), "images/*".into()],
            no_colour: true,
        })
        .unwrap();
        assert!(comparison.no_colour);
        assert_eq!(comparison.exclude_paths, vec!["images/*"]);

        let archive = GitArchiveRequest::new(GitArchiveRequest {
            data_dir: "/results".into(),
            git_dir: "/archives/release.git".into(),
            create: true,
            bare: true,
            create_tag: true,
            branch_name: "release/{machine}".into(),
            tag_name: Some("release/{tag_number}".into()),
            commit_subject: "release {commit}".into(),
            commit_body: "machine: {machine}".into(),
            tag_subject: "tag {tag_number}".into(),
            tag_body: "Yoctui release archive".into(),
            exclusions: vec!["tmp/*".into(), "tmp/*".into()],
            notes: vec![("release".into(), "/results/note.txt".into())],
            push_remote: Some("origin".into()),
        })
        .unwrap();
        assert!(MaintenanceOperation::GitArchive(archive.clone()).network_side_effect());
        let mut local = archive;
        local.push_remote = None;
        assert!(!MaintenanceOperation::GitArchive(local).network_side_effect());
    }

    #[test]
    fn maintenance_integration_snapshot_validates_state_and_bounds_typed_evidence() {
        let file = |path: &str| MaintenanceFileIdentity::new(path.into(), 1, UNIX_EPOCH).unwrap();
        let directory =
            |path: &str| MaintenanceDirectoryIdentity::new(path.into(), UNIX_EPOCH).unwrap();
        let mut snapshot = MaintenanceIntegrationsSnapshot {
            pull_request: OptionalPullRequestIntegration {
                state: OptionalIntegrationState::Available,
                create_helper: Some(file("/tools/create-pull-request")),
                send_helper: Some(file("/tools/send-pull-request")),
                worktree: Some(MaintenanceGitWorktreeIdentity {
                    root: directory("/sources/poky"),
                    head: file("/sources/poky/.git/HEAD"),
                }),
                limitations: Vec::new(),
            },
            error_report: OptionalErrorReportIntegration {
                state: OptionalIntegrationState::Partial,
                helper: Some(file("/tools/send-error-report")),
                candidate_report: None,
                limitations: vec!["report unavailable".into()],
            },
            repo_manifest: OptionalRepoManifestIntegration {
                state: OptionalIntegrationState::Unavailable,
                repo_executable: None,
                workspace: None,
                manifest: None,
                limitations: vec!["repo unavailable".into()],
            },
            toaster: OptionalToasterIntegration {
                state: OptionalIntegrationState::Available,
                executable: Some(file("/tools/toaster")),
                configurations: vec![file("/config/toaster.conf"), file("/config/toaster.conf")],
                observed_processes: vec![
                    ServiceProcessEvidence::new(42, "toaster".into()).unwrap(),
                    ServiceProcessEvidence::new(42, "toaster".into()).unwrap(),
                ],
                limitations: vec!["observational only".into()],
            },
            limitations: Vec::new(),
        };
        let normalized = MaintenanceIntegrationsSnapshot::new(snapshot.clone()).unwrap();
        assert_eq!(normalized.toaster.configurations.len(), 1);
        assert_eq!(normalized.toaster.observed_processes.len(), 1);

        snapshot.pull_request.state = OptionalIntegrationState::Partial;
        assert!(MaintenanceIntegrationsSnapshot::new(snapshot).is_err());
    }

    #[test]
    fn maintenance_workflow_correlates_replaceable_integration_diagnostics() {
        let mut state = MaintenanceState::default();
        update_maintenance(&mut state, MaintenanceAction::InspectCapability);
        update_maintenance(&mut state, MaintenanceAction::InspectCapability);
        let unavailable = MaintenanceIntegrationsSnapshot::new(MaintenanceIntegrationsSnapshot {
            pull_request: OptionalPullRequestIntegration {
                state: OptionalIntegrationState::Unavailable,
                create_helper: None,
                send_helper: None,
                worktree: None,
                limitations: vec!["pull-request integration unavailable".into()],
            },
            error_report: OptionalErrorReportIntegration {
                state: OptionalIntegrationState::Unavailable,
                helper: None,
                candidate_report: None,
                limitations: Vec::new(),
            },
            repo_manifest: OptionalRepoManifestIntegration {
                state: OptionalIntegrationState::Unavailable,
                repo_executable: None,
                workspace: None,
                manifest: None,
                limitations: Vec::new(),
            },
            toaster: OptionalToasterIntegration {
                state: OptionalIntegrationState::Unavailable,
                executable: None,
                configurations: Vec::new(),
                observed_processes: Vec::new(),
                limitations: Vec::new(),
            },
            limitations: vec!["optional integrations are partial".into()],
        })
        .unwrap();

        update_maintenance(
            &mut state,
            MaintenanceAction::IntegrationsLoaded {
                request: 1,
                snapshot: Box::new(unavailable.clone()),
                partial: true,
            },
        );
        assert_eq!(
            state.integrations,
            MaintenanceIntegrationDiagnostics::Loading(2)
        );

        update_maintenance(
            &mut state,
            MaintenanceAction::IntegrationsLoaded {
                request: 2,
                snapshot: Box::new(unavailable.clone()),
                partial: true,
            },
        );
        assert!(matches!(
            state.integrations,
            MaintenanceIntegrationDiagnostics::Partial { request: 2, .. }
        ));

        update_maintenance(
            &mut state,
            MaintenanceAction::IntegrationsFailed {
                request: 1,
                message: "stale failure".into(),
            },
        );
        assert!(matches!(
            state.integrations,
            MaintenanceIntegrationDiagnostics::Partial { request: 2, .. }
        ));
    }

    #[test]
    fn maintenance_sstate_workspace_forms_validate_and_emit_typed_preview_requests() {
        let mut state = ready_state();
        let transition = update_maintenance(&mut state, MaintenanceAction::OpenReadinessForm);
        let MaintenanceDialogUpdate::Open(dialog) = transition.dialog else {
            panic!("readiness form did not open");
        };
        let MaintenanceDialog::ReadinessForm(mut draft) = *dialog else {
            panic!("wrong readiness dialog");
        };
        let invalid = update_maintenance(
            &mut state,
            MaintenanceAction::ConfirmReadinessForm(draft.clone()),
        );
        assert!(matches!(
            invalid.dialog,
            MaintenanceDialogUpdate::Open(dialog)
                if matches!(*dialog, MaintenanceDialog::ReadinessForm(ref draft) if draft.validation.is_some())
        ));
        draft.targets = "core-image-minimal busybox".into();
        draft.timeout = "45".into();
        let valid = update_maintenance(&mut state, MaintenanceAction::ConfirmReadinessForm(draft));
        assert!(matches!(
            valid.effect,
            Some(MaintenanceEffect::PreviewReadiness {
                capability_request: 1,
                request: SstateReadinessRequest {
                    timeout_seconds: 45,
                    ..
                },
            })
        ));
        assert_eq!(valid.dialog, MaintenanceDialogUpdate::Close);

        let cleanup = update_maintenance(&mut state, MaintenanceAction::OpenCleanupForm);
        let MaintenanceDialogUpdate::Open(dialog) = cleanup.dialog else {
            panic!("cleanup form did not open");
        };
        let MaintenanceDialog::CleanupForm(mut draft) = *dialog else {
            panic!("wrong cleanup dialog");
        };
        assert_eq!(draft.cache_dir, Path::new("/cache"));
        assert_eq!(draft.stamps_dirs, vec![PathBuf::from("/build/tmp/stamps")]);
        draft.duplicates = false;
        let invalid = update_maintenance(
            &mut state,
            MaintenanceAction::ConfirmCleanupForm(draft.clone()),
        );
        assert!(matches!(
            invalid.dialog,
            MaintenanceDialogUpdate::Open(dialog)
                if matches!(*dialog, MaintenanceDialog::CleanupForm(ref draft) if draft.validation.is_some())
        ));
        draft.orphans = true;
        draft.jobs = "3".into();
        let valid = update_maintenance(&mut state, MaintenanceAction::ConfirmCleanupForm(draft));
        assert!(matches!(
            valid.effect,
            Some(MaintenanceEffect::PreviewCleanup {
                capability_request: 1,
                request: SstateCleanupRequest { jobs: 3, .. },
            })
        ));
    }

    #[test]
    fn maintenance_sstate_workspace_forms_stay_inert_without_exact_capability() {
        let mut state = MaintenanceState::default();
        assert_eq!(
            update_maintenance(&mut state, MaintenanceAction::OpenReadinessForm),
            MaintenanceTransition::none()
        );
        assert_eq!(
            update_maintenance(&mut state, MaintenanceAction::OpenCleanupForm),
            MaintenanceTransition::none()
        );
    }
}
