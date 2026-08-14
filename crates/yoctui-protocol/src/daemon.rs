//! Typed, bounded protocol for the persistent daemon and attachable clients.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use thiserror::Error;

pub const PROTOCOL_MAJOR: u16 = 1;
pub const PROTOCOL_MINOR: u16 = 0;
pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CAPABILITIES: usize = 128;
pub const MAX_RETAINED_EVENTS: usize = 65_536;
pub const MAX_SNAPSHOT_LOGS: usize = 100_000;
pub const MAX_DAEMON_CLIENTS: usize = 32;
pub const MAX_DAEMON_PTY_SESSIONS: usize = 64;
pub const MAX_TERMINAL_SCROLLBACK_LINES: usize = 100_000;
pub const MAX_UTILITY_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PTY_OUTPUT_EVENT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self {
        major: PROTOCOL_MAJOR,
        minor: PROTOCOL_MINOR,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub [u8; 16]);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DaemonInstanceId(pub [u8; 16]);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PtySessionId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    StateSnapshots,
    IncrementalEvents,
    EventReplay,
    BackgroundJobs,
    BitBakeLifecycle,
    PtySessions,
    PtyWriterLease,
    PaneAttachments,
    TerminalMouse,
    GracefulShutdown,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientHello {
    pub minimum_version: ProtocolVersion,
    pub maximum_version: ProtocolVersion,
    pub client_id: ClientId,
    pub client_name: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonHello {
    pub selected_version: ProtocolVersion,
    pub daemon_instance_id: DaemonInstanceId,
    pub boot_id: String,
    pub capabilities: Vec<Capability>,
    pub limits: ProtocolLimits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolLimits {
    pub maximum_frame_bytes: u32,
    pub maximum_snapshot_bytes: u32,
    pub maximum_pending_requests: u16,
    pub maximum_queue_depth: u16,
    pub maximum_terminal_rows: u16,
    pub maximum_terminal_columns: u16,
    pub maximum_clients: u16,
    pub maximum_pty_sessions: u16,
    pub maximum_scrollback_lines: u32,
    pub maximum_utility_output_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceIdentity {
    pub canonical_source: String,
    pub canonical_build: String,
    pub identity_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeCursor {
    pub daemon_instance_id: DaemonInstanceId,
    pub last_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub state: bool,
    pub jobs: bool,
    pub logs: bool,
    pub pty_sessions: Vec<PtySessionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum ClientMessage {
    Hello(ClientHello),
    Attach {
        workspace: Option<WorkspaceIdentity>,
        subscription: Subscription,
        resume: Option<ResumeCursor>,
    },
    Subscribe {
        subscription: Subscription,
    },
    Unsubscribe {
        subscription: Subscription,
    },
    Command(CommandRequest),
    PtyInput(PtyInput),
    PtyResize(PtyResize),
    Layout {
        event: ClientLayoutEvent,
    },
    Mouse {
        event: ServerMouseEvent,
    },
    Detach,
    Pong {
        nonce: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRequest {
    pub request_id: RequestId,
    pub expected_generation: Option<u64>,
    pub command: DaemonCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonCommand {
    StartBuild {
        targets: Vec<String>,
        task: Option<String>,
        force: bool,
    },
    CancelJob {
        job_id: JobId,
    },
    StartDevtool {
        operation: DaemonDevtoolOperation,
        build_directory: String,
    },
    StartSdk {
        session_id: u64,
        operation: DaemonSdkOperation,
        context: DaemonSdkContext,
    },
    CancelSdk {
        session_id: u64,
    },
    StartQemu {
        session_id: u64,
        request: DaemonQemuRequest,
        build_directory: String,
        executable: String,
    },
    CancelQemu {
        session_id: u64,
    },
    StartWicCreate {
        session_id: u64,
        request: DaemonWicCreateRequest,
        build_directory: String,
        executable: String,
    },
    StartWicWrite {
        session_id: u64,
        executable: String,
        image_path: String,
        device_path: String,
        device_major_minor: String,
        device_size_bytes: u64,
        device_model: Option<String>,
        device_serial: Option<String>,
        device_transport: Option<String>,
        build_directory: String,
    },
    CancelWic {
        session_id: u64,
    },
    StartTestSession {
        session_id: u64,
        request: DaemonTestSelftestRequest,
        build_directory: String,
        path_directories: Vec<String>,
    },
    CancelTestSession {
        session_id: u64,
    },
    ImportTestResults {
        generation: u64,
        roots: Vec<String>,
    },
    CompareTestResults {
        generation: u64,
        baseline_identity: String,
        candidate_identity: String,
    },
    ExportTestJunit {
        generation: u64,
        result_identity: String,
        destination: String,
    },
    InspectTestResultTool {
        path_directories: Vec<String>,
    },
    InspectQaCapability {
        request: DaemonQaCapabilityRequest,
    },
    StartQaLayerCheck {
        session_id: u64,
        operation_id: u64,
        check_id: String,
        layer_name: String,
        layer_root: String,
        executable: String,
        arguments: Vec<String>,
        report_roots: Vec<String>,
    },
    CancelQaLayerCheck {
        session_id: u64,
    },
    StartQaReportScan {
        generation: u64,
        build_directory: String,
        paths: Vec<String>,
    },
    CancelQaReportScan {
        generation: u64,
    },
    StartSecurityReportScan {
        generation: u64,
        paths: Vec<String>,
    },
    CancelSecurityReportScan {
        generation: u64,
    },
    StartSecurityPackageMap {
        session_id: u64,
        executable: String,
        arguments: Vec<String>,
        report_roots: Vec<String>,
    },
    CancelSecurityPackageMap {
        session_id: u64,
    },
    InspectMaintenanceCapability {
        request: u64,
        build_directory: String,
        sstate_directory: Option<String>,
        tmp_directory: Option<String>,
        stamps_directories: Vec<String>,
        executable_search_path: Vec<String>,
    },
    StartMaintenanceSstateReadiness {
        session_id: u64,
        capability_request: u64,
        operation_id: u64,
        build_directory: String,
        sstate_directory: Option<String>,
        tmp_directory: Option<String>,
        stamps_directories: Vec<String>,
        executable_search_path: Vec<String>,
        targets: Vec<String>,
        mode: String,
        output: Option<String>,
        log: Option<String>,
        timeout_seconds: u64,
    },
    CancelMaintenance {
        session_id: u64,
    },
    StartMaintenanceExternal {
        session_id: u64,
        executable: String,
        expected_name: String,
        arguments: Vec<String>,
        current_directory: String,
    },
    InspectMaintenanceServices {
        request: u64,
        build_directory: String,
        prserv_host: Option<String>,
        hashserve: Option<String>,
        hashserve_upstream: Option<String>,
        signature_handler: Option<String>,
        executable_search_path: Vec<String>,
        process_root: String,
    },
    BitBakeLifecycle {
        operation: BitBakeOperation,
        confirmation: Option<ConfirmationLease>,
    },
    CreatePty {
        name: String,
        kind: PtyKind,
        cwd: String,
        command: PtyCommand,
        dimensions: TerminalDimensions,
    },
    RenamePty {
        session_id: PtySessionId,
        name: String,
    },
    TerminatePty {
        session_id: PtySessionId,
        force: bool,
        confirmation: Option<ConfirmationLease>,
    },
    TakePtyControl {
        session_id: PtySessionId,
        expected_epoch: u64,
    },
    ReleasePtyControl {
        session_id: PtySessionId,
        expected_epoch: u64,
    },
    PrepareShutdown,
    ConfirmShutdown {
        confirmation: ConfirmationLease,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonDevtoolOperation {
    Modify { recipe: String },
    UpdateRecipe { recipe: String },
    Finish { recipe: String, destination: String },
    DeployTarget { recipe: String, target: String },
    UndeployTarget { recipe: String, target: String },
    Reset { recipe: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSdkContext {
    pub build_directory: String,
    pub sdk_deploy_root: String,
    pub workspace_roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSdkArtifactIdentity {
    pub path: String,
    pub size_bytes: u64,
    pub modified_unix_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonSdkNativeMode {
    FindSysroot,
    RunNative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonSdkOperation {
    Publish {
        executable: String,
        artifact: DaemonSdkArtifactIdentity,
        destination: String,
    },
    Native {
        executable: String,
        mode: DaemonSdkNativeMode,
        extracted_root: Option<String>,
        recipe: String,
        tool: Option<String>,
        arguments: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonQemuRequest {
    pub machine: String,
    pub image_machine: String,
    pub image: String,
    pub image_path: String,
    pub artifact_kind: String,
    pub kernel: Option<String>,
    pub rootfs: Option<String>,
    pub networking: String,
    pub display: String,
    pub serial: String,
    pub memory_mib: u32,
    pub extra_arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonWicCreateRequest {
    pub machine: String,
    pub image: String,
    pub kickstart_name: String,
    pub kickstart_path: Option<String>,
    pub output_directory: String,
    pub generate_bmap: bool,
    pub compression: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestSelftestRequest {
    pub executable: String,
    pub family: String,
    pub selector: Option<String>,
    pub parallelism: u16,
    pub verbose: bool,
    pub skip_network: bool,
}

pub const MAX_TEST_RESULT_RECORDS: usize = 4096;
pub const MAX_TEST_RESULT_LIMITATIONS: usize = 256;
pub const MAX_QA_RECORDS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestResultRecord {
    pub identity: String,
    pub outcome: String,
    pub duration_ms: Option<u64>,
    pub log_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonQaSnapshot {
    pub generation: u64,
    pub capability: String,
    pub task_bindings: Vec<String>,
    pub reports: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonQaCapabilityInput {
    pub generation: u64,
    pub build_directory: String,
    pub source_directory: Option<String>,
    pub layer_directories: Vec<String>,
    pub recipe_names: Vec<String>,
    pub report_roots: Vec<String>,
    pub selected_recipe_name: String,
    pub selected_recipe_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonQaCapabilityRequest {
    pub request_id: RequestId,
    pub input: DaemonQaCapabilityInput,
}

impl DaemonQaCapabilityInput {
    pub fn bounded(mut self) -> Self {
        self.layer_directories.truncate(MAX_QA_RECORDS);
        self.recipe_names.truncate(MAX_QA_RECORDS);
        self.report_roots.truncate(MAX_QA_RECORDS);
        self
    }
}

impl DaemonQaSnapshot {
    pub fn bounded(mut self) -> Self {
        self.task_bindings.truncate(MAX_QA_RECORDS);
        self.reports.truncate(MAX_QA_RECORDS);
        self.limitations.truncate(MAX_TEST_RESULT_LIMITATIONS);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestResultSnapshot {
    pub generation: u64,
    pub records: Vec<DaemonTestResultRecord>,
    pub limitations: Vec<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestComparisonDiff {
    pub generation: u64,
    pub baseline: String,
    pub candidate: String,
    pub transitions: Vec<DaemonTestComparisonTransition>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonTestResultToolCapability {
    NotInspected,
    Missing,
    Available { executable: String },
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestComparisonTransition {
    pub identity: String,
    pub baseline: Option<String>,
    pub candidate: Option<String>,
    pub category: String,
}

impl DaemonTestComparisonDiff {
    pub fn bounded(mut self) -> Self {
        self.transitions.truncate(MAX_TEST_RESULT_RECORDS);
        self.limitations.truncate(MAX_TEST_RESULT_LIMITATIONS);
        self
    }
}

#[cfg(test)]
mod daemon_test_snapshot_tests {
    use super::*;
    #[test]
    fn daemon_test_snapshot_is_bounded_and_round_trips() {
        let snapshot = DaemonTestResultSnapshot {
            generation: 4,
            records: (0..(MAX_TEST_RESULT_RECORDS + 2))
                .map(|index| DaemonTestResultRecord {
                    identity: index.to_string(),
                    outcome: "pass".into(),
                    duration_ms: None,
                    log_path: None,
                })
                .collect(),
            limitations: (0..(MAX_TEST_RESULT_LIMITATIONS + 2))
                .map(|index| index.to_string())
                .collect(),
            complete: true,
        }
        .bounded();
        assert_eq!(snapshot.records.len(), MAX_TEST_RESULT_RECORDS);
        assert_eq!(snapshot.limitations.len(), MAX_TEST_RESULT_LIMITATIONS);
        let encoded = serde_json::to_vec(&snapshot).unwrap();
        let decoded: DaemonTestResultSnapshot = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, snapshot);
    }

    #[test]
    fn daemon_test_compare_diff_is_bounded_and_round_trips() {
        let diff = DaemonTestComparisonDiff {
            generation: 2,
            baseline: "a".into(),
            candidate: "b".into(),
            transitions: Vec::new(),
            limitations: vec!["limited".into()],
        }
        .bounded();
        let bytes = serde_json::to_vec(&diff).unwrap();
        assert_eq!(
            serde_json::from_slice::<DaemonTestComparisonDiff>(&bytes).unwrap(),
            diff
        );
    }

    #[test]
    fn daemon_qa_snapshot_is_bounded() {
        let snapshot = DaemonQaSnapshot {
            generation: 1,
            capability: "available".into(),
            task_bindings: (0..MAX_QA_RECORDS + 1).map(|i| i.to_string()).collect(),
            reports: (0..MAX_QA_RECORDS + 1).map(|i| i.to_string()).collect(),
            limitations: Vec::new(),
        }
        .bounded();
        assert_eq!(snapshot.task_bindings.len(), MAX_QA_RECORDS);
        assert_eq!(snapshot.reports.len(), MAX_QA_RECORDS);
    }

    #[test]
    fn daemon_qa_input_is_bounded() {
        let input = DaemonQaCapabilityInput {
            generation: 1,
            build_directory: "/build".into(),
            source_directory: None,
            layer_directories: (0..MAX_QA_RECORDS + 1).map(|i| i.to_string()).collect(),
            recipe_names: Vec::new(),
            report_roots: Vec::new(),
            selected_recipe_name: "recipe".into(),
            selected_recipe_file: "/build/recipe.bb".into(),
        }
        .bounded();
        assert_eq!(input.layer_directories.len(), MAX_QA_RECORDS);
    }
}

impl DaemonTestResultSnapshot {
    pub fn bounded(mut self) -> Self {
        self.records.truncate(MAX_TEST_RESULT_RECORDS);
        self.limitations.truncate(MAX_TEST_RESULT_LIMITATIONS);
        self
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSnapshot {
    pub daemon_instance_id: DaemonInstanceId,
    pub sequence: u64,
    pub generation: u64,
    pub workspace: Option<WorkspaceIdentity>,
    pub project_profile: ProjectProfileSummary,
    pub bitbake: BitBakeState,
    pub jobs: Vec<JobSummary>,
    pub pty_sessions: Vec<PtySessionSummary>,
    pub clients: Vec<ClientSummary>,
    pub recent_logs: Vec<LogRecord>,
    pub recovery_warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequencedEvent {
    pub sequence: u64,
    pub generation: u64,
    pub event: DaemonEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonEvent {
    BitBakeChanged(BitBakeState),
    JobChanged(JobSummary),
    JobRemoved {
        job_id: JobId,
    },
    PtyChanged(PtySessionSummary),
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
    TestResults(DaemonTestResultSnapshot),
    TestComparison(DaemonTestComparisonDiff),
    TestResultTool(DaemonTestResultToolCapability),
    QaSnapshot(DaemonQaSnapshot),
    QaCapability(DaemonQaSnapshot),
    SecuritySnapshot(DaemonSecuritySnapshot),
    MaintenanceSnapshot(DaemonMaintenanceSnapshot),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyScreenSnapshot {
    pub session_id: PtySessionId,
    pub dimensions: TerminalDimensions,
    pub cursor_column: u16,
    pub cursor_row: u16,
    pub rows: Vec<String>,
    pub scrollback_lines: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogRecord {
    pub source: String,
    pub severity: LogSeverity,
    pub message: String,
    pub unix_ms: u64,
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
            recent_logs: 10_000,
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

/// Single-owner snapshot/event journal. Calling `synchronize` while holding the
/// daemon state's lock establishes the snapshot/replay watermark before a
/// client is added to the live subscriber set.
#[derive(Debug, Clone)]
pub struct DaemonSnapshotJournal {
    snapshot: DaemonSnapshot,
    events: VecDeque<SequencedEvent>,
    limits: DaemonSnapshotLimits,
}

impl DaemonSnapshotJournal {
    pub fn new(
        snapshot: DaemonSnapshot,
        limits: DaemonSnapshotLimits,
    ) -> Result<Self, DaemonSnapshotError> {
        let limits = limits.validate()?;
        ensure_snapshot_bound(&snapshot, limits.snapshot_bytes)?;
        Ok(Self {
            snapshot,
            events: VecDeque::new(),
            limits,
        })
    }

    pub fn snapshot(&self) -> &DaemonSnapshot {
        &self.snapshot
    }

    pub fn publish(&mut self, event: DaemonEvent) -> Result<SequencedEvent, DaemonSnapshotError> {
        let sequence = self
            .snapshot
            .sequence
            .checked_add(1)
            .ok_or(DaemonSnapshotError::SequenceExhausted)?;
        let generation = self
            .snapshot
            .generation
            .checked_add(1)
            .ok_or(DaemonSnapshotError::GenerationExhausted)?;
        let sequenced = SequencedEvent {
            sequence,
            generation,
            event,
        };
        let event_bytes = serde_json::to_vec(&sequenced)?.len();
        if event_bytes > MAX_FRAME_BYTES {
            return Err(DaemonSnapshotError::EventTooLarge {
                actual: event_bytes,
                maximum: MAX_FRAME_BYTES,
            });
        }
        let mut candidate = self.snapshot.clone();
        apply_sequenced_event(&mut candidate, &sequenced)?;
        while candidate.recent_logs.len() > self.limits.recent_logs {
            candidate.recent_logs.remove(0);
        }
        ensure_snapshot_bound(&candidate, self.limits.snapshot_bytes)?;
        self.snapshot = candidate;
        self.events.push_back(sequenced.clone());
        while self.events.len() > self.limits.retained_events {
            self.events.pop_front();
        }
        Ok(sequenced)
    }

    pub fn synchronize(&self, resume: Option<ResumeCursor>) -> DaemonSnapshotSync {
        let Some(cursor) = resume else {
            return DaemonSnapshotSync::Replace {
                snapshot: Box::new(self.snapshot.clone()),
                reason: SnapshotReplacementReason::InitialAttach,
            };
        };
        if cursor.daemon_instance_id != self.snapshot.daemon_instance_id {
            return DaemonSnapshotSync::Replace {
                snapshot: Box::new(self.snapshot.clone()),
                reason: SnapshotReplacementReason::DaemonInstanceChanged,
            };
        }
        if cursor.last_sequence > self.snapshot.sequence {
            return DaemonSnapshotSync::Replace {
                snapshot: Box::new(self.snapshot.clone()),
                reason: SnapshotReplacementReason::CursorAhead,
            };
        }
        let first_retained = self
            .events
            .front()
            .map(|event| event.sequence)
            .unwrap_or_else(|| self.snapshot.sequence.saturating_add(1));
        if cursor.last_sequence.saturating_add(1) < first_retained {
            return DaemonSnapshotSync::Replace {
                snapshot: Box::new(self.snapshot.clone()),
                reason: SnapshotReplacementReason::HistoryExpired,
            };
        }
        DaemonSnapshotSync::Replay {
            events: self
                .events
                .iter()
                .filter(|event| event.sequence > cursor.last_sequence)
                .cloned()
                .collect(),
            replayed_through: self.snapshot.sequence,
        }
    }
}

pub fn apply_sequenced_event(
    snapshot: &mut DaemonSnapshot,
    sequenced: &SequencedEvent,
) -> Result<(), DaemonSnapshotError> {
    let expected_sequence = snapshot
        .sequence
        .checked_add(1)
        .ok_or(DaemonSnapshotError::SequenceExhausted)?;
    let expected_generation = snapshot
        .generation
        .checked_add(1)
        .ok_or(DaemonSnapshotError::GenerationExhausted)?;
    if sequenced.sequence != expected_sequence || sequenced.generation != expected_generation {
        return Err(DaemonSnapshotError::EventGap {
            expected_sequence,
            actual_sequence: sequenced.sequence,
            expected_generation,
            actual_generation: sequenced.generation,
        });
    }
    match &sequenced.event {
        DaemonEvent::BitBakeChanged(bitbake) => snapshot.bitbake = bitbake.clone(),
        DaemonEvent::JobChanged(job) => replace_by(&mut snapshot.jobs, job.clone(), |item| item.id),
        DaemonEvent::JobRemoved { job_id } => snapshot.jobs.retain(|job| job.id != *job_id),
        DaemonEvent::PtyChanged(pty) => {
            replace_by(&mut snapshot.pty_sessions, pty.clone(), |item| item.id);
        }
        DaemonEvent::PtyOutput { .. }
        | DaemonEvent::PtyScreen(_)
        | DaemonEvent::TestResults(_)
        | DaemonEvent::TestComparison(_)
        | DaemonEvent::TestResultTool(_)
        | DaemonEvent::QaSnapshot(_)
        | DaemonEvent::QaCapability(_)
        | DaemonEvent::SecuritySnapshot(_)
        | DaemonEvent::MaintenanceSnapshot(_)
        | DaemonEvent::Unknown => {}
        DaemonEvent::ClientChanged(client) => {
            replace_by(&mut snapshot.clients, client.clone(), |item| item.id);
        }
        DaemonEvent::ClientRemoved { client_id } => {
            snapshot.clients.retain(|client| client.id != *client_id);
        }
        DaemonEvent::RecoveryWarning { message } => {
            snapshot.recovery_warnings.push(message.clone());
        }
        DaemonEvent::Log(record) => snapshot.recent_logs.push(record.clone()),
    }
    snapshot.sequence = sequenced.sequence;
    snapshot.generation = sequenced.generation;
    Ok(())
}

fn replace_by<T, K: PartialEq>(items: &mut Vec<T>, replacement: T, key: impl Fn(&T) -> K) {
    let replacement_key = key(&replacement);
    if let Some(index) = items.iter().position(|item| key(item) == replacement_key) {
        items[index] = replacement;
    } else {
        items.push(replacement);
    }
}

fn ensure_snapshot_bound(
    snapshot: &DaemonSnapshot,
    maximum_bytes: usize,
) -> Result<(), DaemonSnapshotError> {
    let encoded = serde_json::to_vec(snapshot)?;
    if encoded.len() > maximum_bytes {
        return Err(DaemonSnapshotError::SnapshotTooLarge {
            actual: encoded.len(),
            maximum: maximum_bytes,
        });
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum DaemonSnapshotError {
    #[error("invalid daemon snapshot limit for {0}")]
    InvalidLimit(&'static str),
    #[error("daemon snapshot is {actual} bytes, exceeding the {maximum}-byte limit")]
    SnapshotTooLarge { actual: usize, maximum: usize },
    #[error("daemon event is {actual} bytes, exceeding the {maximum}-byte limit")]
    EventTooLarge { actual: usize, maximum: usize },
    #[error("daemon snapshot sequence space is exhausted")]
    SequenceExhausted,
    #[error("daemon snapshot generation space is exhausted")]
    GenerationExhausted,
    #[error(
        "daemon event gap: expected sequence/generation {expected_sequence}/{expected_generation}, got {actual_sequence}/{actual_generation}"
    )]
    EventGap {
        expected_sequence: u64,
        actual_sequence: u64,
        expected_generation: u64,
        actual_generation: u64,
    },
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogSeverity {
    Trace,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    pub request_id: RequestId,
    pub outcome: CommandOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandOutcome {
    Accepted,
    Completed,
    ConfirmationRequired {
        confirmation: ConfirmationLease,
        affected_jobs: Vec<JobId>,
        affected_ptys: Vec<PtySessionId>,
    },
    Rejected {
        code: ProtocolErrorCode,
        message: String,
        current_generation: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolFailure {
    pub request_id: Option<RequestId>,
    pub code: ProtocolErrorCode,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolErrorCode {
    IncompatibleVersion,
    UnsupportedCapability,
    AuthenticationFailed,
    MalformedMessage,
    MessageTooLarge,
    LimitExceeded,
    Timeout,
    StaleClient,
    StaleGeneration,
    Conflict,
    NotFound,
    NotWriter,
    ConfirmationRequired,
    ConfirmationExpired,
    Internal,
}

#[derive(Debug, Error)]
pub enum DaemonProtocolError {
    #[error("daemon frame exceeds {MAX_FRAME_BYTES} byte limit")]
    TooLarge,
    #[error("daemon frame has an invalid length prefix")]
    InvalidLength,
    #[error("invalid daemon JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no compatible daemon protocol version")]
    IncompatibleVersion,
    #[error("too many capabilities")]
    TooManyCapabilities,
}

pub fn negotiate_version(
    minimum: ProtocolVersion,
    maximum: ProtocolVersion,
    daemon: ProtocolVersion,
) -> Result<ProtocolVersion, DaemonProtocolError> {
    if minimum.major != maximum.major
        || daemon.major != minimum.major
        || minimum.minor > maximum.minor
        || daemon.minor < minimum.minor
    {
        return Err(DaemonProtocolError::IncompatibleVersion);
    }
    Ok(ProtocolVersion {
        major: daemon.major,
        minor: daemon.minor.min(maximum.minor),
    })
}

pub fn negotiate_capabilities(
    client: &[Capability],
    daemon: &[Capability],
) -> Result<Vec<Capability>, DaemonProtocolError> {
    if client.len() > MAX_CAPABILITIES || daemon.len() > MAX_CAPABILITIES {
        return Err(DaemonProtocolError::TooManyCapabilities);
    }
    let mut common = client
        .iter()
        .copied()
        .filter(|capability| daemon.contains(capability))
        .collect::<Vec<_>>();
    common.sort();
    common.dedup();
    Ok(common)
}

pub fn encode_frame<T: Serialize>(message: &T) -> Result<Vec<u8>, DaemonProtocolError> {
    let payload = serde_json::to_vec(message)?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(DaemonProtocolError::TooLarge);
    }
    let length = u32::try_from(payload.len()).map_err(|_| DaemonProtocolError::TooLarge)?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_frame<T: for<'de> Deserialize<'de>>(frame: &[u8]) -> Result<T, DaemonProtocolError> {
    if frame.len() < 4 {
        return Err(DaemonProtocolError::InvalidLength);
    }
    let length = u32::from_be_bytes(frame[..4].try_into().expect("four-byte prefix")) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(DaemonProtocolError::TooLarge);
    }
    if frame.len() != length + 4 {
        return Err(DaemonProtocolError::InvalidLength);
    }
    Ok(serde_json::from_slice(&frame[4..])?)
}

#[derive(Debug, Default)]
pub struct FrameDecoder {
    pending: Vec<u8>,
}

impl FrameDecoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, DaemonProtocolError> {
        self.pending.extend_from_slice(bytes);
        let mut frames = Vec::new();
        loop {
            if self.pending.len() < 4 {
                break;
            }
            let length = u32::from_be_bytes(
                self.pending[..4]
                    .try_into()
                    .expect("four-byte length prefix"),
            ) as usize;
            if length > MAX_FRAME_BYTES {
                self.pending.clear();
                return Err(DaemonProtocolError::TooLarge);
            }
            let frame_length = 4 + length;
            if self.pending.len() < frame_length {
                break;
            }
            frames.push(self.pending.drain(..frame_length).collect());
        }
        if self.pending.len() > MAX_FRAME_BYTES + 4 {
            self.pending.clear();
            return Err(DaemonProtocolError::TooLarge);
        }
        Ok(frames)
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client_id(byte: u8) -> ClientId {
        ClientId([byte; 16])
    }

    fn daemon_snapshot_fixture() -> DaemonSnapshot {
        DaemonSnapshot {
            daemon_instance_id: DaemonInstanceId([7; 16]),
            sequence: 0,
            generation: 0,
            workspace: None,
            project_profile: ProjectProfileSummary::Absent,
            bitbake: BitBakeState {
                lifecycle: LifecycleState::Disconnected,
                version: None,
                capabilities: Vec::new(),
                diagnostic: None,
            },
            jobs: Vec::new(),
            pty_sessions: Vec::new(),
            clients: Vec::new(),
            recent_logs: Vec::new(),
            recovery_warnings: Vec::new(),
        }
    }

    #[test]
    fn daemon_protocol_negotiates_versions_and_capabilities_explicitly() {
        assert_eq!(
            negotiate_version(
                ProtocolVersion { major: 1, minor: 0 },
                ProtocolVersion { major: 1, minor: 4 },
                ProtocolVersion { major: 1, minor: 2 },
            )
            .unwrap(),
            ProtocolVersion { major: 1, minor: 2 }
        );
        assert!(matches!(
            negotiate_version(
                ProtocolVersion { major: 2, minor: 0 },
                ProtocolVersion { major: 2, minor: 0 },
                ProtocolVersion::CURRENT,
            ),
            Err(DaemonProtocolError::IncompatibleVersion)
        ));
        assert_eq!(
            negotiate_capabilities(
                &[Capability::PtySessions, Capability::StateSnapshots],
                &[Capability::StateSnapshots, Capability::BackgroundJobs],
            )
            .unwrap(),
            vec![Capability::StateSnapshots]
        );

        let future: Capability = serde_json::from_str("\"future_capability\"").unwrap();
        assert_eq!(future, Capability::Unknown);
        let future_event: DaemonEvent =
            serde_json::from_str(r#"{"type":"future_optional_event"}"#).unwrap();
        assert_eq!(future_event, DaemonEvent::Unknown);
    }

    #[test]
    fn daemon_protocol_round_trips_snapshot_event_and_correlated_command() {
        let snapshot = DaemonSnapshot {
            daemon_instance_id: DaemonInstanceId([7; 16]),
            sequence: 42,
            generation: 9,
            workspace: None,
            project_profile: ProjectProfileSummary::Absent,
            bitbake: BitBakeState {
                lifecycle: LifecycleState::Running,
                version: Some("2.8.1".into()),
                capabilities: vec![BitBakeCapability::WorkspaceInspection],
                diagnostic: None,
            },
            jobs: Vec::new(),
            pty_sessions: Vec::new(),
            clients: vec![ClientSummary {
                id: client_id(1),
                name: "ssh-client".into(),
                attached_unix_ms: 1,
                last_seen_unix_ms: 2,
            }],
            recent_logs: Vec::new(),
            recovery_warnings: Vec::new(),
        };
        let message = ServerMessage::Attached {
            snapshot,
            replayed_through: 42,
        };
        assert_eq!(
            decode_frame::<ServerMessage>(&encode_frame(&message).unwrap()).unwrap(),
            message
        );

        let command = ClientMessage::Command(CommandRequest {
            request_id: RequestId(81),
            expected_generation: Some(9),
            command: DaemonCommand::TakePtyControl {
                session_id: PtySessionId(3),
                expected_epoch: 0,
            },
        });
        assert_eq!(
            decode_frame::<ClientMessage>(&encode_frame(&command).unwrap()).unwrap(),
            command
        );
    }

    #[test]
    fn daemon_protocol_frames_partial_messages_and_rejects_oversize() {
        let first = encode_frame(&ClientMessage::Detach).unwrap();
        let second = encode_frame(&ClientMessage::Pong { nonce: 4 }).unwrap();
        let mut decoder = FrameDecoder::default();
        assert!(decoder.push(&first[..3]).unwrap().is_empty());
        let mut remainder = first[3..].to_vec();
        remainder.extend_from_slice(&second);
        let frames = decoder.push(&remainder).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(
            decode_frame::<ClientMessage>(&frames[0]).unwrap(),
            ClientMessage::Detach
        );
        assert_eq!(
            decode_frame::<ClientMessage>(&frames[1]).unwrap(),
            ClientMessage::Pong { nonce: 4 }
        );
        assert_eq!(decoder.pending_len(), 0);

        let mut oversized = FrameDecoder::default();
        assert!(matches!(
            oversized.push(&((MAX_FRAME_BYTES as u32 + 1).to_be_bytes())),
            Err(DaemonProtocolError::TooLarge)
        ));
    }

    #[test]
    fn daemon_protocol_covers_reconnect_stale_writer_layout_and_mouse() {
        let attach = ClientMessage::Attach {
            workspace: None,
            subscription: Subscription {
                state: true,
                jobs: true,
                logs: false,
                pty_sessions: vec![PtySessionId(8)],
            },
            resume: Some(ResumeCursor {
                daemon_instance_id: DaemonInstanceId([9; 16]),
                last_sequence: 77,
            }),
        };
        assert_eq!(
            decode_frame::<ClientMessage>(&encode_frame(&attach).unwrap()).unwrap(),
            attach
        );

        let stale = ServerMessage::CommandResult(CommandResult {
            request_id: RequestId(5),
            outcome: CommandOutcome::Rejected {
                code: ProtocolErrorCode::StaleGeneration,
                message: "snapshot replaced".into(),
                current_generation: 12,
            },
        });
        assert_eq!(
            decode_frame::<ServerMessage>(&encode_frame(&stale).unwrap()).unwrap(),
            stale
        );

        for message in [
            ClientMessage::Layout {
                event: ClientLayoutEvent::AttachSession {
                    pane_id: PaneId(2),
                    session_id: PtySessionId(8),
                },
            },
            ClientMessage::Mouse {
                event: ServerMouseEvent {
                    session_id: PtySessionId(8),
                    writer_epoch: 3,
                    kind: MouseEventKind::Drag,
                    button: 1,
                    column: 40,
                    row: 12,
                    modifiers: 0,
                },
            },
        ] {
            assert_eq!(
                decode_frame::<ClientMessage>(&encode_frame(&message).unwrap()).unwrap(),
                message
            );
        }
    }

    #[test]
    fn multi_client_state_keeps_identity_global_and_layout_local() {
        let mut snapshot = daemon_snapshot_fixture();
        snapshot.clients = vec![
            ClientSummary {
                id: ClientId([1; 16]),
                name: "left".into(),
                attached_unix_ms: 1,
                last_seen_unix_ms: 2,
            },
            ClientSummary {
                id: ClientId([2; 16]),
                name: "right".into(),
                attached_unix_ms: 1,
                last_seen_unix_ms: 2,
            },
        ];
        let event = DaemonEvent::ClientChanged(snapshot.clients[1].clone());
        let sequenced = SequencedEvent {
            sequence: 1,
            generation: 1,
            event,
        };
        apply_sequenced_event(&mut snapshot, &sequenced).unwrap();
        assert_eq!(snapshot.clients.len(), 2);
        assert_ne!(snapshot.clients[0].id, snapshot.clients[1].id);
    }

    #[test]
    fn multi_client_fanout_replays_global_events_from_independent_cursors() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        journal
            .publish(DaemonEvent::RecoveryWarning {
                message: "shared".into(),
            })
            .unwrap();
        let cursor = ResumeCursor {
            daemon_instance_id: journal.snapshot().daemon_instance_id,
            last_sequence: 0,
        };
        let first = journal.synchronize(Some(cursor));
        let second = journal.synchronize(Some(cursor));
        let events = |sync| match sync {
            DaemonSnapshotSync::Replay { events, .. } => events,
            _ => panic!("expected replay"),
        };
        assert_eq!(events(first), events(second));
    }

    #[test]
    fn daemon_snapshot_is_gap_free_bounded_and_replays_only_retained_events() {
        let mut journal = DaemonSnapshotJournal::new(
            daemon_snapshot_fixture(),
            DaemonSnapshotLimits {
                retained_events: 2,
                recent_logs: 2,
                snapshot_bytes: MAX_FRAME_BYTES,
            },
        )
        .unwrap();
        for index in 1..=3 {
            let event = journal
                .publish(DaemonEvent::Log(LogRecord {
                    source: "test".into(),
                    severity: LogSeverity::Info,
                    message: format!("event-{index}"),
                    unix_ms: index,
                }))
                .unwrap();
            assert_eq!(event.sequence, index);
            assert_eq!(event.generation, index);
        }
        assert_eq!(journal.snapshot().sequence, 3);
        assert_eq!(journal.snapshot().recent_logs.len(), 2);
        assert_eq!(journal.snapshot().recent_logs[0].message, "event-2");

        let replay = journal.synchronize(Some(ResumeCursor {
            daemon_instance_id: DaemonInstanceId([7; 16]),
            last_sequence: 1,
        }));
        assert!(matches!(
            replay,
            DaemonSnapshotSync::Replay {
                ref events,
                replayed_through: 3
            } if events.iter().map(|event| event.sequence).collect::<Vec<_>>() == vec![2, 3]
        ));
        assert!(matches!(
            journal.synchronize(Some(ResumeCursor {
                daemon_instance_id: DaemonInstanceId([7; 16]),
                last_sequence: 0,
            })),
            DaemonSnapshotSync::Replace {
                reason: SnapshotReplacementReason::HistoryExpired,
                ..
            }
        ));
        assert!(matches!(
            journal.synchronize(Some(ResumeCursor {
                daemon_instance_id: DaemonInstanceId([8; 16]),
                last_sequence: 3,
            })),
            DaemonSnapshotSync::Replace {
                reason: SnapshotReplacementReason::DaemonInstanceChanged,
                ..
            }
        ));

        let mut client = daemon_snapshot_fixture();
        let gap = SequencedEvent {
            sequence: 2,
            generation: 2,
            event: DaemonEvent::Unknown,
        };
        assert!(matches!(
            apply_sequenced_event(&mut client, &gap),
            Err(DaemonSnapshotError::EventGap { .. })
        ));
        assert_eq!(client.sequence, 0);
    }

    #[test]
    fn daemon_snapshot_rejects_invalid_limits_and_oversized_snapshots() {
        assert!(matches!(
            DaemonSnapshotJournal::new(
                daemon_snapshot_fixture(),
                DaemonSnapshotLimits {
                    retained_events: 0,
                    ..DaemonSnapshotLimits::default()
                },
            ),
            Err(DaemonSnapshotError::InvalidLimit("retained events"))
        ));
        assert!(matches!(
            DaemonSnapshotJournal::new(
                daemon_snapshot_fixture(),
                DaemonSnapshotLimits {
                    snapshot_bytes: 1,
                    ..DaemonSnapshotLimits::default()
                },
            ),
            Err(DaemonSnapshotError::SnapshotTooLarge { .. })
        ));

        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        assert!(matches!(
            journal.publish(DaemonEvent::Log(LogRecord {
                source: "test".into(),
                severity: LogSeverity::Error,
                message: "x".repeat(MAX_FRAME_BYTES),
                unix_ms: 0,
            })),
            Err(DaemonSnapshotError::EventTooLarge { .. })
        ));
        assert_eq!(journal.snapshot().sequence, 0);
    }

    #[test]
    fn resource_limits_are_explicit_and_bounded() {
        const {
            assert!(MAX_DAEMON_CLIENTS < u16::MAX as usize);
            assert!(MAX_DAEMON_PTY_SESSIONS < u16::MAX as usize);
            assert!(MAX_TERMINAL_SCROLLBACK_LINES <= MAX_SNAPSHOT_LOGS);
            assert!(MAX_PTY_OUTPUT_EVENT_BYTES <= MAX_FRAME_BYTES);
            assert!(MAX_UTILITY_OUTPUT_BYTES <= MAX_FRAME_BYTES);
        }
        let limits = ProtocolLimits {
            maximum_frame_bytes: MAX_FRAME_BYTES as u32,
            maximum_snapshot_bytes: MAX_FRAME_BYTES as u32,
            maximum_pending_requests: 64,
            maximum_queue_depth: 256,
            maximum_terminal_rows: 512,
            maximum_terminal_columns: 512,
            maximum_clients: MAX_DAEMON_CLIENTS as u16,
            maximum_pty_sessions: MAX_DAEMON_PTY_SESSIONS as u16,
            maximum_scrollback_lines: MAX_TERMINAL_SCROLLBACK_LINES as u32,
            maximum_utility_output_bytes: MAX_UTILITY_OUTPUT_BYTES as u32,
        };
        let round_trip: ProtocolLimits =
            serde_json::from_slice(&serde_json::to_vec(&limits).unwrap()).unwrap();
        assert_eq!(round_trip, limits);
    }
}
