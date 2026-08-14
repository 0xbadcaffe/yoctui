use crate::{
    App, BackgroundJobs, BuildEnvironmentState, BuildRecord, BuildState, CompletedTask,
    FocusTarget, ImageArtifactInventoryState, LogState, MaintenanceState, PackageDetailState,
    PackageIdentity, PackageInventoryState, ProjectProfileState, QaState, QemuCapability,
    QemuSession, Screen, SdkArtifactInventoryState, SdkSession, SdkToolCapability, SecurityState,
    SignatureComparisonState, SignatureDumpState, TaskId, TaskInfo, TestCapability,
    TestComparisonState, TestJunitExportState, TestResultInventoryState, TestSession, Theme,
    WicCapability, WicDeviceInventoryState, WicOutputInventoryState, WicSession, Workspace,
};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

pub const DEFAULT_DAEMON_LOG_LIMIT: usize = 10_000;
pub const DEFAULT_DAEMON_ERROR_LIMIT: usize = 1_000;
pub const DEFAULT_DAEMON_HISTORY_LIMIT: usize = 1_000;
pub const MAX_DAEMON_COLLECTION_LIMIT: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DaemonModelInstanceId(pub [u8; 16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonRevision {
    pub instance_id: DaemonModelInstanceId,
    pub sequence: u64,
    pub generation: u64,
}

impl DaemonRevision {
    pub fn advance(&mut self) -> Result<(), DaemonStateError> {
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(DaemonStateError::RevisionExhausted)?;
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(DaemonStateError::RevisionExhausted)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonStateLimits {
    pub logs: usize,
    pub errors: usize,
    pub history: usize,
}

impl Default for DaemonStateLimits {
    fn default() -> Self {
        Self {
            logs: DEFAULT_DAEMON_LOG_LIMIT,
            errors: DEFAULT_DAEMON_ERROR_LIMIT,
            history: DEFAULT_DAEMON_HISTORY_LIMIT,
        }
    }
}

impl DaemonStateLimits {
    pub fn validate(self) -> Result<Self, DaemonStateError> {
        for (collection, limit) in [
            ("logs", self.logs),
            ("errors", self.errors),
            ("history", self.history),
        ] {
            if limit == 0 || limit > MAX_DAEMON_COLLECTION_LIMIT {
                return Err(DaemonStateError::InvalidLimit { collection, limit });
            }
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonBitBakeLifecycle {
    Disconnected,
    Connecting,
    Connected,
    Stopping,
    Failed,
    Recovering,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonBitBakeState {
    pub lifecycle: DaemonBitBakeLifecycle,
    pub version: Option<String>,
    pub capabilities: Vec<String>,
    pub diagnostic: Option<String>,
}

impl Default for DaemonBitBakeState {
    fn default() -> Self {
        Self {
            lifecycle: DaemonBitBakeLifecycle::Disconnected,
            version: None,
            capabilities: Vec::new(),
            diagnostic: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonRecoveryState {
    CleanStart,
    Recovering,
    Recovered,
    Degraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonTelemetry {
    pub uptime_seconds: u64,
    pub connected_clients: usize,
    pub active_jobs: usize,
    pub pty_sessions: usize,
    pub queue_depth: usize,
    pub memory_bytes: Option<u64>,
    pub recovery: DaemonRecoveryState,
}

impl Default for DaemonTelemetry {
    fn default() -> Self {
        Self {
            uptime_seconds: 0,
            connected_clients: 0,
            active_jobs: 0,
            pty_sessions: 0,
            queue_depth: 0,
            memory_bytes: None,
            recovery: DaemonRecoveryState::CleanStart,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonSessionMetadata {
    pub started_unix_ms: u64,
    pub boot_id: String,
    pub recovery: DaemonRecoveryState,
    pub recovery_warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonGlobalState {
    pub revision: DaemonRevision,
    pub limits: DaemonStateLimits,
    pub workspace: Workspace,
    pub build_environment: BuildEnvironmentState,
    pub project_profile: ProjectProfileState,
    pub bitbake: DaemonBitBakeState,
    pub session: DaemonSessionMetadata,
    pub recent_logs: VecDeque<String>,
    pub recent_errors: VecDeque<String>,
    pub task_history: VecDeque<String>,
    pub jobs: Option<DaemonJobState>,
}

impl DaemonGlobalState {
    pub fn new(
        instance_id: DaemonModelInstanceId,
        started_unix_ms: u64,
        boot_id: String,
        limits: DaemonStateLimits,
    ) -> Result<Self, DaemonStateError> {
        let limits = limits.validate()?;
        Ok(Self {
            revision: DaemonRevision {
                instance_id,
                sequence: 0,
                generation: 0,
            },
            limits,
            workspace: Workspace::default(),
            build_environment: BuildEnvironmentState::Unconfigured,
            project_profile: ProjectProfileState::NotLoaded,
            bitbake: DaemonBitBakeState::default(),
            session: DaemonSessionMetadata {
                started_unix_ms,
                boot_id,
                recovery: DaemonRecoveryState::CleanStart,
                recovery_warnings: Vec::new(),
            },
            recent_logs: VecDeque::new(),
            recent_errors: VecDeque::new(),
            task_history: VecDeque::new(),
            jobs: None,
        })
    }

    pub fn mutate(
        &mut self,
        mutation: impl FnOnce(&mut Self),
    ) -> Result<DaemonRevision, DaemonStateError> {
        mutation(self);
        trim_front(&mut self.recent_logs, self.limits.logs);
        trim_front(&mut self.recent_errors, self.limits.errors);
        trim_front(&mut self.task_history, self.limits.history);
        self.revision.advance()?;
        Ok(self.revision)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonStateAction {
    ReplaceWorkspace(Workspace),
    ReplaceBuildEnvironment(BuildEnvironmentState),
    ReplaceProjectProfile(ProjectProfileState),
    ReplaceBitBake(DaemonBitBakeState),
    ReplaceJobs(Box<DaemonJobState>),
    ReplaceRecovery {
        state: DaemonRecoveryState,
        warnings: Vec<String>,
    },
    RecordLog(String),
    RecordError(String),
    RecordTaskHistory(String),
}

pub fn update_daemon_state(
    state: &mut DaemonGlobalState,
    action: DaemonStateAction,
) -> Result<DaemonRevision, DaemonStateError> {
    state.mutate(|state| match action {
        DaemonStateAction::ReplaceWorkspace(workspace) => state.workspace = workspace,
        DaemonStateAction::ReplaceBuildEnvironment(environment) => {
            state.build_environment = environment;
        }
        DaemonStateAction::ReplaceProjectProfile(profile) => state.project_profile = profile,
        DaemonStateAction::ReplaceBitBake(bitbake) => state.bitbake = bitbake,
        DaemonStateAction::ReplaceJobs(jobs) => state.jobs = Some(*jobs),
        DaemonStateAction::ReplaceRecovery {
            state: recovery,
            warnings,
        } => {
            state.session.recovery = recovery;
            state.session.recovery_warnings = warnings;
        }
        DaemonStateAction::RecordLog(message) => state.recent_logs.push_back(message),
        DaemonStateAction::RecordError(message) => state.recent_errors.push_back(message),
        DaemonStateAction::RecordTaskHistory(message) => state.task_history.push_back(message),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonPtySessionMetadata {
    pub id: u64,
    pub name: String,
    pub kind: String,
    pub lifecycle: DaemonPtyLifecycle,
    pub restartable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonPtyLifecycle {
    Running,
    Exited,
    Lost,
}

/// Existing typed long-lived workflow state captured without UI selection,
/// focus, dialogs, editors, search queries, or other client presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonJobState {
    pub build: BuildState,
    pub background_jobs: BackgroundJobs,
    pub build_history: VecDeque<BuildRecord>,
    pub active_tasks: HashMap<TaskId, TaskInfo>,
    pub completed_tasks: VecDeque<CompletedTask>,
    pub logs: LogState,
    pub signature_dump: SignatureDumpState,
    pub signature_comparison: SignatureComparisonState,
    pub package_inventory: PackageInventoryState,
    pub package_details: HashMap<PackageIdentity, PackageDetailState>,
    pub image_artifacts: ImageArtifactInventoryState,
    pub sdk_artifacts: SdkArtifactInventoryState,
    pub sdk_tool_capability: SdkToolCapability,
    pub sdk_sessions: VecDeque<SdkSession>,
    pub test_capability: TestCapability,
    pub test_sessions: VecDeque<TestSession>,
    pub test_results: TestResultInventoryState,
    pub test_comparison: TestComparisonState,
    pub test_junit_export: TestJunitExportState,
    pub security: SecurityState,
    pub qa: QaState,
    pub maintenance: MaintenanceState,
    pub qemu_capability: QemuCapability,
    pub qemu_sessions: VecDeque<QemuSession>,
    pub wic_capability: WicCapability,
    pub wic_outputs: WicOutputInventoryState,
    pub wic_devices: WicDeviceInventoryState,
    pub wic_sessions: VecDeque<WicSession>,
    pub pty_sessions: VecDeque<DaemonPtySessionMetadata>,
}

impl DaemonJobState {
    pub fn capture(app: &App) -> Self {
        Self {
            build: app.build.clone(),
            background_jobs: app.background_jobs.clone(),
            build_history: app.build_history.clone(),
            active_tasks: app.tasks.clone(),
            completed_tasks: app.completed_tasks.clone(),
            logs: app.logs.clone(),
            signature_dump: app.signature_dump.clone(),
            signature_comparison: app.signature_comparison.clone(),
            package_inventory: app.package_inventory.clone(),
            package_details: app.package_details.clone(),
            image_artifacts: app.image_artifacts.clone(),
            sdk_artifacts: app.sdk_artifacts.clone(),
            sdk_tool_capability: app.sdk_tool_capability.clone(),
            sdk_sessions: app.sdk_sessions.clone(),
            test_capability: app.test_capability.clone(),
            test_sessions: app.test_sessions.clone(),
            test_results: app.test_results.clone(),
            test_comparison: app.test_comparison.clone(),
            test_junit_export: app.test_junit_export.clone(),
            security: app.security.clone(),
            qa: app.qa.clone(),
            maintenance: app.maintenance.clone(),
            qemu_capability: app.qemu_capability.clone(),
            qemu_sessions: app.qemu_sessions.clone(),
            wic_capability: app.wic_capability.clone(),
            wic_outputs: app.wic_outputs.clone(),
            wic_devices: app.wic_devices.clone(),
            wic_sessions: app.wic_sessions.clone(),
            pty_sessions: VecDeque::new(),
        }
    }

    pub fn install_replica(&self, app: &mut App) {
        app.build = self.build.clone();
        app.background_jobs = self.background_jobs.clone();
        app.build_history = self.build_history.clone();
        app.tasks = self.active_tasks.clone();
        app.completed_tasks = self.completed_tasks.clone();
        app.logs = self.logs.clone();
        app.signature_dump = self.signature_dump.clone();
        app.signature_comparison = self.signature_comparison.clone();
        app.package_inventory = self.package_inventory.clone();
        app.package_details = self.package_details.clone();
        app.image_artifacts = self.image_artifacts.clone();
        app.sdk_artifacts = self.sdk_artifacts.clone();
        app.sdk_tool_capability = self.sdk_tool_capability.clone();
        app.sdk_sessions = self.sdk_sessions.clone();
        app.test_capability = self.test_capability.clone();
        app.test_sessions = self.test_sessions.clone();
        app.test_results = self.test_results.clone();
        app.test_comparison = self.test_comparison.clone();
        app.test_junit_export = self.test_junit_export.clone();
        app.security = self.security.clone();
        app.qa = self.qa.clone();
        app.maintenance = self.maintenance.clone();
        app.qemu_capability = self.qemu_capability.clone();
        app.qemu_sessions = self.qemu_sessions.clone();
        app.wic_capability = self.wic_capability.clone();
        app.wic_outputs = self.wic_outputs.clone();
        app.wic_devices = self.wic_devices.clone();
        app.wic_sessions = self.wic_sessions.clone();
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDaemonJobSummary {
    pub id: u64,
    pub label: String,
    pub lifecycle: ClientDaemonLifecycle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDaemonPtySummary {
    pub id: u64,
    pub name: String,
    pub lifecycle: ClientDaemonLifecycle,
    pub viewers: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDaemonView {
    pub status: ClientReplicaStatus,
    pub instance_identity: Option<String>,
    pub sequence: u64,
    pub generation: u64,
    pub bitbake: ClientDaemonLifecycle,
    pub jobs: Vec<ClientDaemonJobSummary>,
    pub pty_sessions: Vec<ClientDaemonPtySummary>,
    pub connected_clients: usize,
    pub recent_logs: Vec<String>,
    pub recovery_warnings: Vec<String>,
}

impl Default for ClientDaemonView {
    fn default() -> Self {
        Self {
            status: ClientReplicaStatus::Disconnected,
            instance_identity: None,
            sequence: 0,
            generation: 0,
            bitbake: ClientDaemonLifecycle::Disconnected,
            jobs: Vec::new(),
            pty_sessions: Vec::new(),
            connected_clients: 0,
            recent_logs: Vec::new(),
            recovery_warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDaemonReplica {
    pub status: ClientReplicaStatus,
    pub state: Option<DaemonGlobalState>,
}

impl Default for ClientDaemonReplica {
    fn default() -> Self {
        Self {
            status: ClientReplicaStatus::Disconnected,
            state: None,
        }
    }
}

impl ClientDaemonReplica {
    pub fn begin_synchronization(&mut self) {
        self.status = ClientReplicaStatus::Synchronizing;
    }

    pub fn replace(&mut self, snapshot: DaemonGlobalState) {
        self.state = Some(snapshot);
        self.status = ClientReplicaStatus::Current;
    }

    pub fn mark_stale(&mut self) {
        self.status = ClientReplicaStatus::Stale;
    }

    pub fn disconnect(&mut self) {
        self.status = ClientReplicaStatus::Disconnected;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientPresentationState {
    pub screen: Screen,
    pub focus: FocusTarget,
    pub navigator_selection: usize,
    pub theme: Theme,
    pub pane_layout_revision: u64,
}

impl Default for ClientPresentationState {
    fn default() -> Self {
        Self {
            screen: Screen::Dashboard,
            focus: FocusTarget::Navigator,
            navigator_selection: 0,
            theme: Theme::default(),
            pane_layout_revision: 0,
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DaemonStateError {
    #[error("invalid daemon {collection} limit {limit}")]
    InvalidLimit {
        collection: &'static str,
        limit: usize,
    },
    #[error("daemon state revision exhausted")]
    RevisionExhausted,
}

fn trim_front<T>(items: &mut VecDeque<T>, limit: usize) {
    while items.len() > limit {
        items.pop_front();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_state_partition_keeps_global_authority_and_client_presentation_distinct() {
        let mut daemon = DaemonGlobalState::new(
            DaemonModelInstanceId([1; 16]),
            10,
            "boot-a".into(),
            DaemonStateLimits {
                logs: 2,
                errors: 2,
                history: 2,
            },
        )
        .unwrap();
        let revision = daemon
            .mutate(|state| {
                state.bitbake.lifecycle = DaemonBitBakeLifecycle::Connected;
                state.bitbake.version = Some("2.8.1".into());
                state
                    .recent_logs
                    .extend(["one".into(), "two".into(), "three".into()]);
            })
            .unwrap();
        assert_eq!(revision.sequence, 1);
        assert_eq!(revision.generation, 1);
        assert_eq!(
            daemon.recent_logs.iter().cloned().collect::<Vec<_>>(),
            ["two", "three"]
        );

        let mut replica = ClientDaemonReplica::default();
        replica.begin_synchronization();
        replica.replace(daemon.clone());
        let presentation = ClientPresentationState {
            screen: Screen::Layers,
            theme: Theme::WhiteClassic,
            ..ClientPresentationState::default()
        };
        assert_eq!(replica.state.as_ref().unwrap(), &daemon);
        assert_eq!(presentation.screen, Screen::Layers);
        assert_eq!(daemon.revision, revision);
        replica.mark_stale();
        assert_eq!(replica.status, ClientReplicaStatus::Stale);
        replica.disconnect();
        assert_eq!(replica.status, ClientReplicaStatus::Disconnected);
        assert_eq!(replica.state.as_ref().unwrap().revision, revision);
    }

    #[test]
    fn daemon_state_partition_rejects_unbounded_or_zero_collection_limits() {
        for limits in [
            DaemonStateLimits {
                logs: 0,
                ..DaemonStateLimits::default()
            },
            DaemonStateLimits {
                errors: MAX_DAEMON_COLLECTION_LIMIT + 1,
                ..DaemonStateLimits::default()
            },
        ] {
            assert!(matches!(
                DaemonGlobalState::new(DaemonModelInstanceId([0; 16]), 0, "boot".into(), limits),
                Err(DaemonStateError::InvalidLimit { .. })
            ));
        }
    }

    #[test]
    fn daemon_state_partition_fails_closed_when_revision_space_is_exhausted() {
        let mut revision = DaemonRevision {
            instance_id: DaemonModelInstanceId([0; 16]),
            sequence: u64::MAX,
            generation: 0,
        };
        assert_eq!(revision.advance(), Err(DaemonStateError::RevisionExhausted));
    }

    #[test]
    fn daemon_telemetry_defaults_are_safe_and_track_runtime_counts() {
        let telemetry = DaemonTelemetry {
            uptime_seconds: 42,
            connected_clients: 2,
            active_jobs: 3,
            pty_sessions: 1,
            queue_depth: 4,
            memory_bytes: Some(8 * 1024 * 1024),
            recovery: DaemonRecoveryState::Recovered,
        };
        assert_eq!(
            DaemonTelemetry::default().recovery,
            DaemonRecoveryState::CleanStart
        );
        assert_eq!(telemetry.uptime_seconds, 42);
        assert_eq!(telemetry.connected_clients + telemetry.pty_sessions, 3);
        assert_eq!(telemetry.memory_bytes, Some(8 * 1024 * 1024));
    }

    #[test]
    fn daemon_state_jobs_reuse_existing_typed_families_without_client_presentation() {
        let mut source = App::new(16, 4096);
        source.screen = Screen::Maintenance;
        source.focus = FocusTarget::Inspector;
        source.notification = Some("client-only".into());
        source.logs.insert(crate::LogEntry {
            id: 0,
            severity: crate::Severity::Info,
            message: "daemon-owned log".into(),
            recipe: None,
            task: None,
            path: None,
            timestamp: std::time::SystemTime::UNIX_EPOCH,
            build: None,
            protected: false,
            diagnostic: None,
        });
        let jobs = DaemonJobState::capture(&source);

        let mut replica = App::new(16, 4096);
        replica.screen = Screen::Layers;
        replica.focus = FocusTarget::Navigator;
        replica.notification = Some("keep-local".into());
        jobs.install_replica(&mut replica);

        assert_eq!(replica.logs, source.logs);
        assert_eq!(replica.background_jobs, source.background_jobs);
        assert_eq!(replica.qemu_sessions, source.qemu_sessions);
        assert_eq!(replica.wic_sessions, source.wic_sessions);
        assert_eq!(replica.sdk_sessions, source.sdk_sessions);
        assert_eq!(replica.test_sessions, source.test_sessions);
        assert_eq!(replica.security, source.security);
        assert_eq!(replica.qa, source.qa);
        assert_eq!(replica.maintenance, source.maintenance);
        assert_eq!(replica.screen, Screen::Layers);
        assert_eq!(replica.focus, FocusTarget::Navigator);
        assert_eq!(replica.notification.as_deref(), Some("keep-local"));
        assert!(jobs.pty_sessions.is_empty());
    }
}
