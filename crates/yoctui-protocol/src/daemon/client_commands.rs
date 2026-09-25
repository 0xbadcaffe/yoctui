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
    PtyViewport(PtyViewport),
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
    InspectRootfsSources {
        query: crate::rootfs::RootfsSourcesRequestData,
    },
    StartBuild {
        targets: Vec<String>,
        task: Option<String>,
        force: bool,
    },
    CancelJob {
        job_id: JobId,
    },
    StartRaw {
        request: RawExecutionRequestData,
    },
    StartRawPty {
        request: RawExecutionRequestData,
        dimensions: TerminalDimensions,
    },
    CancelRaw {
        request_id: String,
    },
    SetRawAttachment {
        request_id: String,
        attached: bool,
    },
    StartDevtool {
        operation: DaemonDevtoolOperation,
        build_directory: String,
    },
    InspectDevtoolStatus {
        recipe: String,
        recipe_file: String,
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
    ClosePty {
        session_id: PtySessionId,
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
    UpdateRecipePatch { recipe: String, destination: String },
    Finish { recipe: String, destination: String },
    DeployTarget { recipe: String, target: String },
    UndeployTarget { recipe: String, target: String },
    Reset { recipe: String },
    Upgrade { recipe: String },
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
