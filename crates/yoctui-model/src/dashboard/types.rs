pub const DASHBOARD_COLLECTION_LIMIT: usize = 4;
pub const COMMAND_CENTER_COLLECTION_LIMIT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardNextActionKind {
    ReviewFailures,
    MonitorTasks,
    InspectArtifacts,
    ConfigureEnvironment,
    StartBuild,
}

impl DashboardNextActionKind {
    pub const fn action_id(self) -> &'static str {
        match self {
            Self::ReviewFailures => "dashboard.errors",
            Self::MonitorTasks => "dashboard.tasks",
            Self::InspectArtifacts => "dashboard.artifacts",
            Self::ConfigureEnvironment => "dashboard.environment",
            Self::StartBuild => "dashboard.build",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardNextAction {
    pub kind: DashboardNextActionKind,
    pub label: String,
    pub shortcut: String,
    pub state: WorkspaceAvailabilityState,
    pub enabled: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardArtifactSource {
    BackgroundJob,
    ImageInventory,
    SdkInventory,
}

impl DashboardArtifactSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::BackgroundJob => "job",
            Self::ImageInventory => "image",
            Self::SdkInventory => "SDK",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardArtifactRef<'a> {
    pub path: &'a Path,
    pub source: DashboardArtifactSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardEnvironmentState {
    Ready,
    NeedsConfiguration,
    Synchronizing,
    Disconnected,
}

impl DashboardEnvironmentState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NeedsConfiguration => "configuration required",
            Self::Synchronizing => "synchronizing",
            Self::Disconnected => "daemon disconnected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardHealthProjection {
    pub environment: DashboardEnvironmentState,
    pub replica: ClientReplicaStatus,
    pub bitbake: ClientDaemonLifecycle,
    pub build_filesystem_sample: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardProjection<'a> {
    pub summary: BuildSummary,
    pub progress: ProgressHierarchy,
    pub next_action: DashboardNextAction,
    pub failures: Vec<&'a LogEntry>,
    pub recent_work: Vec<JobHistoryRowRef<'a>>,
    pub artifacts: Vec<DashboardArtifactRef<'a>>,
    pub health: DashboardHealthProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCenterFavoriteRef<'a> {
    pub favorite: &'a RawFavorite,
    pub projection: RawFavoriteProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCenterProjection<'a> {
    pub dashboard: DashboardProjection<'a>,
    pub recent_contexts: Vec<JobHistoryRowRef<'a>>,
    pub active_jobs: Vec<&'a BackgroundJob>,
    pub favorite_commands: Vec<CommandCenterFavoriteRef<'a>>,
    pub terminals: Vec<&'a ClientDaemonPtySummary>,
}

fn push_dashboard_artifact<'a>(
    artifacts: &mut Vec<DashboardArtifactRef<'a>>,
    path: &'a Path,
    source: DashboardArtifactSource,
) {
    if artifacts.len() < DASHBOARD_COLLECTION_LIMIT
        && !artifacts.iter().any(|artifact| artifact.path == path)
    {
        artifacts.push(DashboardArtifactRef { path, source });
    }
}

