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
