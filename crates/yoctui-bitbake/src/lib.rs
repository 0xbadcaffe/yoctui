//! BitBake adapters. They execute BitBake; they never evaluate metadata themselves.
mod bitbake_cli_control;
mod bitbake_restart;
#[cfg(unix)]
mod bitbake_socket;
mod build_environment;
mod compatibility_api;
mod compatibility_cache;
mod compatibility_command;
mod compatibility_devtool;
#[cfg(any(test, feature = "test-fixtures"))]
mod compatibility_fixtures;
mod compatibility_layers;
mod compatibility_probe;
mod compatibility_recipetool;
mod compatibility_resolver;
mod compatibility_version;
mod image;
mod maintenance_optional;
mod maintenance_release;
mod maintenance_service;
mod maintenance_sstate;
mod package;
#[cfg(unix)]
mod pty_runner;
mod qa_layer;
mod qa_report;
mod qa_task;
mod qemu;
mod raw_job;
mod sdk;
mod sdk_shell;
mod sdk_tool;
mod security;
mod security_mapper;
mod security_report;
mod server_controller;
mod signature;
mod test_results;
mod test_runner;
mod utility;
mod wic;

#[cfg(test)]
mod test_support {
    use std::{
        fs,
        path::Path,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_EXECUTABLE: AtomicU64 = AtomicU64::new(1);

    pub(crate) fn write_executable(path: &Path, body: &str) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let temporary = path.with_extension(format!(
                "yoctui-fixture-write-{}-{}",
                std::process::id(),
                NEXT_EXECUTABLE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::write(&temporary, body).unwrap();
            let mut permissions = fs::metadata(&temporary).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&temporary, permissions).unwrap();
            fs::rename(temporary, path).unwrap();
        }
        #[cfg(not(unix))]
        fs::write(path, body).unwrap();
    }
}

use async_trait::async_trait;
pub use bitbake_cli_control::{
    BitBakeCliCommand, BitBakeCliControlError, BitBakeCliOperation, BitBakeCliOutcome,
    BitBakeCliPreview, BitBakeCliRunner,
};
pub use bitbake_restart::{
    BitBakeMetadataRefresher, BitBakeRestartCoordinator, BitBakeRestartError,
    BitBakeRestartMetadata,
};
#[cfg(unix)]
pub use bitbake_socket::BitBakeSocketAdapter;
pub use build_environment::{
    BuildEnvironmentAdapter, BuildEnvironmentAdapterError, BuildEnvironmentClonePreview,
    BuildEnvironmentResponse,
};
pub use compatibility_api::{
    BitBakeApiAuthority, BitBakeApiCompatibilityError, BitBakeApiOperation,
};
pub use compatibility_cache::{
    CapabilityCacheError, CapabilityCacheSelection, CapabilityFingerprintMaterial,
    CapabilitySnapshotCache,
};
pub use compatibility_command::{
    AuthorizedBitBakeCommand, BITBAKE_DIFFSIGS_ARGV_IMPLEMENTATION,
    BITBAKE_DUMPSIG_ARGV_IMPLEMENTATION, BITBAKE_GETVAR_ENVIRONMENT_IMPLEMENTATION,
    BITBAKE_GETVAR_UTILITY_IMPLEMENTATION, BitBakeCommandAuthorizationError, BitBakeCommandPlanner,
    BitBakeServerCommandOperation,
};
pub use compatibility_devtool::{
    DEVTOOL_DEPLOY_TARGET_IMPLEMENTATION, DEVTOOL_EDIT_RECIPE_IMPLEMENTATION,
    DEVTOOL_FINISH_IMPLEMENTATION, DEVTOOL_MODIFY_IMPLEMENTATION, DEVTOOL_RESET_IMPLEMENTATION,
    DEVTOOL_STATUS_IMPLEMENTATION, DEVTOOL_UNDEPLOY_TARGET_IMPLEMENTATION,
    DEVTOOL_UPDATE_RECIPE_IMPLEMENTATION, DEVTOOL_UPGRADE_IMPLEMENTATION, DevtoolCommandPlanner,
    DevtoolCompatibilityError,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use compatibility_fixtures::{
    CompatibilityFixtureRole, FixtureCapabilityExpectation, FixtureCapabilityState,
    ReleaseCapabilityFixture, fixture_implementation, fixture_state, release_capability_fixtures,
};
pub use compatibility_layers::{
    BITBAKE_LAYERS_ADD_IMPLEMENTATION, BITBAKE_LAYERS_CREATE_ADD_IMPLEMENTATION,
    BITBAKE_LAYERS_CREATE_IMPLEMENTATION, BITBAKE_LAYERS_REMOVE_IMPLEMENTATION,
    BITBAKE_LAYERS_SHOW_IMPLEMENTATION, BitBakeLayersCommandPlanner, BitBakeLayersCommandSpec,
    BitBakeLayersCompatibilityError,
};
pub use compatibility_probe::{
    CapabilityProbeContext, CapabilityProbeContextError, CapabilityProbeObservation,
    CapabilityProbeRunner, CapabilityProbeStatus,
};
pub use compatibility_recipetool::{
    RECIPETOOL_APPEND_FILE_IMPLEMENTATION, RECIPETOOL_CREATE_IMPLEMENTATION,
    RECIPETOOL_CREATE_OUTFILE_IMPLEMENTATION, RecipetoolCommandPlanner, RecipetoolCommandSpec,
    RecipetoolCompatibilityError,
};
pub use compatibility_resolver::{
    CapabilityResolver, ResolvedCapability, ResolvedCapabilitySnapshot,
};
pub use compatibility_version::{
    CorrelatedVersion, VersionFallbackMap, VersionFallbackResolution, VersionParseError,
};
pub use image::{
    ImageArtifactAdapter, ImageArtifactAdapterError, ImageArtifactCancellation,
    ImageArtifactResponse,
};
pub use maintenance_optional::{
    MaintenanceDirectoryIdentity, MaintenanceGitWorktreeIdentity, MaintenanceOptionalAdapterError,
    MaintenanceOptionalCapabilityInput, MaintenanceOptionalCapabilityInspector,
    MaintenanceOptionalInspection, OptionalErrorReportIntegration, OptionalIntegrationState,
    OptionalPullRequestIntegration, OptionalRepoManifestIntegration, OptionalToasterIntegration,
};
pub use maintenance_release::{
    GitArchiveLocalResult, MaintenanceReleaseAdapterError, MaintenanceReleaseCapabilityInput,
    MaintenanceReleaseCapabilityInspector, MaintenanceReleaseEvidenceSnapshot,
    build_compare_command, buildhistory_command, git_archive_local_command,
    git_archive_push_command, locked_signature_command,
};
pub use maintenance_service::{
    MaintenanceEndpointObservation, MaintenanceServiceAdapterError,
    MaintenanceServiceCapabilityInput, MaintenanceServiceCapabilityInspector,
    MaintenanceServiceInspection, pr_service_command,
};
pub use maintenance_sstate::{
    MaintenanceSstateAdapterError, MaintenanceSstateCapabilityInput,
    MaintenanceSstateCapabilityInspector, MaintenanceSstateCommandKind,
    MaintenanceSstateCommandSpec, MaintenanceSstateJobRunner, MaintenanceSstateRunnerEvent,
    parse_cleanup_preview,
};
pub use package::{
    PKGDATA_LIST_PACKAGE_FILES_IMPLEMENTATION, PKGDATA_LIST_PACKAGES_IMPLEMENTATION,
    PKGDATA_PACKAGE_INFO_IMPLEMENTATION, PKGDATA_READ_VALUE_IMPLEMENTATION, PackageDataAdapter,
    PackageDataAdapterError, PackageDataCancellation, PackageDataCommandSpec,
    PackageDetailResponse, PackageInventoryResponse,
};
#[cfg(unix)]
pub use pty_runner::{PtyRunner, PtyRunnerError, PtyRunnerEvent};
pub use qa_layer::{
    QaConfiguredLayerInput, QaLayerAdapterError, QaLayerCapabilityInput,
    QaLayerCapabilityInspector, QaLayerCapabilityResponse, QaLayerCommandSpec, QaLayerJobRunner,
    QaLayerRunnerEvent,
};
pub use qa_report::{
    QaReportAdapter, QaReportAdapterError, QaReportCancellation, QaReportCandidate, QaReportOrigin,
    QaReportResponse, QaReportScanInput, QaReportScanOutcome,
};
pub use qa_task::{
    QaFamilyTaskBinding, QaReportRootInput, QaTaskCapabilityError, QaTaskCapabilityInput,
    QaTaskCapabilityInspector, QaTaskCapabilityResponse, QaTaskScopeInput,
};
pub use qemu::{QemuAdapterError, QemuCapabilityInspector, QemuCommandSpec, QemuJobRunner};
pub use raw_job::{
    RawJobCommandSpec, RawJobPlanner, RawJobPlannerError, RawJobRunner, RawJobRunnerError,
    RawJobRunnerEvent, RawPtyCommandSpec, RawPtyPlanner,
};
pub use sdk::{
    SdkArtifactAdapter, SdkArtifactAdapterError, SdkArtifactCancellation, SdkArtifactResponse,
    SdkArtifactScanOutcome,
};
pub use sdk_shell::{SdkShellAdapter, SdkShellEnvironment, SdkShellError, SdkShellPreview};
pub use sdk_tool::{
    SdkToolAdapter, SdkToolAdapterError, SdkToolCapabilityInspector, SdkToolCommandSpec,
    SdkToolJobRunner, SdkToolRunnerEvent,
};
pub use security::{SecurityCapabilityError, SecurityCapabilityInput, SecurityCapabilityInspector};
pub use security_mapper::{
    SecurityMapperAdapterError, SecurityMapperCommandSpec, SecurityMapperJobRunner,
    SecurityMapperRunnerEvent,
};
pub use security_report::{
    SecurityReportAdapter, SecurityReportAdapterError, SecurityReportCancellation,
    SecurityReportResponse, SecurityReportScanOutcome,
};
pub use server_controller::{
    BitBakeDetection, BitBakeServerAdapter, BitBakeServerAdapterError, BitBakeServerCapability,
    BitBakeServerContext, BitBakeServerController, BitBakeServerControllerError,
    BitBakeServerControllerState, BitBakeServerEndpoint, BitBakeServerLifecycle,
    BitBakeServerObservation, BitBakeServerOperation, BitBakeServerSession,
};
pub use signature::{
    SignatureAdapter, SignatureAdapterError, SignatureCancellation, SignatureCommandSpec,
    SignatureComparisonResponse, SignatureDumpResponse,
};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
pub use test_results::{
    ResultToolCapabilityInspector, TestResultAdapter, TestResultAdapterError,
    TestResultCommandSpec, TestResultImportResponse, TestResultJob, TestResultOperation,
    TestResultRunnerEvent,
};
pub use test_runner::{
    TestCommandSpec, TestRunnerAdapter, TestRunnerAdapterError, TestRunnerCapabilityInspector,
    TestRunnerEvent, TestRunnerJob,
};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command as TokioCommand},
};
pub use utility::{
    UtilityCommandSpec, UtilityCompatibilityAuthority, UtilityCompatibilityError, UtilityRisk,
    parse_utility_arguments,
};
pub use wic::{
    WicAdapterError, WicCapabilityInspector, WicCreateCommandSpec, WicDeviceInspector,
    WicDeviceInventoryResponse, WicJobRunner, WicWriteCommandSpec,
};
use yoctui_model::{
    BuildRequest, DependencyEdge, DependencyEdgeKind, DependencyGraph, DependencyNode,
    DependencyNodeId, DevtoolCapability, DevtoolGitState, DevtoolOperation, DevtoolStatus,
    DevtoolStatusError, DevtoolWorkspace, ImageArtifactInventory, ImageArtifactRequest, Layer,
    LogEntry, PackageDetail, PackageDetailRequest, PackageInventoryRequest, PackageSummary, Recipe,
    RecipeBuildStatus, RecipeIdentity, RecipeMetadata, RecipeWorkspaceStatus, RootfsComposition,
    RootfsCompositionRequest, Severity, SignatureComparisonRequest, SignatureDifference,
    SignatureRecord, SignatureTarget, TaskStats, VariableOperation, Workspace,
};
use yoctui_protocol::{
    Command, DependencyEdgeData, DependencyEdgeKindData, DependencyGraphData, DependencyNodeData,
    DependencyNodeIdData, Envelope, Event, LayerData, LayerRelationshipData, MAX_LINE_BYTES,
    ProtocolError, RecipeBuildStatusData, RecipeData, RecipeWorkspaceStatusData, TaskStatsData,
    VERSION, decode_line, encode_line,
};

const MAX_PROCESS_LINE_BYTES: usize = 1024 * 1024;
const MAX_BRIDGE_STDERR_BYTES: usize = 16 * 1024;
const MAX_DEPENDENCY_GRAPH_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_DEPENDENCY_NODES: usize = 2_000;
const MAX_DEPENDENCY_EDGES: usize = 4_000;
const DEPENDENCY_GRAPH_TIMEOUT: Duration = Duration::from_secs(120);

async fn read_output<R>(stream: R, sender: tokio::sync::mpsc::Sender<LogEntry>)
where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut bytes = Vec::new();
    let mut discarding = false;
    while let Ok(buffer) = reader.fill_buf().await {
        if buffer.is_empty() {
            if !bytes.is_empty()
                && !discarding
                && sender
                    .send(classify_output(output_text(&bytes)))
                    .await
                    .is_err()
            {
                break;
            }
            break;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        if !discarding {
            if bytes.len() + take > MAX_PROCESS_LINE_BYTES {
                let mut message = output_text(&bytes);
                message.push_str(" [line truncated]");
                if sender.send(classify_output(message)).await.is_err() {
                    break;
                }
                bytes.clear();
                discarding = true;
            } else {
                bytes.extend_from_slice(&buffer[..take]);
            }
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if !discarding
                && sender
                    .send(classify_output(output_text(&bytes)))
                    .await
                    .is_err()
            {
                break;
            }
            bytes.clear();
            discarding = false;
        }
    }
}

pub fn output_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches(['\r', '\n'])
        .into()
}
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("process: {0}")]
    Process(#[from] std::io::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("bridge: {0}")]
    Bridge(String),
    #[error("signature: {0}")]
    Signature(#[from] SignatureAdapterError),
    #[error("compatibility API: {0}")]
    CompatibilityApi(#[from] BitBakeApiCompatibilityError),
    #[error("backend is not running")]
    NotRunning,
}

#[derive(Debug, Clone)]
pub struct DevtoolInspector {
    devtool_program: Option<PathBuf>,
    git_program: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
    capability_generation: u64,
    capability: yoctui_model::CapabilityId,
    build_directory: PathBuf,
}
impl DevtoolCommandSpec {
    pub fn from_operation(
        operation: &DevtoolOperation,
        compatibility: &yoctui_model::DaemonCompatibilitySnapshot,
        expected_generation: u64,
        build_directory: &Path,
    ) -> Result<Self, DevtoolCompatibilityError> {
        let executable = compatibility
            .snapshot
            .environment
            .available_tools
            .value()
            .and_then(|tools| tools.iter().find(|tool| tool.id == "devtool"))
            .map(|tool| tool.executable.clone())
            .ok_or(DevtoolCompatibilityError::ToolIdentityUnknown)?;
        Self::with_executable(
            executable,
            operation,
            compatibility,
            expected_generation,
            build_directory,
        )
    }

    pub fn with_executable(
        executable: PathBuf,
        operation: &DevtoolOperation,
        compatibility: &yoctui_model::DaemonCompatibilitySnapshot,
        expected_generation: u64,
        build_directory: &Path,
    ) -> Result<Self, DevtoolCompatibilityError> {
        DevtoolCommandPlanner::new(
            compatibility,
            expected_generation,
            build_directory,
            &executable,
        )?
        .operation(operation)
    }

    pub(crate) fn from_authorized_parts(
        executable: PathBuf,
        arguments: Vec<OsString>,
        capability_generation: u64,
        capability: yoctui_model::CapabilityId,
        build_directory: PathBuf,
    ) -> Self {
        Self {
            executable,
            arguments,
            capability_generation,
            capability,
            build_directory,
        }
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn capability_generation(&self) -> u64 {
        self.capability_generation
    }

    pub fn capability(&self) -> yoctui_model::CapabilityId {
        self.capability
    }
}

const MAX_DEVTOOL_LINE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevtoolOutputStream {
    Stdout,
    Stderr,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevtoolRunnerEvent {
    Started,
    Output {
        stream: DevtoolOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        exit_code: Option<i32>,
    },
    Failed {
        exit_code: Option<i32>,
    },
    Cancelled {
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QemuRunnerOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QemuRunnerEvent {
    Starting,
    Started,
    Output {
        stream: QemuRunnerOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        exit_code: i32,
    },
    Failed {
        message: String,
        exit_code: Option<i32>,
    },
    Cancelled {
        forced: bool,
        exit_code: Option<i32>,
    },
    CancellationRejected {
        message: String,
    },
    Lost {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WicRunnerOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WicRunnerEvent {
    Starting,
    Started,
    Output {
        stream: WicRunnerOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        exit_code: i32,
        outputs: Vec<yoctui_model::WicOutput>,
        limitations: Vec<String>,
    },
    Failed {
        message: String,
        exit_code: Option<i32>,
    },
    Cancelled {
        forced: bool,
        exit_code: Option<i32>,
    },
    CancellationRejected {
        message: String,
    },
    Lost {
        message: String,
    },
}
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DevtoolRunnerError {
    #[error("a Devtool process or unconsumed terminal event is already active")]
    Busy,
    #[error("Devtool executable is missing: {0}")]
    MissingExecutable(PathBuf),
    #[error("could not start Devtool: {0}")]
    Spawn(String),
    #[error("Devtool process stream is unavailable: {0:?}")]
    StreamUnavailable(DevtoolOutputStream),
    #[error("Devtool runner is not active")]
    NotRunning,
    #[error("Devtool process control failed: {0}")]
    ProcessControl(String),
    #[error(
        "Devtool command authorization belongs to another capability generation or build directory"
    )]
    AuthorizationMismatch,
}
#[derive(Debug)]
enum DevtoolPipeEvent {
    Output {
        stream: DevtoolOutputStream,
        line: String,
        truncated: bool,
    },
    Failed {
        stream: DevtoolOutputStream,
        message: String,
    },
}

async fn read_devtool_output<R>(
    stream: R,
    kind: DevtoolOutputStream,
    sender: tokio::sync::mpsc::Sender<DevtoolPipeEvent>,
) where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut bytes = Vec::new();
    let mut truncated = false;
    loop {
        let buffer = match reader.fill_buf().await {
            Ok(buffer) => buffer,
            Err(error) => {
                let _ = sender
                    .send(DevtoolPipeEvent::Failed {
                        stream: kind,
                        message: error.to_string(),
                    })
                    .await;
                break;
            }
        };
        if buffer.is_empty() {
            if (!bytes.is_empty() || truncated)
                && sender
                    .send(DevtoolPipeEvent::Output {
                        stream: kind,
                        line: output_text(&bytes),
                        truncated,
                    })
                    .await
                    .is_err()
            {
                break;
            }
            break;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        if !truncated {
            let remaining = MAX_DEVTOOL_LINE_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(DevtoolPipeEvent::Output {
                    stream: kind,
                    line: output_text(&bytes),
                    truncated,
                })
                .await
                .is_err()
            {
                break;
            }
            bytes.clear();
            truncated = false;
        }
    }
}

pub struct DevtoolJobRunner {
    build_dir: PathBuf,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<DevtoolPipeEvent>>,
    streams_drained: bool,
    started_pending: bool,
    terminal_pending: Option<DevtoolRunnerEvent>,
    cancellation_timeout: Duration,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}
impl DevtoolJobRunner {
    pub fn new(build_dir: PathBuf) -> Self {
        Self {
            build_dir,
            child: None,
            output: None,
            streams_drained: true,
            started_pending: false,
            terminal_pending: None,
            cancellation_timeout: Duration::from_secs(5),
            cancellation_requested: false,
            #[cfg(unix)]
            process_group: None,
        }
    }

    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }

    pub fn is_active(&self) -> bool {
        self.child.is_some()
    }

    pub async fn start(&mut self, command: DevtoolCommandSpec) -> Result<(), DevtoolRunnerError> {
        if self.child.is_some()
            || self.started_pending
            || self.terminal_pending.is_some()
            || self.output.is_some()
        {
            return Err(DevtoolRunnerError::Busy);
        }
        if !self.build_dir.is_dir() {
            return Err(DevtoolRunnerError::Spawn(format!(
                "build directory does not exist: {}",
                self.build_dir.display()
            )));
        }
        if command.capability_generation == 0 || command.build_directory != self.build_dir {
            return Err(DevtoolRunnerError::AuthorizationMismatch);
        }
        let mut process = TokioCommand::new(&command.executable);
        process
            .args(&command.arguments)
            .current_dir(&self.build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        process.process_group(0);
        let mut child = process.spawn().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                DevtoolRunnerError::MissingExecutable(command.executable.clone())
            } else {
                DevtoolRunnerError::Spawn(error.to_string())
            }
        })?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            return Err(DevtoolRunnerError::StreamUnavailable(
                DevtoolOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            return Err(DevtoolRunnerError::StreamUnavailable(
                DevtoolOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(1024);
        tokio::spawn(read_devtool_output(
            stdout,
            DevtoolOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_devtool_output(
            stderr,
            DevtoolOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.started_pending = true;
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<DevtoolRunnerEvent, DevtoolRunnerError> {
        if self.started_pending {
            self.started_pending = false;
            return Ok(DevtoolRunnerEvent::Started);
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            if let Some(child) = self.child.as_mut() {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            self.child = None;
            self.streams_drained = true;
            #[cfg(unix)]
            {
                self.process_group = None;
            }
            return Ok(DevtoolRunnerEvent::Lost {
                message: "Devtool output event channel was lost".into(),
            });
        }
        if let Some(receiver) = self.output.as_mut() {
            match receiver.recv().await {
                Some(DevtoolPipeEvent::Output {
                    stream,
                    line,
                    truncated,
                }) => {
                    return Ok(DevtoolRunnerEvent::Output {
                        stream,
                        line,
                        truncated,
                    });
                }
                Some(DevtoolPipeEvent::Failed { stream, message }) => {
                    if let Some(child) = self.child.as_mut() {
                        let _ = child.kill().await;
                        let _ = child.wait().await;
                    }
                    self.child = None;
                    self.output = None;
                    self.streams_drained = true;
                    #[cfg(unix)]
                    {
                        self.process_group = None;
                    }
                    return Ok(DevtoolRunnerEvent::Lost {
                        message: format!("{stream:?} stream failed: {message}"),
                    });
                }
                None => {
                    self.output = None;
                    self.streams_drained = true;
                }
            }
        }
        if let Some(event) = self.terminal_pending.take() {
            return Ok(event);
        }
        let Some(child) = self.child.as_mut() else {
            return Err(DevtoolRunnerError::NotRunning);
        };
        let status = match child.wait().await {
            Ok(status) => status,
            Err(error) => {
                self.child = None;
                self.cancellation_requested = false;
                #[cfg(unix)]
                {
                    self.process_group = None;
                }
                return Ok(DevtoolRunnerEvent::Lost {
                    message: format!("Devtool process wait failed: {error}"),
                });
            }
        };
        self.child = None;
        self.cancellation_requested = false;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
        if status.success() {
            Ok(DevtoolRunnerEvent::Completed {
                exit_code: status.code(),
            })
        } else {
            Ok(DevtoolRunnerEvent::Failed {
                exit_code: status.code(),
            })
        }
    }

    pub async fn cancel(&mut self) -> Result<bool, DevtoolRunnerError> {
        if self.cancellation_requested {
            return Ok(false);
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(false);
        };
        self.cancellation_requested = true;
        let mut forced = false;
        #[cfg(unix)]
        let status =
            if let Some(process_group) = self.process_group {
                // SAFETY: the group is the child PID created by `process_group(0)`, so the
                // negative PID targets only the spawned Devtool process group.
                let signal = unsafe { libc::kill(-process_group, libc::SIGTERM) };
                if signal != 0 {
                    child
                        .start_kill()
                        .map_err(|error| DevtoolRunnerError::ProcessControl(error.to_string()))?;
                    forced = true;
                }
                match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                    Ok(result) => result
                        .map_err(|error| DevtoolRunnerError::ProcessControl(error.to_string()))?,
                    Err(_) => {
                        // SAFETY: same child-owned process group as the graceful signal.
                        let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                        forced = true;
                        child.wait().await.map_err(|error| {
                            DevtoolRunnerError::ProcessControl(error.to_string())
                        })?
                    }
                }
            } else {
                forced = true;
                child
                    .kill()
                    .await
                    .map_err(|error| DevtoolRunnerError::ProcessControl(error.to_string()))?;
                child
                    .wait()
                    .await
                    .map_err(|error| DevtoolRunnerError::ProcessControl(error.to_string()))?
            };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| DevtoolRunnerError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| DevtoolRunnerError::ProcessControl(error.to_string()))?
        };
        self.child = None;
        self.cancellation_requested = false;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
        self.terminal_pending = Some(DevtoolRunnerEvent::Cancelled {
            forced,
            exit_code: status.code(),
        });
        Ok(true)
    }
}
impl Drop for DevtoolJobRunner {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(process_group) = self.process_group {
            // SAFETY: this is the child-owned process group created by `start`.
            let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }
}

impl Default for DevtoolInspector {
    fn default() -> Self {
        Self {
            devtool_program: None,
            git_program: "git".into(),
        }
    }
}

impl DevtoolInspector {
    pub fn with_programs(devtool_program: PathBuf, git_program: PathBuf) -> Self {
        Self {
            devtool_program: Some(devtool_program),
            git_program,
        }
    }

    pub async fn inspect(&self, _build_dir: &Path, identity: RecipeIdentity) -> DevtoolStatus {
        DevtoolStatus {
            identity,
            capability: DevtoolCapability::Unavailable {
                reason: "Devtool status requires the current environment capability snapshot."
                    .into(),
            },
            workspace: DevtoolWorkspace::NotMember,
            git: DevtoolGitState::NotApplicable,
            error: None,
        }
    }

    pub async fn inspect_with_compatibility(
        &self,
        build_dir: &Path,
        identity: RecipeIdentity,
        compatibility: &yoctui_model::DaemonCompatibilitySnapshot,
        expected_generation: u64,
    ) -> DevtoolStatus {
        if !identity.file.is_absolute() {
            return DevtoolStatus {
                identity,
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: DevtoolGitState::NotApplicable,
                error: Some(DevtoolStatusError::InvalidRecipeIdentity),
            };
        }

        let executable = self.devtool_program.clone().or_else(|| {
            compatibility
                .snapshot
                .environment
                .available_tools
                .value()
                .and_then(|tools| tools.iter().find(|tool| tool.id == "devtool"))
                .map(|tool| tool.executable.clone())
        });
        let command = executable
            .ok_or(DevtoolCompatibilityError::ToolIdentityUnknown)
            .and_then(|executable| {
                DevtoolCommandPlanner::new(
                    compatibility,
                    expected_generation,
                    build_dir,
                    &executable,
                )?
                .status()
            });
        let command = match command {
            Ok(command) => command,
            Err(error) => {
                return DevtoolStatus {
                    identity,
                    capability: DevtoolCapability::Unavailable {
                        reason: error.to_string(),
                    },
                    workspace: DevtoolWorkspace::NotMember,
                    git: DevtoolGitState::NotApplicable,
                    error: None,
                };
            }
        };
        let output = TokioCommand::new(command.executable())
            .args(command.arguments())
            .current_dir(build_dir)
            .output()
            .await;
        let output = match output {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return DevtoolStatus {
                    identity,
                    capability: DevtoolCapability::MissingExecutable,
                    workspace: DevtoolWorkspace::NotMember,
                    git: DevtoolGitState::NotApplicable,
                    error: None,
                };
            }
            Err(error) => {
                return DevtoolStatus {
                    identity,
                    capability: DevtoolCapability::Available,
                    workspace: DevtoolWorkspace::NotMember,
                    git: DevtoolGitState::NotApplicable,
                    error: Some(DevtoolStatusError::DevtoolFailed {
                        exit_code: None,
                        message: error.to_string(),
                    }),
                };
            }
        };
        if !output.status.success() {
            return DevtoolStatus {
                identity,
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: DevtoolGitState::NotApplicable,
                error: Some(DevtoolStatusError::DevtoolFailed {
                    exit_code: output.status.code(),
                    message: output_text(&output.stderr),
                }),
            };
        }
        let stdout = match String::from_utf8(output.stdout) {
            Ok(stdout) => stdout,
            Err(error) => {
                return DevtoolStatus {
                    identity,
                    capability: DevtoolCapability::Available,
                    workspace: DevtoolWorkspace::NotMember,
                    git: DevtoolGitState::NotApplicable,
                    error: Some(DevtoolStatusError::MalformedOutput {
                        line: error.to_string(),
                    }),
                };
            }
        };
        let entries = match parse_devtool_status(&stdout) {
            Ok(entries) => entries,
            Err(line) => {
                return DevtoolStatus {
                    identity,
                    capability: DevtoolCapability::Available,
                    workspace: DevtoolWorkspace::NotMember,
                    git: DevtoolGitState::NotApplicable,
                    error: Some(DevtoolStatusError::MalformedOutput { line }),
                };
            }
        };
        let Some((source_path, recipe_file)) = entries
            .into_iter()
            .find(|(recipe, _, _)| recipe == &identity.name)
            .map(|(_, source_path, recipe_file)| (source_path, recipe_file))
        else {
            return DevtoolStatus {
                identity,
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: DevtoolGitState::NotApplicable,
                error: None,
            };
        };
        if !source_path.is_dir() {
            return DevtoolStatus {
                identity,
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::MissingDirectory { source_path },
                git: DevtoolGitState::NotApplicable,
                error: None,
            };
        }
        let git = inspect_git(&self.git_program, &source_path).await;
        DevtoolStatus {
            identity,
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path,
                recipe_file,
            },
            git,
            error: None,
        }
    }
}

fn parse_devtool_status(output: &str) -> Result<Vec<(String, PathBuf, Option<PathBuf>)>, String> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (recipe, value) = line.split_once(": ").ok_or_else(|| line.to_owned())?;
            if recipe.is_empty() || value.is_empty() {
                return Err(line.to_owned());
            }
            let (source, recipe_file) = value
                .strip_suffix(')')
                .and_then(|value| value.rsplit_once(" ("))
                .map_or((value, None), |(source, recipe_file)| {
                    (source, Some(PathBuf::from(recipe_file)))
                });
            let source = PathBuf::from(source);
            if !source.is_absolute() {
                return Err(line.to_owned());
            }
            Ok((recipe.to_owned(), source, recipe_file))
        })
        .collect()
}

async fn inspect_git(program: &Path, source_path: &Path) -> DevtoolGitState {
    let output = TokioCommand::new(program)
        .arg("-C")
        .arg(source_path)
        .args(["status", "--porcelain=v2", "--branch"])
        .output()
        .await;
    let output = match output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return DevtoolGitState::MissingExecutable;
        }
        Err(error) => {
            return DevtoolGitState::Failed {
                exit_code: None,
                message: error.to_string(),
            };
        }
    };
    if !output.status.success() {
        let message = output_text(&output.stderr);
        if message
            .to_ascii_lowercase()
            .contains("not a git repository")
        {
            return DevtoolGitState::NotRepository;
        }
        return DevtoolGitState::Failed {
            exit_code: output.status.code(),
            message,
        };
    }
    let output = match String::from_utf8(output.stdout) {
        Ok(output) => output,
        Err(error) => {
            return DevtoolGitState::Malformed {
                message: error.to_string(),
            };
        }
    };
    parse_git_status(&output).unwrap_or_else(|message| DevtoolGitState::Malformed { message })
}

fn parse_git_status(output: &str) -> Result<DevtoolGitState, String> {
    let mut branch = None;
    let mut head = None;
    let mut modified = 0;
    let mut untracked = 0;
    let mut conflicted = 0;
    for line in output.lines().filter(|line| !line.is_empty()) {
        if let Some(value) = line.strip_prefix("# branch.head ") {
            branch = (value != "(detached)").then(|| value.to_owned());
        } else if let Some(value) = line.strip_prefix("# branch.oid ") {
            head = (value != "(initial)").then(|| value.to_owned());
        } else if line.starts_with("# branch.") {
            continue;
        } else if line.starts_with("1 ") || line.starts_with("2 ") {
            modified += 1;
        } else if line.starts_with("u ") {
            conflicted += 1;
        } else if line.starts_with("? ") {
            untracked += 1;
        } else if line.starts_with("! ") {
            continue;
        } else {
            return Err(format!("unrecognized Git status record: {line}"));
        }
    }
    Ok(DevtoolGitState::Available {
        branch,
        head,
        modified,
        untracked,
        conflicted,
    })
}
#[derive(Debug, Clone)]
pub enum BackendEvent {
    Workspace(Workspace),
    Recipes(Vec<Recipe>),
    Layers(Vec<Layer>),
    Variable {
        name: String,
        recipe: Option<String>,
        value: Option<String>,
        provenance: Option<String>,
        unexpanded_value: Option<String>,
        operations: Vec<VariableOperation>,
        active_overrides: Vec<String>,
    },
    Dependencies {
        recipe: String,
        build: Vec<String>,
        runtime: Vec<String>,
    },
    DependencyGraph {
        graph: DependencyGraph,
        limitations: Vec<String>,
    },
    DependencyGraphFailed {
        root: DependencyNodeId,
        message: String,
    },
    SignatureDump {
        target: SignatureTarget,
        records: Vec<SignatureRecord>,
        limitations: Vec<String>,
    },
    SignatureDumpFailed {
        target: SignatureTarget,
        message: String,
    },
    SignatureComparison {
        request: SignatureComparisonRequest,
        differences: Vec<SignatureDifference>,
        limitations: Vec<String>,
    },
    SignatureComparisonFailed {
        request: SignatureComparisonRequest,
        message: String,
    },
    PackageInventory {
        request: PackageInventoryRequest,
        packages: Vec<PackageSummary>,
        limitations: Vec<String>,
    },
    PackageInventoryFailed {
        request: PackageInventoryRequest,
        message: String,
    },
    PackageDetail {
        request: PackageDetailRequest,
        detail: PackageDetail,
        limitations: Vec<String>,
    },
    PackageDetailFailed {
        request: PackageDetailRequest,
        message: String,
    },
    ImageArtifacts {
        request: ImageArtifactRequest,
        inventory: ImageArtifactInventory,
        limitations: Vec<String>,
    },
    ImageArtifactsFailed {
        request: ImageArtifactRequest,
        message: String,
    },
    RootfsComposition {
        request: RootfsCompositionRequest,
        composition: RootfsComposition,
        limitations: Vec<String>,
    },
    RootfsCompositionUnavailable {
        request: RootfsCompositionRequest,
        reason: String,
    },
    RootfsCompositionFailed {
        request: RootfsCompositionRequest,
        message: String,
    },
    RecipeSources {
        recipe: String,
        paths: Vec<PathBuf>,
    },
    RecipeMetadata(RecipeMetadata),
    LayerRelationships(Vec<LayerRelationship>),
    BuildStarted,
    ParseProgress {
        current: Option<u64>,
        total: Option<u64>,
    },
    Log(LogEntry),
    TaskQueued {
        recipe: String,
        task: String,
        worker: Option<String>,
        stats: Option<TaskStats>,
    },
    TaskStarted {
        recipe: String,
        task: String,
        pid: Option<u32>,
        worker: Option<String>,
        log_path: Option<PathBuf>,
        stats: Option<TaskStats>,
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
    },
    BuildCompleted {
        success: bool,
        exit_code: Option<i32>,
    },
    CommandFailed {
        code: String,
        message: String,
    },
    Ignored,
    Disconnected,
}

impl From<SignatureDumpResponse> for BackendEvent {
    fn from(response: SignatureDumpResponse) -> Self {
        Self::SignatureDump {
            target: response.target,
            records: response.records,
            limitations: response.limitations,
        }
    }
}

impl From<SignatureComparisonResponse> for BackendEvent {
    fn from(response: SignatureComparisonResponse) -> Self {
        Self::SignatureComparison {
            request: response.request,
            differences: response.differences,
            limitations: response.limitations,
        }
    }
}

impl From<PackageInventoryResponse> for BackendEvent {
    fn from(response: PackageInventoryResponse) -> Self {
        Self::PackageInventory {
            request: response.request,
            packages: response.packages,
            limitations: response.limitations,
        }
    }
}

impl From<PackageDetailResponse> for BackendEvent {
    fn from(response: PackageDetailResponse) -> Self {
        Self::PackageDetail {
            request: response.request,
            detail: response.detail,
            limitations: response.limitations,
        }
    }
}

impl From<ImageArtifactResponse> for BackendEvent {
    fn from(response: ImageArtifactResponse) -> Self {
        Self::ImageArtifacts {
            request: response.request,
            inventory: response.inventory,
            limitations: response.limitations,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariableValue {
    pub recipe: Option<String>,
    pub value: Option<String>,
    pub provenance: Option<String>,
    pub unexpanded_value: Option<String>,
    pub operations: Vec<VariableOperation>,
    pub active_overrides: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecipeDependencies {
    pub build: Vec<String>,
    pub runtime: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyGraphResponse {
    pub graph: DependencyGraph,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayerRelationship {
    pub name: String,
    pub priority: Option<i32>,
    pub compatible: Vec<String>,
    pub depends: Vec<String>,
    pub overlays: Vec<String>,
    pub appends: Vec<String>,
}

#[async_trait]
pub trait BitBakeBackend: Send {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError>;
    async fn list_recipes(&mut self, filter: Option<String>) -> Result<Vec<Recipe>, BackendError>;
    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError>;
    async fn get_variable(
        &mut self,
        name: String,
        recipe: Option<String>,
    ) -> Result<VariableValue, BackendError>;
    async fn get_dependencies(
        &mut self,
        recipe: String,
    ) -> Result<RecipeDependencies, BackendError>;
    async fn get_dependency_graph(
        &mut self,
        recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError>;
    async fn get_signature_dump(
        &mut self,
        target: SignatureTarget,
    ) -> Result<SignatureDumpResponse, BackendError>;
    async fn compare_signatures(
        &mut self,
        request: SignatureComparisonRequest,
    ) -> Result<SignatureComparisonResponse, BackendError>;
    async fn get_recipe_sources(&mut self, recipe: String) -> Result<Vec<PathBuf>, BackendError>;
    async fn get_recipe_metadata(&mut self, recipe: String)
    -> Result<RecipeMetadata, BackendError>;
    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError>;
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError>;
    async fn cancel_build(&mut self) -> Result<(), BackendError>;
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError>;
    async fn shutdown(&mut self) -> Result<(), BackendError>;
}
pub fn strip_ansi(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for x in chars.by_ref() {
                if x.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c)
        }
    }
    out
}
pub fn classify_output(line: String) -> LogEntry {
    let clean = strip_ansi(&line);
    let lower = clean.to_ascii_lowercase();
    let severity = if lower.contains("error:") || lower.starts_with("error") {
        Severity::Error
    } else if lower.contains("warning:") || lower.starts_with("warning") {
        Severity::Warning
    } else {
        Severity::Info
    };
    LogEntry {
        id: 0,
        severity,
        message: clean,
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::now(),
        build: None,
        protected: false,
        diagnostic: None,
    }
}
pub struct ProcessBackend {
    build_dir: PathBuf,
    signature_adapter: SignatureAdapter,
    executable: PathBuf,
    arguments: Vec<OsString>,
    environment: Vec<(OsString, OsString)>,
    compatibility: Option<yoctui_model::DaemonCompatibilitySnapshot>,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<LogEntry>>,
    build_started_pending: bool,
    cancellation_timeout: Duration,
    #[cfg(unix)]
    process_group: Option<i32>,
}
impl ProcessBackend {
    pub fn new(build_dir: PathBuf) -> Self {
        Self::with_executable(build_dir, PathBuf::from("bitbake"))
    }

    pub fn with_executable(build_dir: PathBuf, executable: PathBuf) -> Self {
        Self::with_command(build_dir, executable, Vec::new())
    }

    pub fn with_command(build_dir: PathBuf, executable: PathBuf, arguments: Vec<OsString>) -> Self {
        Self {
            signature_adapter: SignatureAdapter::new(build_dir.clone()),
            build_dir,
            executable,
            arguments,
            environment: Vec::new(),
            compatibility: None,
            child: None,
            output: None,
            build_started_pending: false,
            cancellation_timeout: Duration::from_secs(5),
            #[cfg(unix)]
            process_group: None,
        }
    }
    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }
    pub fn with_environment(mut self, environment: BTreeMap<String, String>) -> Self {
        self.environment = environment
            .into_iter()
            .map(|(key, value)| (OsString::from(key), OsString::from(value)))
            .collect();
        self
    }
    pub fn with_compatibility(
        mut self,
        compatibility: yoctui_model::DaemonCompatibilitySnapshot,
    ) -> Result<Self, BackendError> {
        let compatibility = compatibility
            .normalize()
            .map_err(|error| BackendError::Bridge(error.to_string()))?;
        self.signature_adapter = self
            .signature_adapter
            .with_compatibility(compatibility.clone())?;
        self.compatibility = Some(compatibility);
        Ok(self)
    }
    fn command_planner(&self) -> Result<BitBakeCommandPlanner<'_>, BackendError> {
        let compatibility = self.compatibility.as_ref().ok_or_else(|| {
            BackendError::Bridge(
                "BitBake command is unavailable until the daemon supplies an authoritative capability snapshot"
                    .into(),
            )
        })?;
        BitBakeCommandPlanner::new(
            compatibility,
            compatibility.snapshot.generation,
            &self.build_dir,
        )
        .map_err(|error| BackendError::Bridge(error.to_string()))
    }
    async fn collect(&mut self) -> Result<(bool, Option<i32>), BackendError> {
        let child = self.child.as_mut().ok_or(BackendError::NotRunning)?;
        let status = child.wait().await?;
        Ok((status.success(), status.code()))
    }

    async fn generate_dependency_graph(
        &self,
        recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError> {
        if self.child.is_some() {
            return Err(BackendError::Bridge(
                "dependency graph generation is unavailable during an active build".into(),
            ));
        }
        BuildRequest {
            targets: vec![recipe.clone()],
            task: None,
            force: false,
        }
        .validate()
        .map_err(|error| BackendError::Bridge(error.to_string()))?;

        let graph_path = self.build_dir.join("task-depends.dot");
        match tokio::fs::remove_file(&graph_path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        let authorized = self
            .command_planner()?
            .dependency_graph(&recipe)
            .map_err(|error| BackendError::Bridge(error.to_string()))?;
        if authorized.executable != self.executable {
            return Err(BackendError::Bridge(format!(
                "configured BitBake executable {} does not match capability-authorized executable {}",
                self.executable.display(),
                authorized.executable.display()
            )));
        }
        let mut command = TokioCommand::new(&authorized.executable);
        command.envs(self.environment.iter().map(|(key, value)| (key, value)));
        command
            .args(&self.arguments)
            .args(&authorized.arguments)
            .current_dir(&self.build_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn()?;
        let status = match tokio::time::timeout(DEPENDENCY_GRAPH_TIMEOUT, child.wait()).await {
            Ok(status) => status?,
            Err(_) => {
                let _ = child.kill().await;
                return Err(BackendError::Bridge(
                    "BitBake dependency graph generation timed out after 120 seconds".into(),
                ));
            }
        };
        if !status.success() {
            return Err(BackendError::Bridge(format!(
                "BitBake dependency graph generation exited with {}",
                status
                    .code()
                    .map_or_else(|| "no exit code".into(), |code| code.to_string())
            )));
        }

        let metadata = tokio::fs::symlink_metadata(&graph_path).await?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(BackendError::Bridge(
                "BitBake dependency graph output is not a regular file".into(),
            ));
        }
        if metadata.len() > MAX_DEPENDENCY_GRAPH_FILE_BYTES {
            return Err(BackendError::Bridge(format!(
                "BitBake dependency graph exceeds the {} byte limit",
                MAX_DEPENDENCY_GRAPH_FILE_BYTES
            )));
        }
        let canonical_build_dir = tokio::fs::canonicalize(&self.build_dir).await?;
        let canonical_graph = tokio::fs::canonicalize(&graph_path).await?;
        if canonical_graph.parent() != Some(canonical_build_dir.as_path()) {
            return Err(BackendError::Bridge(
                "BitBake dependency graph output escaped the build directory".into(),
            ));
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        tokio::fs::File::open(&canonical_graph)
            .await?
            .take(MAX_DEPENDENCY_GRAPH_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .await?;
        if bytes.len() as u64 > MAX_DEPENDENCY_GRAPH_FILE_BYTES {
            return Err(BackendError::Bridge(
                "BitBake dependency graph grew beyond its byte limit while reading".into(),
            ));
        }
        parse_task_dependency_dot(&recipe, &bytes)
    }
}
#[async_trait]
impl BitBakeBackend for ProcessBackend {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError> {
        Ok(Workspace {
            build_dir: Some(self.build_dir.clone()),
            ..Workspace::default()
        })
    }
    async fn list_recipes(&mut self, _filter: Option<String>) -> Result<Vec<Recipe>, BackendError> {
        Ok(Vec::new())
    }
    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError> {
        Ok(Vec::new())
    }
    async fn get_variable(
        &mut self,
        _name: String,
        _recipe: Option<String>,
    ) -> Result<VariableValue, BackendError> {
        Ok(VariableValue::default())
    }
    async fn get_dependencies(
        &mut self,
        _recipe: String,
    ) -> Result<RecipeDependencies, BackendError> {
        Err(BackendError::Bridge(
            "the process backend cannot inspect authoritative recipe dependencies; use the Yoctui bridge"
                .into(),
        ))
    }
    async fn get_dependency_graph(
        &mut self,
        recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError> {
        self.generate_dependency_graph(recipe).await
    }
    async fn get_signature_dump(
        &mut self,
        target: SignatureTarget,
    ) -> Result<SignatureDumpResponse, BackendError> {
        if self.child.is_some() {
            return Err(BackendError::Bridge(
                "signature inspection is unavailable during an active process-backend build".into(),
            ));
        }
        self.signature_adapter
            .dump(target)
            .await
            .map_err(Into::into)
    }
    async fn compare_signatures(
        &mut self,
        request: SignatureComparisonRequest,
    ) -> Result<SignatureComparisonResponse, BackendError> {
        if self.child.is_some() {
            return Err(BackendError::Bridge(
                "signature comparison is unavailable during an active process-backend build".into(),
            ));
        }
        self.signature_adapter
            .compare(request)
            .await
            .map_err(Into::into)
    }
    async fn get_recipe_sources(&mut self, _recipe: String) -> Result<Vec<PathBuf>, BackendError> {
        Err(BackendError::Bridge("the process backend cannot inspect authoritative recipe source paths; use the Yoctui bridge".into()))
    }
    async fn get_recipe_metadata(
        &mut self,
        _recipe: String,
    ) -> Result<RecipeMetadata, BackendError> {
        Err(BackendError::Bridge(
            "the process backend cannot inspect authoritative recipe metadata; use the Yoctui bridge"
                .into(),
        ))
    }
    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError> {
        Err(BackendError::Bridge("the process backend cannot inspect authoritative layer relationships; use the Yoctui bridge".into()))
    }
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError> {
        request
            .validate()
            .map_err(|e| BackendError::Bridge(e.to_string()))?;
        let authorized = self
            .command_planner()?
            .build(&request)
            .map_err(|error| BackendError::Bridge(error.to_string()))?;
        if authorized.executable != self.executable {
            return Err(BackendError::Bridge(format!(
                "configured BitBake executable {} does not match capability-authorized executable {}",
                self.executable.display(),
                authorized.executable.display()
            )));
        }
        let mut cmd = TokioCommand::new(&authorized.executable);
        cmd.args(&self.arguments);
        cmd.envs(self.environment.iter().map(|(key, value)| (key, value)));
        cmd.args(&authorized.arguments)
            .current_dir(&self.build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        cmd.process_group(0);
        let mut child = cmd.spawn()?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let stdout = child
            .stdout
            .take()
            .ok_or(BackendError::Bridge("stdout unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or(BackendError::Bridge("stderr unavailable".into()))?;
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        tokio::spawn(read_output(stdout, tx.clone()));
        tokio::spawn(read_output(stderr, tx.clone()));
        drop(tx);
        self.child = Some(child);
        self.output = Some(rx);
        self.build_started_pending = true;
        Ok(())
    }
    async fn cancel_build(&mut self) -> Result<(), BackendError> {
        let c = self.child.as_mut().ok_or(BackendError::NotRunning)?;
        #[cfg(unix)]
        if let Some(process_group) = self.process_group {
            // SAFETY: process_group comes from the child PID after `process_group(0)`, and a
            // negative PID targets only that child process group, never the caller's group.
            let result = unsafe { libc::kill(-process_group, libc::SIGTERM) };
            if result == 0
                && tokio::time::timeout(self.cancellation_timeout, c.wait())
                    .await
                    .is_ok()
            {
                return Ok(());
            }
            // SAFETY: same process-group identity and scope as the graceful signal above.
            let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
        }
        c.kill().await?;
        let _ = c.wait().await?;
        Ok(())
    }
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {
        if self.build_started_pending {
            self.build_started_pending = false;
            return Ok(BackendEvent::BuildStarted);
        }
        if let Some(output) = self.output.as_mut()
            && let Some(line) = output.recv().await
        {
            return Ok(BackendEvent::Log(line));
        }
        let (success, exit_code) = self.collect().await?;
        Ok(BackendEvent::BuildCompleted { success, exit_code })
    }

    async fn shutdown(&mut self) -> Result<(), BackendError> {
        if let Some(child) = self.child.as_mut()
            && child.try_wait()?.is_none()
        {
            self.cancel_build().await?;
        }
        Ok(())
    }
}
pub struct BridgeBackend {
    child: Child,
    stdin: ChildStdin,
    lines: BufReader<tokio::process::ChildStdout>,
    sequence: u64,
    last_sequence: u64,
    accepted_correlations: VecDeque<String>,
    signature_adapter: SignatureAdapter,
    api_authority: Option<BitBakeApiAuthority>,
    stderr_tail: Arc<Mutex<BridgeStderrTail>>,
    stderr_task: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Default)]
struct BridgeStderrTail {
    bytes: VecDeque<u8>,
    truncated: bool,
}

impl BridgeStderrTail {
    fn push(&mut self, bytes: &[u8]) {
        if bytes.len() >= MAX_BRIDGE_STDERR_BYTES {
            self.bytes.clear();
            self.bytes.extend(
                bytes[bytes.len().saturating_sub(MAX_BRIDGE_STDERR_BYTES)..]
                    .iter()
                    .copied(),
            );
            self.truncated = true;
            return;
        }
        let overflow = self
            .bytes
            .len()
            .saturating_add(bytes.len())
            .saturating_sub(MAX_BRIDGE_STDERR_BYTES);
        if overflow > 0 {
            self.bytes.drain(..overflow);
            self.truncated = true;
        }
        self.bytes.extend(bytes.iter().copied());
    }

    fn diagnostic(&self) -> Option<String> {
        if self.bytes.is_empty() {
            return None;
        }
        let bytes = self.bytes.iter().copied().collect::<Vec<_>>();
        let text = String::from_utf8_lossy(&bytes);
        let mut output = String::new();
        if self.truncated {
            output.push_str("[earlier bridge stderr truncated]\n");
        }
        for line in text.lines() {
            let normalized = line.to_ascii_lowercase();
            if [
                "password",
                "passwd",
                "secret",
                "token",
                "credential",
                "api_key",
            ]
            .iter()
            .any(|word| normalized.contains(word))
            {
                output.push_str("[redacted sensitive diagnostic]");
            } else {
                output.extend(line.chars().map(|character| {
                    if character.is_control() && character != '\t' {
                        '�'
                    } else {
                        character
                    }
                }));
            }
            output.push('\n');
        }
        let output = output.trim().to_owned();
        (!output.is_empty()).then_some(output)
    }
}

async fn drain_bridge_stderr<R>(mut stderr: R, tail: Arc<Mutex<BridgeStderrTail>>)
where
    R: AsyncRead + Unpin,
{
    let mut buffer = [0_u8; 4096];
    loop {
        let count = match stderr.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(count) => count,
        };
        if let Ok(mut tail) = tail.lock() {
            tail.push(&buffer[..count]);
        }
    }
}

const BUNDLED_BRIDGE_SOURCE: &str = include_str!("../bridge/yoctui_bridge.py");

impl BridgeBackend {
    pub async fn spawn_bundled(python: &str, build_dir: PathBuf) -> Result<Self, BackendError> {
        Self::spawn_bundled_with_environment(python, build_dir, BTreeMap::new()).await
    }

    pub async fn spawn_bundled_with_environment(
        python: &str,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
    ) -> Result<Self, BackendError> {
        let mut command = TokioCommand::new(python);
        command.arg("-c").arg(BUNDLED_BRIDGE_SOURCE);
        Self::spawn_command(command, build_dir, environment, None).await
    }

    pub async fn spawn_bundled_with_compatibility(
        python: &str,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
        compatibility: yoctui_model::DaemonCompatibilitySnapshot,
        expected_generation: u64,
    ) -> Result<Self, BackendError> {
        let mut command = TokioCommand::new(python);
        command.arg("-c").arg(BUNDLED_BRIDGE_SOURCE);
        let authority = BitBakeApiAuthority::new(compatibility, expected_generation, &build_dir)?;
        Self::spawn_command(command, build_dir, environment, Some(authority)).await
    }

    pub async fn spawn(
        python: &str,
        script: PathBuf,
        build_dir: PathBuf,
    ) -> Result<Self, BackendError> {
        Self::spawn_with_environment(python, script, build_dir, BTreeMap::new()).await
    }
    pub async fn spawn_with_environment(
        python: &str,
        script: PathBuf,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
    ) -> Result<Self, BackendError> {
        let mut command = TokioCommand::new(python);
        command.arg(script);
        Self::spawn_command(command, build_dir, environment, None).await
    }

    pub async fn spawn_with_compatibility(
        python: &str,
        script: PathBuf,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
        compatibility: yoctui_model::DaemonCompatibilitySnapshot,
        expected_generation: u64,
    ) -> Result<Self, BackendError> {
        let mut command = TokioCommand::new(python);
        command.arg(script);
        let authority = BitBakeApiAuthority::new(compatibility, expected_generation, &build_dir)?;
        Self::spawn_command(command, build_dir, environment, Some(authority)).await
    }

    async fn spawn_command(
        mut command: TokioCommand,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
        api_authority: Option<BitBakeApiAuthority>,
    ) -> Result<Self, BackendError> {
        let mut child = command
            .current_dir(&build_dir)
            .envs(environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| BackendError::Bridge("bridge stdin unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BackendError::Bridge("bridge stdout unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| BackendError::Bridge("bridge stderr unavailable".into()))?;
        let stderr_tail = Arc::new(Mutex::new(BridgeStderrTail::default()));
        let stderr_task = tokio::spawn(drain_bridge_stderr(stderr, Arc::clone(&stderr_tail)));
        let signature_adapter = if let Some(authority) = api_authority.as_ref() {
            SignatureAdapter::new(build_dir.clone())
                .with_compatibility(authority.compatibility_snapshot().clone())?
        } else {
            SignatureAdapter::new(build_dir.clone())
        };
        let mut backend = Self {
            child,
            stdin,
            lines: BufReader::new(stdout),
            sequence: 0,
            last_sequence: 0,
            accepted_correlations: VecDeque::new(),
            signature_adapter,
            api_authority,
            stderr_tail,
            stderr_task: Some(stderr_task),
        };
        if let Err(error) = backend.handshake().await {
            backend.stop_failed_startup().await;
            return Err(backend.with_stderr_context(error));
        }
        Ok(backend)
    }

    fn stderr_diagnostic(&self) -> Option<String> {
        self.stderr_tail.lock().ok()?.diagnostic()
    }

    fn with_stderr_context(&self, error: BackendError) -> BackendError {
        let Some(diagnostic) = self.stderr_diagnostic() else {
            return error;
        };
        BackendError::Bridge(format!("{error}; bridge stderr: {diagnostic}"))
    }

    fn disconnected(&self, context: &str) -> BackendError {
        self.with_stderr_context(BackendError::Bridge(context.into()))
    }

    async fn finish_stderr_capture(&mut self) {
        let Some(mut task) = self.stderr_task.take() else {
            return;
        };
        if tokio::time::timeout(Duration::from_secs(1), &mut task)
            .await
            .is_err()
        {
            task.abort();
        }
    }

    async fn stop_failed_startup(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.start_kill();
            let _ = self.child.wait().await;
        }
        self.finish_stderr_capture().await;
    }
    async fn command(&mut self, message: Command) -> Result<(), BackendError> {
        self.sequence += 1;
        let correlation = self.sequence.to_string();
        self.accepted_correlations.push_back(correlation.clone());
        while self.accepted_correlations.len() > 32 {
            self.accepted_correlations.pop_front();
        }
        let bytes = encode_line(&Envelope {
            protocol_version: VERSION,
            sequence: self.sequence,
            correlation_id: Some(correlation),
            message,
        })?;
        self.stdin.write_all(&bytes).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    fn validate_correlation(&self, envelope: &Envelope<Event>) -> Result<(), BackendError> {
        let Some(correlation) = envelope.correlation_id.as_ref() else {
            return Err(BackendError::Bridge(
                "bridge response omitted its command correlation".into(),
            ));
        };
        if !self.accepted_correlations.contains(correlation) {
            return Err(BackendError::Bridge(format!(
                "bridge response used unknown correlation {correlation}"
            )));
        }
        Ok(())
    }

    async fn next_line(&mut self) -> Result<Option<Vec<u8>>, BackendError> {
        let mut line = Vec::new();
        loop {
            let buffer = self.lines.fill_buf().await?;
            if buffer.is_empty() {
                return if line.is_empty() {
                    Ok(None)
                } else {
                    Err(ProtocolError::TooLarge.into())
                };
            }
            let newline = buffer.iter().position(|byte| *byte == b'\n');
            let take = newline.unwrap_or(buffer.len());
            if line.len() + take > MAX_LINE_BYTES {
                self.lines.consume(take);
                return Err(ProtocolError::TooLarge.into());
            }
            line.extend_from_slice(&buffer[..take]);
            self.lines.consume(take + usize::from(newline.is_some()));
            if newline.is_some() {
                return Ok(Some(line));
            }
        }
    }

    async fn handshake(&mut self) -> Result<(), BackendError> {
        let compatibility = self
            .api_authority
            .as_ref()
            .map(|authority| Box::new(authority.bridge_handshake()));
        self.command(Command::Hello { compatibility }).await?;
        let Some(line) = self.next_line().await? else {
            return Err(self.disconnected("bridge disconnected during protocol handshake"));
        };
        let envelope: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.validate_correlation(&envelope)?;
        self.last_sequence = envelope.sequence;
        match envelope.message {
            Event::HelloAck {
                compatibility_generation,
                capabilities,
                ..
            } => {
                if let Some(authority) = self.api_authority.as_mut() {
                    authority.accept_negotiation(compatibility_generation, &capabilities)?;
                }
                Ok(())
            }
            Event::ProtocolError { code, message } | Event::CommandFailed { code, message } => Err(
                BackendError::Bridge(format!("handshake rejected: {code}: {message}")),
            ),
            _ => Err(BackendError::Bridge(
                "bridge sent an unexpected handshake event".into(),
            )),
        }
    }

    fn require_api(&self, operation: BitBakeApiOperation) -> Result<(), BackendError> {
        self.api_authority
            .as_ref()
            .ok_or_else(|| {
                BackendError::Bridge(
                    "BitBake API operation requires a daemon capability snapshot".into(),
                )
            })?
            .require(operation)
            .map_err(Into::into)
    }

    pub fn supports_api(&self, operation: BitBakeApiOperation) -> bool {
        self.api_authority
            .as_ref()
            .is_some_and(|authority| authority.require(operation).is_ok())
    }

    /// Ask the bridge to finish its protocol work before the drop fallback kills it.
    pub async fn shutdown(&mut self) -> Result<(), BackendError> {
        self.command(Command::Shutdown).await?;
        let Some(line) = self.next_line().await? else {
            return Err(BackendError::Bridge(
                "bridge disconnected before acknowledging shutdown".into(),
            ));
        };
        let envelope: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.validate_correlation(&envelope)?;
        self.last_sequence = envelope.sequence;
        match envelope.message {
            Event::BridgeShutdown => {}
            Event::CommandFailed { code, message } | Event::ProtocolError { code, message } => {
                return Err(BackendError::Bridge(format!(
                    "shutdown rejected: {code}: {message}"
                )));
            }
            _ => {
                return Err(BackendError::Bridge(
                    "bridge sent an unexpected shutdown event".into(),
                ));
            }
        }
        tokio::time::timeout(Duration::from_secs(2), self.child.wait())
            .await
            .map_err(|_| {
                BackendError::Bridge("bridge did not exit after shutdown acknowledgement".into())
            })??;
        self.finish_stderr_capture().await;
        Ok(())
    }

    /// Terminate the connected BitBake process server through Tinfoil's
    /// supported process-server connection, then wait for the bridge to exit.
    pub async fn terminate_server(&mut self) -> Result<(), BackendError> {
        self.require_api(BitBakeApiOperation::ServerSocket)?;
        self.command(Command::TerminateServer).await?;
        let Some(line) = self.next_line().await? else {
            return Err(BackendError::Bridge(
                "bridge disconnected before acknowledging server termination".into(),
            ));
        };
        let envelope: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.validate_correlation(&envelope)?;
        self.last_sequence = envelope.sequence;
        match envelope.message {
            Event::ServerTerminated => {}
            Event::CommandFailed { code, message } | Event::ProtocolError { code, message } => {
                return Err(BackendError::Bridge(format!(
                    "server termination rejected: {code}: {message}"
                )));
            }
            _ => {
                return Err(BackendError::Bridge(
                    "bridge sent an unexpected server termination event".into(),
                ));
            }
        }
        tokio::time::timeout(Duration::from_secs(2), self.child.wait())
            .await
            .map_err(|_| {
                BackendError::Bridge(
                    "bridge did not exit after server termination acknowledgement".into(),
                )
            })??;
        self.finish_stderr_capture().await;
        Ok(())
    }
    fn event(event: Event) -> Result<BackendEvent, BackendError> {
        Ok(match event {
            Event::Workspace { data } => BackendEvent::Workspace(Workspace {
                build_dir: data.build_dir.map(PathBuf::from),
                source_dir: data.source_dir.map(PathBuf::from),
                variables: data.variables,
                variable_provenance: data.variable_provenance,
                variable_provenance_chain: data.variable_provenance_chain,
                bitbake_version: data.bitbake_version,
                release: data.release,
                layers: data
                    .layers
                    .into_iter()
                    .map(|layer| Layer {
                        name: layer.name,
                        path: PathBuf::from(layer.path),
                        priority: layer.priority,
                    })
                    .collect(),
                recipes: data
                    .recipes
                    .into_iter()
                    .map(|recipe| Recipe {
                        name: recipe.name,
                        version: recipe.version,
                        layer: recipe.layer,
                        preferred_version: recipe.preferred_version,
                        file: recipe.file.map(PathBuf::from),
                        append_count: recipe.append_count,
                    })
                    .collect(),
            }),
            Event::Recipes { recipes } => BackendEvent::Recipes(
                recipes
                    .into_iter()
                    .map(
                        |RecipeData {
                             name,
                             version,
                             layer,
                             preferred_version,
                             file,
                             append_count,
                         }| Recipe {
                            name,
                            version,
                            layer,
                            preferred_version,
                            file: file.map(PathBuf::from),
                            append_count,
                        },
                    )
                    .collect(),
            ),
            Event::Layers { layers } => BackendEvent::Layers(
                layers
                    .into_iter()
                    .map(
                        |LayerData {
                             name,
                             path,
                             priority,
                         }| Layer {
                            name,
                            path: PathBuf::from(path),
                            priority,
                        },
                    )
                    .collect(),
            ),
            Event::Variable {
                name,
                recipe,
                value,
                provenance,
                unexpanded_value,
                operations,
                active_overrides,
            } => BackendEvent::Variable {
                name,
                recipe,
                value,
                provenance,
                unexpanded_value,
                operations: operations
                    .into_iter()
                    .map(|operation| VariableOperation {
                        operation: operation.operation,
                        file: operation.file.map(PathBuf::from),
                        line: operation.line,
                        value: operation.value,
                    })
                    .collect(),
                active_overrides,
            },
            Event::Dependencies {
                recipe,
                build,
                runtime,
            } => BackendEvent::Dependencies {
                recipe,
                build,
                runtime,
            },
            Event::DependencyGraph { data } => {
                let DependencyGraphData {
                    root,
                    nodes,
                    edges,
                    mut limitations,
                } = data;
                let root = dependency_node_id(root)?;
                let mut dropped_paths = 0;
                let nodes = nodes
                    .into_iter()
                    .map(|DependencyNodeData { id, provider, log }| {
                        let provider = provider.map(PathBuf::from).and_then(|path| {
                            if path.is_absolute() {
                                Some(path)
                            } else {
                                dropped_paths += 1;
                                None
                            }
                        });
                        let log = log.map(PathBuf::from).and_then(|path| {
                            if path.is_absolute() {
                                Some(path)
                            } else {
                                dropped_paths += 1;
                                None
                            }
                        });
                        Ok(DependencyNode {
                            id: dependency_node_id(id)?,
                            provider,
                            log,
                        })
                    })
                    .collect::<Result<Vec<_>, BackendError>>()?;
                let edges = edges
                    .into_iter()
                    .map(|DependencyEdgeData { from, to, kind }| {
                        Ok(DependencyEdge {
                            from: dependency_node_id(from)?,
                            to: dependency_node_id(to)?,
                            kind: match kind {
                                DependencyEdgeKindData::Build => DependencyEdgeKind::Build,
                                DependencyEdgeKindData::Runtime => DependencyEdgeKind::Runtime,
                                DependencyEdgeKindData::Task => DependencyEdgeKind::Task,
                            },
                        })
                    })
                    .collect::<Result<Vec<_>, BackendError>>()?;
                let (graph, report) = DependencyGraph::normalize(
                    root,
                    nodes,
                    edges,
                    MAX_DEPENDENCY_NODES,
                    MAX_DEPENDENCY_EDGES,
                );
                if report.is_partial() {
                    limitations.push(format!(
                        "Rust adapter bounds dropped {} nodes and {} edges",
                        report.truncated_nodes, report.truncated_edges
                    ));
                }
                if dropped_paths > 0 {
                    limitations.push(format!(
                        "Rust adapter dropped {dropped_paths} non-absolute provider or log paths"
                    ));
                }
                BackendEvent::DependencyGraph { graph, limitations }
            }
            Event::RecipeSources { recipe, paths } => BackendEvent::RecipeSources {
                recipe,
                paths: paths.into_iter().map(PathBuf::from).collect(),
            },
            Event::RecipeMetadata { data } => BackendEvent::RecipeMetadata(RecipeMetadata {
                recipe: data.recipe,
                workspace_status: data.workspace_status.map(|status| match status {
                    RecipeWorkspaceStatusData::Clean => RecipeWorkspaceStatus::Clean,
                    RecipeWorkspaceStatusData::Modified => RecipeWorkspaceStatus::Modified,
                }),
                build_status: data.build_status.map(|status| match status {
                    RecipeBuildStatusData::Idle => RecipeBuildStatus::Idle,
                    RecipeBuildStatusData::Queued => RecipeBuildStatus::Queued,
                    RecipeBuildStatusData::Running => RecipeBuildStatus::Running,
                    RecipeBuildStatusData::Succeeded => RecipeBuildStatus::Succeeded,
                    RecipeBuildStatusData::Failed => RecipeBuildStatus::Failed,
                    RecipeBuildStatusData::Cancelled => RecipeBuildStatus::Cancelled,
                }),
                tasks: data.tasks,
                sources: data
                    .sources
                    .map(|paths| paths.into_iter().map(PathBuf::from).collect()),
                patches: data.patches,
                packages: data.packages,
                history: data.history,
            }),
            Event::LayerRelationships { layers } => BackendEvent::LayerRelationships(
                layers
                    .into_iter()
                    .map(
                        |LayerRelationshipData {
                             name,
                             priority,
                             compatible,
                             depends,
                             overlays,
                             appends,
                         }| LayerRelationship {
                            name,
                            priority,
                            compatible,
                            depends,
                            overlays,
                            appends,
                        },
                    )
                    .collect(),
            ),
            Event::BuildStarted => BackendEvent::BuildStarted,
            Event::ParseProgress { current, total } => {
                BackendEvent::ParseProgress { current, total }
            }
            Event::TaskQueued {
                recipe,
                task,
                worker,
                stats,
            } => BackendEvent::TaskQueued {
                recipe,
                task,
                worker,
                stats: task_stats(stats),
            },
            Event::TaskStarted {
                recipe,
                task,
                pid,
                worker,
                log_path,
                stats,
            } => BackendEvent::TaskStarted {
                recipe,
                task,
                pid,
                worker,
                log_path: log_path.map(PathBuf::from),
                stats: task_stats(stats),
            },
            Event::TaskProgress {
                recipe,
                task,
                progress,
            } => BackendEvent::TaskProgress {
                recipe,
                task,
                progress,
            },
            Event::TaskCompleted {
                recipe,
                task,
                success,
            } => BackendEvent::TaskCompleted {
                recipe,
                task,
                success,
            },
            Event::Log {
                level,
                message,
                recipe,
                task,
                path,
            } => {
                let severity = match level.as_str() {
                    "warning" => Severity::Warning,
                    "error" => Severity::Error,
                    _ => Severity::Info,
                };
                BackendEvent::Log(LogEntry {
                    id: 0,
                    severity,
                    message,
                    recipe,
                    task,
                    path: path.map(PathBuf::from),
                    timestamp: SystemTime::now(),
                    build: None,
                    protected: false,
                    diagnostic: None,
                })
            }
            Event::Warning { message } => BackendEvent::Log(LogEntry {
                id: 0,
                severity: Severity::Warning,
                message,
                recipe: None,
                task: None,
                path: None,
                timestamp: SystemTime::now(),
                build: None,
                protected: true,
                diagnostic: None,
            }),
            Event::Error { message } => BackendEvent::Log(LogEntry {
                id: 0,
                severity: Severity::Error,
                message,
                recipe: None,
                task: None,
                path: None,
                timestamp: SystemTime::now(),
                build: None,
                protected: true,
                diagnostic: None,
            }),
            Event::BuildCompleted { success, exit_code } => {
                BackendEvent::BuildCompleted { success, exit_code }
            }
            Event::CommandFailed { code, message } | Event::ProtocolError { code, message } => {
                BackendEvent::CommandFailed { code, message }
            }
            Event::BridgeShutdown | Event::ServerTerminated => BackendEvent::Disconnected,
            Event::HelloAck { .. } | Event::Unknown => BackendEvent::Ignored,
        })
    }
}

fn task_stats(data: Option<TaskStatsData>) -> Option<TaskStats> {
    data.map(|stats| TaskStats {
        completed: stats.completed,
        total: stats.total,
        active: stats.active,
        failed: stats.failed,
    })
}

fn dependency_node_id(data: DependencyNodeIdData) -> Result<DependencyNodeId, BackendError> {
    if data.recipe.is_empty()
        || data.recipe.len() > 512
        || data.recipe.chars().any(char::is_whitespace)
        || data.recipe.chars().any(char::is_control)
        || data.task.as_ref().is_some_and(|task| {
            task.is_empty()
                || task.len() > 512
                || task.chars().any(char::is_whitespace)
                || task.chars().any(char::is_control)
        })
    {
        return Err(BackendError::Bridge(
            "protocol dependency graph contains an invalid node identity".into(),
        ));
    }
    Ok(match data.task {
        Some(task) => DependencyNodeId::task(data.recipe, task),
        None => DependencyNodeId::recipe(data.recipe),
    })
}

fn legacy_dependency_graph(
    recipe: String,
    build: Vec<String>,
    runtime: Vec<String>,
) -> DependencyGraphResponse {
    let root = DependencyNodeId::recipe(recipe);
    let edges = build
        .into_iter()
        .map(|dependency| DependencyEdge {
            from: root.clone(),
            to: DependencyNodeId::recipe(dependency),
            kind: DependencyEdgeKind::Build,
        })
        .chain(runtime.into_iter().map(|dependency| DependencyEdge {
            from: root.clone(),
            to: DependencyNodeId::recipe(dependency),
            kind: DependencyEdgeKind::Runtime,
        }))
        .collect();
    let (graph, _) = DependencyGraph::normalize(
        root,
        Vec::new(),
        edges,
        MAX_DEPENDENCY_NODES,
        MAX_DEPENDENCY_EDGES,
    );
    DependencyGraphResponse {
        graph,
        limitations: vec![
            "Legacy bridge supplied direct recipe edges only; task dependencies are unavailable."
                .into(),
        ],
    }
}

fn dot_quoted_id(line: &str) -> Result<(String, &str), BackendError> {
    let Some(content) = line.strip_prefix('"') else {
        return Err(BackendError::Bridge(
            "malformed dependency graph identifier".into(),
        ));
    };
    let mut value = String::new();
    let mut escaped = false;
    for (offset, character) in content.char_indices() {
        if escaped {
            match character {
                '"' | '\\' => value.push(character),
                _ => {
                    return Err(BackendError::Bridge(
                        "unsupported escape in dependency graph identifier".into(),
                    ));
                }
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Ok((value, &content[offset + character.len_utf8()..]));
        } else {
            value.push(character);
        }
    }
    Err(BackendError::Bridge(
        "unterminated dependency graph identifier".into(),
    ))
}

fn dependency_task_identity(value: String) -> Result<DependencyNodeId, BackendError> {
    let Some((recipe, task)) = value.rsplit_once('.') else {
        return Err(BackendError::Bridge(
            "dependency graph task identity has no task separator".into(),
        ));
    };
    if recipe.is_empty()
        || task.is_empty()
        || recipe.len() > 512
        || task.len() > 512
        || recipe.chars().any(char::is_whitespace)
        || task.chars().any(char::is_whitespace)
        || recipe.chars().any(char::is_control)
        || task.chars().any(char::is_control)
    {
        return Err(BackendError::Bridge(
            "dependency graph contains an invalid task identity".into(),
        ));
    }
    Ok(DependencyNodeId::task(recipe, task))
}

fn parse_task_dependency_dot(
    recipe: &str,
    bytes: &[u8],
) -> Result<DependencyGraphResponse, BackendError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| BackendError::Bridge("dependency graph is not valid UTF-8".into()))?;
    let root = DependencyNodeId::recipe(recipe);
    let mut nodes = vec![DependencyNode::identity(root.clone())];
    let mut edges = Vec::new();
    let mut opened = false;
    let mut closed = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !opened {
            if line != "digraph depends {" {
                return Err(BackendError::Bridge(
                    "dependency graph has an invalid header".into(),
                ));
            }
            opened = true;
            continue;
        }
        if line == "}" {
            closed = true;
            continue;
        }
        if closed {
            return Err(BackendError::Bridge(
                "dependency graph contains records after its closing brace".into(),
            ));
        }

        let (source, remainder) = dot_quoted_id(line)?;
        let remainder = remainder.trim_start();
        if remainder.starts_with('[') {
            if !remainder.trim_end_matches(';').ends_with(']') {
                return Err(BackendError::Bridge(
                    "dependency graph node contains malformed attributes".into(),
                ));
            }
            nodes.push(DependencyNode::identity(dependency_task_identity(source)?));
            continue;
        }
        let Some(remainder) = remainder.strip_prefix("->") else {
            return Err(BackendError::Bridge(
                "dependency graph contains an unsupported record".into(),
            ));
        };
        let (destination, trailing) = dot_quoted_id(remainder.trim_start())?;
        if !trailing.trim().trim_end_matches(';').is_empty() {
            return Err(BackendError::Bridge(
                "dependency graph edge contains unsupported attributes".into(),
            ));
        }
        let from = dependency_task_identity(source)?;
        let to = dependency_task_identity(destination)?;
        nodes.push(DependencyNode::identity(from.clone()));
        nodes.push(DependencyNode::identity(to.clone()));
        edges.push(DependencyEdge {
            from: from.clone(),
            to: to.clone(),
            kind: DependencyEdgeKind::Task,
        });
        if from.recipe_name() != to.recipe_name() {
            edges.push(DependencyEdge {
                from: DependencyNodeId::recipe(from.recipe_name()),
                to: DependencyNodeId::recipe(to.recipe_name()),
                kind: DependencyEdgeKind::Build,
            });
        }
    }
    if !opened || !closed {
        return Err(BackendError::Bridge(
            "dependency graph is incomplete".into(),
        ));
    }
    let (graph, report) = DependencyGraph::normalize(
        root,
        nodes,
        edges,
        MAX_DEPENDENCY_NODES,
        MAX_DEPENDENCY_EDGES,
    );
    let mut limitations = vec![
        "The process backend task graph does not report runtime dependency edges.".into(),
        "The process backend task graph does not report provider or task-log paths.".into(),
    ];
    if report.is_partial() {
        limitations.push(format!(
            "Dependency graph bounds dropped {} nodes and {} edges.",
            report.truncated_nodes, report.truncated_edges
        ));
    }
    Ok(DependencyGraphResponse { graph, limitations })
}
#[async_trait]
impl BitBakeBackend for BridgeBackend {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError> {
        self.require_api(BitBakeApiOperation::Workspace)?;
        self.command(Command::InspectWorkspace).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Workspace(workspace) => return Ok(workspace),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(self.disconnected("bridge disconnected during inspection"));
                }
                _ => {}
            }
        }
    }
    async fn list_recipes(&mut self, filter: Option<String>) -> Result<Vec<Recipe>, BackendError> {
        self.require_api(BitBakeApiOperation::Recipes)?;
        self.command(Command::ListRecipes { filter }).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Recipes(recipes) => return Ok(recipes),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(self.disconnected("bridge disconnected while listing recipes"));
                }
                _ => {}
            }
        }
    }
    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError> {
        self.require_api(BitBakeApiOperation::Layers)?;
        self.command(Command::ListLayers).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Layers(layers) => return Ok(layers),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(self.disconnected("bridge disconnected while listing layers"));
                }
                _ => {}
            }
        }
    }
    async fn get_variable(
        &mut self,
        name: String,
        recipe: Option<String>,
    ) -> Result<VariableValue, BackendError> {
        self.require_api(BitBakeApiOperation::Variable)?;
        let requested_recipe = recipe.clone();
        self.command(Command::GetVariable {
            name: name.clone(),
            recipe,
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Variable {
                    name: returned,
                    recipe,
                    value,
                    provenance,
                    unexpanded_value,
                    operations,
                    active_overrides,
                } if returned == name && recipe == requested_recipe => {
                    return Ok(VariableValue {
                        recipe,
                        value,
                        provenance,
                        unexpanded_value,
                        operations,
                        active_overrides,
                    });
                }
                BackendEvent::Variable { .. } => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(self.disconnected("bridge disconnected while reading a variable"));
                }
                _ => {}
            }
        }
    }
    async fn get_dependencies(
        &mut self,
        recipe: String,
    ) -> Result<RecipeDependencies, BackendError> {
        self.require_api(BitBakeApiOperation::Dependencies)?;
        self.command(Command::GetDependencies {
            recipe: recipe.clone(),
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Dependencies {
                    recipe: returned,
                    build,
                    runtime,
                } if returned == recipe => return Ok(RecipeDependencies { build, runtime }),
                BackendEvent::Dependencies { .. } => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading recipe dependencies")
                    );
                }
                _ => continue,
            }
        }
    }
    async fn get_dependency_graph(
        &mut self,
        recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError> {
        self.require_api(BitBakeApiOperation::DependencyGraph)?;
        self.command(Command::GetDependencyGraph {
            recipe: recipe.clone(),
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::DependencyGraph { graph, limitations }
                    if graph.root.recipe_name() == recipe =>
                {
                    return Ok(DependencyGraphResponse { graph, limitations });
                }
                BackendEvent::DependencyGraph { graph, .. } => {
                    return Err(BackendError::Bridge(format!(
                        "bridge returned dependency graph root {} for requested recipe {recipe}",
                        graph.root.recipe_name()
                    )));
                }
                BackendEvent::Dependencies {
                    recipe: returned,
                    build,
                    runtime,
                } if returned == recipe => {
                    return Ok(legacy_dependency_graph(recipe, build, runtime));
                }
                BackendEvent::CommandFailed { code, .. }
                    if code == "invalid_request" || code == "unsupported_command" =>
                {
                    break;
                }
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading the dependency graph")
                    );
                }
                _ => continue,
            }
        }
        let dependencies = self.get_dependencies(recipe.clone()).await?;
        Ok(legacy_dependency_graph(
            recipe,
            dependencies.build,
            dependencies.runtime,
        ))
    }
    async fn get_signature_dump(
        &mut self,
        target: SignatureTarget,
    ) -> Result<SignatureDumpResponse, BackendError> {
        self.signature_adapter
            .dump(target)
            .await
            .map_err(Into::into)
    }
    async fn compare_signatures(
        &mut self,
        request: SignatureComparisonRequest,
    ) -> Result<SignatureComparisonResponse, BackendError> {
        self.signature_adapter
            .compare(request)
            .await
            .map_err(Into::into)
    }
    async fn get_recipe_sources(&mut self, recipe: String) -> Result<Vec<PathBuf>, BackendError> {
        self.require_api(BitBakeApiOperation::RecipeSources)?;
        self.command(Command::GetRecipeSources {
            recipe: recipe.clone(),
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::RecipeSources {
                    recipe: returned,
                    paths,
                } if returned == recipe => return Ok(paths),
                BackendEvent::RecipeSources { .. } => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading recipe source paths")
                    );
                }
                _ => continue,
            }
        }
    }
    async fn get_recipe_metadata(
        &mut self,
        recipe: String,
    ) -> Result<RecipeMetadata, BackendError> {
        self.require_api(BitBakeApiOperation::RecipeMetadata)?;
        self.command(Command::GetRecipeMetadata {
            recipe: recipe.clone(),
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::RecipeMetadata(metadata) if metadata.recipe == recipe => {
                    return Ok(metadata);
                }
                BackendEvent::RecipeMetadata(_) => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading recipe metadata")
                    );
                }
                _ => continue,
            }
        }
    }
    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError> {
        self.require_api(BitBakeApiOperation::LayerRelationships)?;
        self.command(Command::GetLayerRelationships).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::LayerRelationships(layers) => return Ok(layers),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading layer relationships")
                    );
                }
                _ => continue,
            }
        }
    }
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError> {
        self.require_api(BitBakeApiOperation::Build)?;
        if request.force {
            self.require_api(BitBakeApiOperation::ForceTask)?;
        }
        self.command(Command::StartBuild {
            targets: request.targets,
            task: request.task,
            force: request.force,
        })
        .await
    }
    async fn cancel_build(&mut self) -> Result<(), BackendError> {
        self.require_api(BitBakeApiOperation::Cancel)?;
        self.command(Command::CancelBuild).await
    }
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {
        let Some(line) = self.next_line().await? else {
            return Ok(BackendEvent::Disconnected);
        };
        let e: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.validate_correlation(&e)?;
        self.last_sequence = e.sequence;
        Self::event(e.message)
    }

    async fn shutdown(&mut self) -> Result<(), BackendError> {
        BridgeBackend::shutdown(self).await
    }
}
impl Drop for BridgeBackend {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn fixture_script(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("yoctui-{name}-{}-{nonce}", std::process::id()))
    }

    fn process_compatibility(
        build_dir: &std::path::Path,
        executable: &std::path::Path,
    ) -> yoctui_model::DaemonCompatibilitySnapshot {
        use yoctui_model::{
            AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind,
            CapabilityEvidenceOutcome, CapabilityId, CapabilityImplementation,
            CapabilityImplementationKind, CapabilityRecord, CapabilitySnapshot, CapabilityState,
            IdentityAuthority, ToolIdentity, YoctoEnvironmentIdentity,
        };
        let capabilities = [
            (
                CapabilityId::BitBakeBuild,
                compatibility_command::BITBAKE_BUILD_ARGV_IMPLEMENTATION,
            ),
            (
                CapabilityId::BitBakeForceTask,
                compatibility_command::BITBAKE_FORCE_TASK_ARGV_IMPLEMENTATION,
            ),
            (
                CapabilityId::BitBakeGraphGeneration,
                compatibility_command::BITBAKE_GRAPH_ARGV_IMPLEMENTATION,
            ),
            (
                CapabilityId::BitBakeDumpSig,
                compatibility_command::BITBAKE_DUMPSIG_ARGV_IMPLEMENTATION,
            ),
            (
                CapabilityId::BitBakeDiffSigs,
                compatibility_command::BITBAKE_DIFFSIGS_ARGV_IMPLEMENTATION,
            ),
        ];
        yoctui_model::DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation: 1,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        build_dir.to_owned(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    available_tools: AuthoritativeValue::detected(
                        vec![ToolIdentity {
                            id: "bitbake".into(),
                            executable: executable.to_owned(),
                            version: None,
                        }],
                        IdentityAuthority::ExecutableProbe,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities: capabilities
                    .iter()
                    .map(|(id, _)| CapabilityRecord {
                        id: *id,
                        state: CapabilityState::Available,
                        evidence: vec![CapabilityEvidence {
                            kind: CapabilityEvidenceKind::DirectProbe,
                            outcome: CapabilityEvidenceOutcome::Positive,
                            subject: format!("{} fixture probe", id.as_str()),
                            detail: "The fake BitBake command supports this exact test argv."
                                .into(),
                            argv: vec!["bitbake".into(), "--help".into()],
                        }],
                    })
                    .collect(),
            },
            implementations: capabilities
                .into_iter()
                .map(|(id, implementation)| {
                    (
                        id,
                        CapabilityImplementation {
                            id: implementation.into(),
                            kind: CapabilityImplementationKind::Command,
                        },
                    )
                })
                .collect(),
        }
        .normalize()
        .unwrap()
    }

    fn shell_backend(script: PathBuf) -> ProcessBackend {
        let build_dir = std::env::temp_dir();
        ProcessBackend::with_command(
            build_dir.clone(),
            PathBuf::from("/bin/sh"),
            vec![script.into_os_string()],
        )
        .with_compatibility(process_compatibility(&build_dir, Path::new("/bin/sh")))
        .unwrap()
    }

    fn devtool_compatibility(
        build_dir: &Path,
        executable: &Path,
    ) -> yoctui_model::DaemonCompatibilitySnapshot {
        use yoctui_model::{
            AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind,
            CapabilityEvidenceOutcome, CapabilityId, CapabilityImplementation,
            CapabilityImplementationKind, CapabilityRecord, CapabilitySnapshot, CapabilityState,
            IdentityAuthority, ToolIdentity, YoctoEnvironmentIdentity,
        };
        let capabilities = [
            (
                CapabilityId::DevtoolStatus,
                compatibility_devtool::DEVTOOL_STATUS_IMPLEMENTATION,
            ),
            (
                CapabilityId::DevtoolModify,
                compatibility_devtool::DEVTOOL_MODIFY_IMPLEMENTATION,
            ),
            (
                CapabilityId::DevtoolUpdateRecipe,
                compatibility_devtool::DEVTOOL_UPDATE_RECIPE_IMPLEMENTATION,
            ),
            (
                CapabilityId::DevtoolFinish,
                compatibility_devtool::DEVTOOL_FINISH_IMPLEMENTATION,
            ),
            (
                CapabilityId::DevtoolDeployTarget,
                compatibility_devtool::DEVTOOL_DEPLOY_TARGET_IMPLEMENTATION,
            ),
            (
                CapabilityId::DevtoolUndeployTarget,
                compatibility_devtool::DEVTOOL_UNDEPLOY_TARGET_IMPLEMENTATION,
            ),
            (
                CapabilityId::DevtoolReset,
                compatibility_devtool::DEVTOOL_RESET_IMPLEMENTATION,
            ),
            (
                CapabilityId::DevtoolUpgrade,
                compatibility_devtool::DEVTOOL_UPGRADE_IMPLEMENTATION,
            ),
        ];
        yoctui_model::DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation: 1,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        build_dir.to_owned(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    available_tools: AuthoritativeValue::detected(
                        vec![ToolIdentity {
                            id: "devtool".into(),
                            executable: executable.to_owned(),
                            version: None,
                        }],
                        IdentityAuthority::ExecutableProbe,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities: capabilities
                    .iter()
                    .map(|(id, _)| CapabilityRecord {
                        id: *id,
                        state: CapabilityState::Available,
                        evidence: vec![CapabilityEvidence {
                            kind: CapabilityEvidenceKind::DirectProbe,
                            outcome: CapabilityEvidenceOutcome::Positive,
                            subject: format!("{} test probe", id.as_str()),
                            detail: "The fixture exposes this exact Devtool subcommand.".into(),
                            argv: vec![executable.display().to_string(), "--help".into()],
                        }],
                    })
                    .collect(),
            },
            implementations: capabilities
                .into_iter()
                .map(|(id, implementation)| {
                    (
                        id,
                        CapabilityImplementation {
                            id: implementation.into(),
                            kind: CapabilityImplementationKind::Command,
                        },
                    )
                })
                .collect(),
        }
        .normalize()
        .unwrap()
    }

    fn authorized_devtool_command(
        mut executable: PathBuf,
        operation: &DevtoolOperation,
    ) -> Result<DevtoolCommandSpec, DevtoolCompatibilityError> {
        if !executable.is_absolute() {
            executable = Path::new("/test/bin").join(executable);
        }
        let build_dir = std::env::temp_dir();
        let compatibility = devtool_compatibility(&build_dir, &executable);
        DevtoolCommandSpec::with_executable(
            executable,
            operation,
            &compatibility,
            compatibility.snapshot.generation,
            &build_dir,
        )
    }

    #[cfg(unix)]
    fn fake_devtool_command(name: &str, body: &str) -> (PathBuf, DevtoolCommandSpec) {
        let script = fixture_script(name);
        fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let command = DevtoolCommandSpec {
            executable: PathBuf::from("/bin/sh"),
            arguments: vec![
                script.clone().into_os_string(),
                OsString::from("modify"),
                OsString::from("busybox"),
            ],
            capability_generation: 1,
            capability: yoctui_model::CapabilityId::DevtoolModify,
            build_directory: std::env::temp_dir(),
        };
        (script, command)
    }

    #[tokio::test]
    async fn devtool_metadata_fake_process_reports_workspace_and_dirty_git_state() {
        let root = fixture_script("devtool-workspace");
        let source = root.join("sources/busybox");
        fs::create_dir_all(&source).unwrap();
        let devtool = root.join("devtool");
        let git = root.join("git");
        fs::write(
            &devtool,
            format!(
                "#!/bin/sh\nprintf '%s\\n' 'busybox: {} (/layers/core/busybox_1.0.bb)'\n",
                source.display()
            ),
        )
        .unwrap();
        fs::write(
            &git,
            "#!/bin/sh\nprintf '%s\\n' '# branch.oid abc123' '# branch.head work' '1 .M N... 100644 100644 100644 abc abc file.c' '? new.txt' 'u UU N... 100644 100644 100644 100644 abc abc abc conflict.c'\n",
        )
        .unwrap();
        for script in [&devtool, &git] {
            let mut permissions = fs::metadata(script).unwrap().permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(script, permissions).unwrap();
        }
        let identity = RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/core/busybox_1.0.bb".into(),
        };
        let compatibility = devtool_compatibility(&root, &devtool);
        let status = DevtoolInspector::with_programs(devtool, git)
            .inspect_with_compatibility(&root, identity.clone(), &compatibility, 1)
            .await;
        assert_eq!(status.identity, identity);
        assert_eq!(status.capability, DevtoolCapability::Available);
        assert_eq!(
            status.workspace,
            DevtoolWorkspace::Present {
                source_path: source,
                recipe_file: Some("/layers/core/busybox_1.0.bb".into()),
            }
        );
        assert_eq!(
            status.git,
            DevtoolGitState::Available {
                branch: Some("work".into()),
                head: Some("abc123".into()),
                modified: 1,
                untracked: 1,
                conflicted: 1,
            }
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn devtool_job_spec_builds_exact_shell_free_arguments_for_every_operation() {
        let cases = [
            (
                DevtoolOperation::Modify {
                    recipe: "busybox".into(),
                },
                vec!["modify", "busybox"],
            ),
            (
                DevtoolOperation::UpdateRecipe {
                    recipe: "busybox".into(),
                },
                vec!["update-recipe", "busybox"],
            ),
            (
                DevtoolOperation::Finish {
                    recipe: "busybox".into(),
                    destination: "/layers/meta-custom".into(),
                },
                vec!["finish", "busybox", "/layers/meta-custom"],
            ),
            (
                DevtoolOperation::DeployTarget {
                    recipe: "busybox".into(),
                    target: "root@192.0.2.1:/opt".into(),
                },
                vec!["deploy-target", "busybox", "root@192.0.2.1:/opt"],
            ),
            (
                DevtoolOperation::UndeployTarget {
                    recipe: "busybox".into(),
                    target: "root@192.0.2.1".into(),
                },
                vec!["undeploy-target", "busybox", "root@192.0.2.1"],
            ),
            (
                DevtoolOperation::Reset {
                    recipe: "busybox".into(),
                },
                vec!["reset", "busybox"],
            ),
        ];
        for (operation, expected) in cases {
            let command = authorized_devtool_command("devtool".into(), &operation).unwrap();
            assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
            assert_eq!(
                command.arguments(),
                expected.into_iter().map(OsString::from).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn devtool_job_spec_rejects_invalid_operations_before_process_construction() {
        assert!(matches!(
            authorized_devtool_command("devtool".into(), &DevtoolOperation::Reset {
                recipe: "--help".into(),
            }),
            Err(DevtoolCompatibilityError::InvalidRequest(message)) if message.contains("recipe")
        ));
        assert!(matches!(
            authorized_devtool_command("devtool".into(), &DevtoolOperation::DeployTarget {
                recipe: "busybox".into(),
                target: "root@host\n--help".into(),
            }),
            Err(DevtoolCompatibilityError::InvalidRequest(message)) if message.contains("target")
        ));
        assert!(matches!(
            authorized_devtool_command("devtool".into(), &DevtoolOperation::Finish {
                recipe: "busybox".into(),
                destination: "meta-custom".into(),
            }),
            Err(DevtoolCompatibilityError::InvalidRequest(message)) if message.contains("absolute")
        ));
    }

    #[test]
    fn devtool_publish_update_uses_exact_shell_free_arguments() {
        let command = authorized_devtool_command(
            "devtool".into(),
            &DevtoolOperation::UpdateRecipe {
                recipe: "busybox".into(),
            },
        )
        .unwrap();
        assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
        assert_eq!(
            command.arguments(),
            [OsString::from("update-recipe"), OsString::from("busybox")]
        );
    }

    #[test]
    fn devtool_publish_finish_uses_exact_shell_free_arguments() {
        let command = authorized_devtool_command(
            "devtool".into(),
            &DevtoolOperation::Finish {
                recipe: "busybox".into(),
                destination: "/layers/meta-demo".into(),
            },
        )
        .unwrap();
        assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
        assert_eq!(
            command.arguments(),
            [
                OsString::from("finish"),
                OsString::from("busybox"),
                OsString::from("/layers/meta-demo"),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn devtool_publish_finish_preserves_native_destination_bytes() {
        use std::os::unix::ffi::OsStringExt;

        let mut bytes = b"/layers/meta-".to_vec();
        bytes.push(0xfe);
        let command = authorized_devtool_command(
            "devtool".into(),
            &DevtoolOperation::Finish {
                recipe: "busybox".into(),
                destination: PathBuf::from(OsString::from_vec(bytes.clone())),
            },
        )
        .unwrap();
        assert_eq!(command.arguments()[2], OsString::from_vec(bytes));
    }

    #[test]
    fn devtool_target_deploy_validates_before_exact_shell_free_arguments() {
        let operation = DevtoolOperation::DeployTarget {
            recipe: "busybox".into(),
            target: "root@192.0.2.1:/opt/demo".into(),
        };
        let command = authorized_devtool_command("devtool".into(), &operation).unwrap();
        assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
        assert_eq!(
            command.arguments(),
            [
                OsString::from("deploy-target"),
                OsString::from("busybox"),
                OsString::from("root@192.0.2.1:/opt/demo"),
            ]
        );
        assert!(matches!(
            authorized_devtool_command("devtool".into(), &DevtoolOperation::DeployTarget {
                recipe: "busybox".into(),
                target: "--help".into(),
            }),
            Err(DevtoolCompatibilityError::InvalidRequest(message)) if message.contains("target")
        ));
    }

    #[test]
    fn devtool_target_reset_uses_exact_shell_free_arguments() {
        let command = authorized_devtool_command(
            "devtool".into(),
            &DevtoolOperation::Reset {
                recipe: "busybox".into(),
            },
        )
        .unwrap();
        assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
        assert_eq!(
            command.arguments(),
            [OsString::from("reset"), OsString::from("busybox")]
        );
    }

    #[cfg(unix)]
    #[test]
    fn devtool_job_spec_preserves_non_utf8_finish_destination() {
        use std::os::unix::ffi::OsStringExt;

        let mut bytes = b"/layers/meta-".to_vec();
        bytes.push(0xff);
        let destination = PathBuf::from(OsString::from_vec(bytes.clone()));
        let command = authorized_devtool_command(
            "devtool".into(),
            &DevtoolOperation::Finish {
                recipe: "busybox".into(),
                destination,
            },
        )
        .unwrap();
        assert_eq!(command.arguments()[2], OsString::from_vec(bytes));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn devtool_job_runner_streams_bounded_stdout_stderr_and_invalid_utf8() {
        let (script, command) = fake_devtool_command(
            "devtool-runner-output",
            "printf 'stdout line\\n'\nprintf 'stderr line\\n' >&2\nprintf '\\377bad\\n'\nhead -c 70000 /dev/zero | tr '\\000' x\nprintf '\\n'",
        );
        let mut runner = DevtoolJobRunner::new(std::env::temp_dir());
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            DevtoolRunnerEvent::Started
        );
        let mut output = Vec::new();
        loop {
            match runner.next_event().await.unwrap() {
                DevtoolRunnerEvent::Output {
                    stream,
                    line,
                    truncated,
                } => output.push((stream, line, truncated)),
                DevtoolRunnerEvent::Completed { exit_code } => {
                    assert_eq!(exit_code, Some(0));
                    break;
                }
                event => panic!("unexpected runner event: {event:?}"),
            }
        }
        assert!(output.iter().any(|(stream, line, _)| {
            *stream == DevtoolOutputStream::Stdout && line == "stdout line"
        }));
        assert!(output.iter().any(|(stream, line, _)| {
            *stream == DevtoolOutputStream::Stderr && line == "stderr line"
        }));
        assert!(output.iter().any(|(_, line, _)| line.contains('\u{fffd}')));
        let (_, truncated_line, truncated) = output
            .iter()
            .find(|(_, _, truncated)| *truncated)
            .expect("oversized output was not marked truncated");
        assert!(*truncated);
        assert!(truncated_line.len() <= MAX_DEVTOOL_LINE_BYTES);
        fs::remove_file(script).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn devtool_job_runner_rejects_duplicate_missing_and_failed_processes() {
        let (script, command) =
            fake_devtool_command("devtool-runner-failure", "printf 'failed\\n' >&2\nexit 7");
        let mut runner = DevtoolJobRunner::new(std::env::temp_dir());
        runner.start(command.clone()).await.unwrap();
        assert_eq!(runner.start(command).await, Err(DevtoolRunnerError::Busy));
        loop {
            if let DevtoolRunnerEvent::Failed { exit_code } = runner.next_event().await.unwrap() {
                assert_eq!(exit_code, Some(7));
                break;
            }
        }
        fs::remove_file(script).unwrap();

        let missing = fixture_script("missing-devtool-runner");
        let command = DevtoolCommandSpec::with_executable(
            missing.clone(),
            &DevtoolOperation::Reset {
                recipe: "busybox".into(),
            },
            &devtool_compatibility(&std::env::temp_dir(), &missing),
            1,
            &std::env::temp_dir(),
        )
        .unwrap();
        let mut runner = DevtoolJobRunner::new(std::env::temp_dir());
        assert_eq!(
            runner.start(command).await,
            Err(DevtoolRunnerError::MissingExecutable(missing))
        );

        let non_executable = fixture_script("non-executable-devtool-runner");
        fs::write(&non_executable, "#!/bin/sh\nexit 0\n").unwrap();
        let command = DevtoolCommandSpec::with_executable(
            non_executable.clone(),
            &DevtoolOperation::Reset {
                recipe: "busybox".into(),
            },
            &devtool_compatibility(&std::env::temp_dir(), &non_executable),
            1,
            &std::env::temp_dir(),
        )
        .unwrap();
        assert!(matches!(
            runner.start(command).await,
            Err(DevtoolRunnerError::Spawn(_))
        ));
        fs::remove_file(non_executable).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn devtool_job_runner_acknowledges_and_escalates_cancellation() {
        for (name, trap, expected_forced) in [
            ("devtool-runner-graceful", "trap 'exit 0' TERM", false),
            ("devtool-runner-forced", "trap '' TERM", true),
        ] {
            let (script, command) = fake_devtool_command(
                name,
                &format!("{trap}\nprintf 'ready\\n'\nwhile :; do :; done"),
            );
            let mut runner = DevtoolJobRunner::new(std::env::temp_dir())
                .with_cancellation_timeout(Duration::from_millis(250));
            runner.start(command).await.unwrap();
            assert_eq!(
                runner.next_event().await.unwrap(),
                DevtoolRunnerEvent::Started
            );
            loop {
                if matches!(
                    runner.next_event().await.unwrap(),
                    DevtoolRunnerEvent::Output { ref line, .. } if line == "ready"
                ) {
                    break;
                }
            }
            assert!(runner.cancel().await.unwrap());
            assert!(!runner.cancel().await.unwrap());
            loop {
                if let DevtoolRunnerEvent::Cancelled { forced, .. } =
                    runner.next_event().await.unwrap()
                {
                    assert_eq!(forced, expected_forced);
                    break;
                }
            }
            fs::remove_file(script).unwrap();
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn devtool_job_runner_reports_unexpected_event_channel_loss() {
        let (script, command) =
            fake_devtool_command("devtool-runner-channel-loss", "printf 'ready\\n'\nsleep 30");
        let mut runner = DevtoolJobRunner::new(std::env::temp_dir());
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            DevtoolRunnerEvent::Started
        );
        runner.output = None;
        assert!(matches!(
            runner.next_event().await.unwrap(),
            DevtoolRunnerEvent::Lost { message } if message.contains("channel")
        ));
        assert!(!runner.is_active());
        fs::remove_file(script).unwrap();
    }

    #[tokio::test]
    async fn devtool_metadata_distinguishes_missing_tool_and_workspace_directory() {
        let root = fixture_script("devtool-missing");
        fs::create_dir_all(&root).unwrap();
        let identity = RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/core/busybox_1.0.bb".into(),
        };
        let missing_devtool = root.join("does-not-exist");
        let compatibility = devtool_compatibility(&root, &missing_devtool);
        let missing =
            DevtoolInspector::with_programs(missing_devtool, root.join("does-not-exist-either"))
                .inspect_with_compatibility(&root, identity.clone(), &compatibility, 1)
                .await;
        assert_eq!(missing.capability, DevtoolCapability::MissingExecutable);

        let devtool = root.join("devtool");
        let absent_source = root.join("sources/absent");
        fs::write(
            &devtool,
            format!(
                "#!/bin/sh\nprintf '%s\\n' 'busybox: {}'\n",
                absent_source.display()
            ),
        )
        .unwrap();
        let mut permissions = fs::metadata(&devtool).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&devtool, permissions).unwrap();
        let compatibility = devtool_compatibility(&root, &devtool);
        let status = DevtoolInspector::with_programs(devtool, root.join("git"))
            .inspect_with_compatibility(&root, identity, &compatibility, 1)
            .await;
        assert_eq!(
            status.workspace,
            DevtoolWorkspace::MissingDirectory {
                source_path: absent_source
            }
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn devtool_metadata_rejects_malformed_external_records() {
        assert_eq!(
            parse_devtool_status("busybox relative/path"),
            Err("busybox relative/path".into())
        );
        assert_eq!(
            parse_git_status("unexpected"),
            Err("unrecognized Git status record: unexpected".into())
        );
    }

    #[test]
    fn ansi_and_severity() {
        assert_eq!(strip_ansi("\x1b[31merror: bad\x1b[0m"), "error: bad");
        assert_eq!(
            classify_output("WARNING: x".into()).severity,
            Severity::Warning
        )
    }
    #[test]
    fn typed_event_preserves_unknown_progress_and_ignores_future_events() {
        assert!(matches!(
            BridgeBackend::event(Event::TaskProgress {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: None,
            })
            .unwrap(),
            BackendEvent::TaskProgress { progress: None, .. }
        ));
        assert!(matches!(
            BridgeBackend::event(Event::Unknown).unwrap(),
            BackendEvent::Ignored
        ));
        assert!(matches!(
            BridgeBackend::event(Event::BridgeShutdown).unwrap(),
            BackendEvent::Disconnected
        ));
    }

    #[test]
    fn dependency_graph_typed_event_converts_nodes_edges_and_limitations() {
        let root = DependencyNodeIdData {
            recipe: "image".into(),
            task: None,
        };
        let task = DependencyNodeIdData {
            recipe: "busybox".into(),
            task: Some("do_compile".into()),
        };
        let event = Event::DependencyGraph {
            data: DependencyGraphData {
                root: root.clone(),
                nodes: vec![DependencyNodeData {
                    id: task.clone(),
                    provider: Some("/layers/meta/busybox.bb".into()),
                    log: Some("tmp/log.do_compile".into()),
                }],
                edges: vec![DependencyEdgeData {
                    from: root,
                    to: task.clone(),
                    kind: DependencyEdgeKindData::Task,
                }],
                limitations: vec!["runtime unavailable".into()],
            },
        };
        let BackendEvent::DependencyGraph { graph, limitations } =
            BridgeBackend::event(event).unwrap()
        else {
            panic!("dependency graph event was not preserved");
        };
        assert_eq!(graph.root, DependencyNodeId::recipe("image"));
        assert_eq!(
            graph
                .nodes
                .iter()
                .find(|node| node.id == DependencyNodeId::task("busybox", "do_compile"))
                .and_then(|node| node.provider.as_deref()),
            Some(Path::new("/layers/meta/busybox.bb"))
        );
        assert_eq!(graph.edges[0].kind, DependencyEdgeKind::Task);
        assert_eq!(
            graph
                .nodes
                .iter()
                .find(|node| node.id == DependencyNodeId::task("busybox", "do_compile"))
                .and_then(|node| node.log.as_ref()),
            None
        );
        assert!(
            limitations
                .iter()
                .any(|value| value == "runtime unavailable")
        );
        assert!(
            limitations
                .iter()
                .any(|value| value.contains("non-absolute"))
        );
    }

    #[test]
    fn typed_event_workspace_converts_wire_paths_and_metadata() {
        let event = Event::Workspace {
            data: yoctui_protocol::WorkspaceData {
                build_dir: Some("/build".into()),
                source_dir: Some("/poky".into()),
                variables: std::collections::HashMap::from([(
                    "MACHINE".into(),
                    "qemux86-64".into(),
                )]),
                variable_provenance: std::collections::HashMap::new(),
                variable_provenance_chain: std::collections::HashMap::new(),
                bitbake_version: Some("2.19.0".into()),
                release: Some("6.0".into()),
                layers: vec![LayerData {
                    name: "core".into(),
                    path: "/poky/meta".into(),
                    priority: Some(5),
                }],
                recipes: vec![RecipeData {
                    name: "base-files".into(),
                    version: None,
                    layer: Some("core".into()),
                    preferred_version: None,
                    file: Some("/poky/meta/recipes-core/base-files/base-files.bb".into()),
                    append_count: Some(0),
                }],
            },
        };
        let BackendEvent::Workspace(workspace) = BridgeBackend::event(event).unwrap() else {
            panic!("workspace event was not preserved");
        };
        assert_eq!(workspace.build_dir, Some(PathBuf::from("/build")));
        assert_eq!(workspace.layers[0].path, PathBuf::from("/poky/meta"));
        assert_eq!(workspace.recipes[0].name, "base-files");
    }
    #[test]
    fn config_metadata_converts_scope_unexpanded_value_and_operations() {
        let event = Event::Variable {
            name: "MACHINE".into(),
            recipe: Some("base-files".into()),
            value: Some("qemux86-64".into()),
            provenance: Some("/build/conf/local.conf:12".into()),
            unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
            operations: vec![yoctui_protocol::VariableOperationData {
                operation: "set".into(),
                file: Some("/build/conf/local.conf".into()),
                line: Some(12),
                value: Some("${DEFAULT_MACHINE}".into()),
            }],
            active_overrides: vec!["qemux86-64".into()],
        };
        let BackendEvent::Variable {
            name,
            recipe,
            value,
            unexpanded_value,
            operations,
            active_overrides,
            ..
        } = BridgeBackend::event(event).unwrap()
        else {
            panic!("variable detail event was not preserved");
        };
        assert_eq!(name, "MACHINE");
        assert_eq!(recipe.as_deref(), Some("base-files"));
        assert_eq!(value.as_deref(), Some("qemux86-64"));
        assert_eq!(unexpanded_value.as_deref(), Some("${DEFAULT_MACHINE}"));
        assert_eq!(
            operations[0].file,
            Some(PathBuf::from("/build/conf/local.conf"))
        );
        assert_eq!(operations[0].line, Some(12));
        assert_eq!(active_overrides, ["qemux86-64"]);
    }
    #[test]
    fn recipe_metadata_converts_typed_statuses_and_paths() {
        let event = Event::RecipeMetadata {
            data: yoctui_protocol::RecipeMetadataData {
                recipe: "busybox".into(),
                workspace_status: Some(RecipeWorkspaceStatusData::Modified),
                build_status: Some(RecipeBuildStatusData::Running),
                tasks: Some(vec!["do_compile".into()]),
                sources: Some(vec!["/layers/meta/busybox.bb".into()]),
                patches: Some(vec!["file://fix.patch".into()]),
                packages: Some(vec!["busybox".into()]),
                history: None,
            },
        };
        let BackendEvent::RecipeMetadata(metadata) = BridgeBackend::event(event).unwrap() else {
            panic!("recipe metadata event was not preserved");
        };
        assert_eq!(
            metadata.workspace_status,
            Some(RecipeWorkspaceStatus::Modified)
        );
        assert_eq!(metadata.build_status, Some(RecipeBuildStatus::Running));
        assert_eq!(
            metadata.sources,
            Some(vec![PathBuf::from("/layers/meta/busybox.bb")])
        );
        assert_eq!(metadata.history, None);
    }
    #[test]
    fn live_tasks_preserves_queue_statistics_and_task_details() {
        let queued = BridgeBackend::event(Event::TaskQueued {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            worker: Some("worker-1".into()),
            stats: Some(TaskStatsData {
                completed: 3,
                total: 10,
                active: 2,
                failed: 1,
            }),
        })
        .unwrap();
        assert!(matches!(
            queued,
            BackendEvent::TaskQueued {
                stats: Some(TaskStats { total: 10, .. }),
                ..
            }
        ));
        let started = BridgeBackend::event(Event::TaskStarted {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            pid: Some(42),
            worker: Some("worker-1".into()),
            log_path: Some("/tmp/log.do_compile".into()),
            stats: None,
        })
        .unwrap();
        assert!(matches!(
            started,
            BackendEvent::TaskStarted {
                pid: Some(42),
                log_path: Some(path),
                ..
            } if path.as_os_str() == "/tmp/log.do_compile"
        ));
    }
    #[test]
    fn invalid_utf8_output_is_preserved_lossily() {
        assert_eq!(output_text(b"warning: \xff\n"), "warning: �");
    }

    #[tokio::test]
    async fn oversized_process_line_is_truncated_and_stream_continues() {
        let (mut writer, reader) = tokio::io::duplex(MAX_PROCESS_LINE_BYTES + 2);
        let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
        let reader_task = tokio::spawn(read_output(reader, sender));
        writer
            .write_all(&vec![b'x'; MAX_PROCESS_LINE_BYTES + 1])
            .await
            .unwrap();
        writer.write_all(b"\nnext line\n").await.unwrap();
        drop(writer);
        reader_task.await.unwrap();
        assert!(
            receiver
                .recv()
                .await
                .unwrap()
                .message
                .ends_with("[line truncated]")
        );
        assert_eq!(receiver.recv().await.unwrap().message, "next line");
    }

    #[tokio::test]
    async fn process_backend_collects_both_output_streams() {
        let script = fixture_script("fake-bitbake");
        fs::write(
            &script,
            "#!/bin/sh\nprintf 'NOTE: stdout line\\n'\nprintf 'WARNING: stderr line\\n' >&2\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let mut backend = shell_backend(script.clone());
        backend
            .start_build(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
                force: false,
            })
            .await
            .unwrap();
        let mut messages = Vec::new();
        loop {
            match backend.next_event().await.unwrap() {
                BackendEvent::Log(entry) => messages.push(entry),
                BackendEvent::BuildCompleted { success, .. } => {
                    assert!(success);
                    break;
                }
                _ => {}
            }
        }
        fs::remove_file(script).unwrap();
        assert_eq!(messages.len(), 2);
        assert!(
            messages
                .iter()
                .any(|entry| entry.severity == Severity::Warning)
        );
    }

    #[test]
    fn dependency_graph_dot_parser_normalizes_task_build_cycles_and_bounds() {
        let graph = br#"digraph depends {
"image.do_build" [label="image do_build"]
"busybox.do_build" [label="busybox do_build"]
"image.do_build" -> "busybox.do_build"
"image.do_build" -> "busybox.do_build"
"busybox.do_build" -> "image.do_build"
}
"#;
        let response = parse_task_dependency_dot("image", graph).unwrap();
        assert!(
            response
                .limitations
                .iter()
                .any(|value| value.contains("runtime"))
        );
        assert!(response.graph.edges.contains(&DependencyEdge {
            from: DependencyNodeId::recipe("image"),
            to: DependencyNodeId::recipe("busybox"),
            kind: DependencyEdgeKind::Build,
        }));
        assert!(response.graph.edges.contains(&DependencyEdge {
            from: DependencyNodeId::task("image", "do_build"),
            to: DependencyNodeId::task("busybox", "do_build"),
            kind: DependencyEdgeKind::Task,
        }));
        assert_eq!(
            parse_task_dependency_dot("image", b"not a dot graph")
                .unwrap_err()
                .to_string(),
            "bridge: dependency graph has an invalid header"
        );

        let mut bounded = String::from("digraph depends {\n");
        for index in 0..=MAX_DEPENDENCY_EDGES {
            bounded.push_str(&format!(
                "\"image.do_{index}\" -> \"dep-{index}.do_build\"\n"
            ));
        }
        bounded.push_str("}\n");
        let response = parse_task_dependency_dot("image", bounded.as_bytes()).unwrap();
        assert!(
            response
                .limitations
                .iter()
                .any(|value| value.contains("bounds dropped"))
        );
        assert!(response.graph.nodes.len() <= MAX_DEPENDENCY_NODES);
        assert!(response.graph.edges.len() <= MAX_DEPENDENCY_EDGES);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn dependency_graph_process_backend_is_shell_free_and_rejects_failures() {
        let root = fixture_script("dependency-graph-build");
        fs::create_dir_all(&root).unwrap();
        let script = root.join("fake-bitbake");
        fs::write(
            &script,
            r#"#!/bin/sh
test "$1" = "-g" || exit 8
test "$2" = "image" || exit 9
printf '%s\n' 'digraph depends {' '"image.do_build" -> "busybox.do_build"' '}' > task-depends.dot
"#,
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let mut backend = ProcessBackend::with_executable(root.clone(), script.clone())
            .with_compatibility(process_compatibility(&root, &script))
            .unwrap();
        let response = backend.get_dependency_graph("image".into()).await.unwrap();
        assert_eq!(response.graph.root, DependencyNodeId::recipe("image"));
        assert!(
            response
                .graph
                .edges
                .iter()
                .any(|edge| edge.kind == DependencyEdgeKind::Task
                    && edge.to == DependencyNodeId::task("busybox", "do_build"))
        );

        fs::write(&script, "#!/bin/sh\nexit 7\n").unwrap();
        let error = backend
            .get_dependency_graph("image".into())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("exited with 7"));

        fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
        let error = backend
            .get_dependency_graph("image".into())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("No such file"));

        let missing = root.join("missing-bitbake");
        let mut unavailable = ProcessBackend::with_executable(root.clone(), missing.clone())
            .with_compatibility(process_compatibility(&root, &missing))
            .unwrap();
        let error = unavailable
            .get_dependency_graph("image".into())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("process:"));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn dependency_graph_process_backend_rejects_symlink_output() {
        let root = fixture_script("dependency-graph-symlink");
        fs::create_dir_all(&root).unwrap();
        let outside = fixture_script("dependency-graph-outside");
        fs::write(&outside, "digraph depends {\n}\n").unwrap();
        let script = root.join("fake-bitbake");
        fs::write(
            &script,
            format!(
                "#!/bin/sh\nln -s '{}' task-depends.dot\n",
                outside.display()
            ),
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let mut backend = ProcessBackend::with_executable(root.clone(), script.clone())
            .with_compatibility(process_compatibility(&root, &script))
            .unwrap();
        let error = backend
            .get_dependency_graph("image".into())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("not a regular file"));
        fs::remove_dir_all(root).unwrap();
        fs::remove_file(outside).unwrap();
    }

    #[tokio::test]
    async fn recipe_bitbake_action_process_backend_preserves_force_task_and_target_arguments() {
        let script = fixture_script("fake-recipe-task");
        fs::write(&script, "#!/bin/sh\nprintf '%s\\n' \"$*\"\n").unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let mut backend = shell_backend(script.clone());
        backend
            .start_build(BuildRequest {
                targets: vec!["busybox".into()],
                task: Some("compile".into()),
                force: true,
            })
            .await
            .unwrap();
        let output = loop {
            match backend.next_event().await.unwrap() {
                BackendEvent::Log(entry) => break entry.message,
                BackendEvent::BuildCompleted { .. } => {
                    panic!("process completed before its arguments were observed")
                }
                _ => {}
            }
        };
        fs::remove_file(script).unwrap();
        assert_eq!(output, "-f -c compile busybox");
    }

    #[tokio::test]
    async fn process_backend_cancellation_acknowledges_a_hung_child() {
        let script = fixture_script("hung-bitbake");
        fs::write(&script, "#!/bin/sh\nsleep 30\n").unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let mut backend = shell_backend(script.clone());
        backend
            .start_build(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
                force: false,
            })
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), backend.cancel_build())
            .await
            .unwrap()
            .unwrap();
        loop {
            if let BackendEvent::BuildCompleted { success, .. } =
                tokio::time::timeout(Duration::from_secs(2), backend.next_event())
                    .await
                    .unwrap()
                    .unwrap()
            {
                assert!(!success);
                break;
            }
        }
        fs::remove_file(script).unwrap();
    }

    #[tokio::test]
    async fn process_backend_escalates_after_configured_cancellation_timeout() {
        let script = fixture_script("term-ignoring-bitbake");
        fs::write(
            &script,
            "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let mut backend =
            shell_backend(script.clone()).with_cancellation_timeout(Duration::from_millis(20));
        backend
            .start_build(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
                force: false,
            })
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), backend.cancel_build())
            .await
            .unwrap()
            .unwrap();
        fs::remove_file(script).unwrap();
    }

    #[tokio::test]
    async fn process_backend_reports_exit_code() {
        let script = fixture_script("failed-bitbake");
        fs::write(
            &script,
            "#!/bin/sh\nprintf 'ERROR: failed build\\n' >&2\nexit 7\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let mut backend = shell_backend(script.clone());
        backend
            .start_build(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
                force: false,
            })
            .await
            .unwrap();
        loop {
            if let BackendEvent::BuildCompleted { success, exit_code } =
                backend.next_event().await.unwrap()
            {
                assert!(!success);
                assert_eq!(exit_code, Some(7));
                break;
            }
        }
        fs::remove_file(script).unwrap();
    }

    #[test]
    fn bridge_stderr_tail_is_bounded_and_redacts_sensitive_lines() {
        let mut tail = BridgeStderrTail::default();
        tail.push(&vec![b'x'; MAX_BRIDGE_STDERR_BYTES + 20]);
        tail.push(b"\nAPI_TOKEN=do-not-display\nlast diagnostic\n");
        let diagnostic = tail.diagnostic().unwrap();
        assert!(tail.bytes.len() <= MAX_BRIDGE_STDERR_BYTES);
        assert!(diagnostic.starts_with("[earlier bridge stderr truncated]"));
        assert!(diagnostic.contains("[redacted sensitive diagnostic]"));
        assert!(diagnostic.contains("last diagnostic"));
        assert!(!diagnostic.contains("do-not-display"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn bridge_stderr_is_captured_without_affecting_protocol() {
        let script = fixture_script("bridge-stderr-protocol");
        fs::write(
            &script,
            r#"#!/bin/sh
read -r _request
printf '%s\n' 'NOTE: bridge startup diagnostic' >&2
printf '%s\n' '{"protocol_version":1,"sequence":1,"correlation_id":"1","message":{"type":"hello_ack","bitbake_version":"test"}}'
read -r _request
printf '%s\n' '{"protocol_version":1,"sequence":2,"correlation_id":"2","message":{"type":"bridge_shutdown"}}'
"#,
        )
        .unwrap();
        let mut backend = BridgeBackend::spawn("/bin/sh", script.clone(), std::env::temp_dir())
            .await
            .unwrap();
        for _ in 0..20 {
            if backend.stderr_diagnostic().is_some() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            backend.stderr_diagnostic().as_deref(),
            Some("NOTE: bridge startup diagnostic")
        );
        backend.shutdown().await.unwrap();
        fs::remove_file(script).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn bridge_stderr_is_attached_to_failed_handshake() {
        let script = fixture_script("bridge-stderr-failure");
        fs::write(
            &script,
            "#!/bin/sh\nprintf '%s\\n' 'fatal bridge fixture marker' >&2\nexit 17\n",
        )
        .unwrap();
        let error =
            match BridgeBackend::spawn("/bin/sh", script.clone(), std::env::temp_dir()).await {
                Ok(_) => panic!("bridge startup unexpectedly succeeded"),
                Err(error) => error,
            };
        assert!(error.to_string().contains("fatal bridge fixture marker"));
        assert!(error.to_string().contains("bridge stderr:"));
        fs::remove_file(script).unwrap();
    }

    #[tokio::test]
    async fn bridge_backend_requires_compatibility_before_workspace_inspection() {
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bridge/yoctui_bridge.py");
        let mut backend = BridgeBackend::spawn("python3", script, std::env::temp_dir())
            .await
            .unwrap();
        assert!(
            backend
                .inspect_workspace()
                .await
                .unwrap_err()
                .to_string()
                .contains("requires a daemon capability snapshot")
        );
        backend.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn bridge_backend_waits_for_shutdown_acknowledgement() {
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bridge/yoctui_bridge.py");
        let mut backend = BridgeBackend::spawn("python3", script, std::env::temp_dir())
            .await
            .unwrap();
        backend.shutdown().await.unwrap();
        assert!(backend.child.try_wait().unwrap().is_some());
    }

    #[tokio::test]
    async fn bridge_backend_rejects_typed_queries_without_compatibility() {
        let mut backend = BridgeBackend::spawn_bundled("python3", std::env::temp_dir())
            .await
            .unwrap();
        assert!(
            backend
                .list_recipes(None)
                .await
                .unwrap_err()
                .to_string()
                .contains("requires a daemon capability snapshot")
        );
        backend.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn bundled_bridge_starts_without_a_source_checkout_path() {
        assert!(BUNDLED_BRIDGE_SOURCE.contains("class BitBakeAdapter"));
        let mut backend = BridgeBackend::spawn_bundled("python3", std::env::temp_dir())
            .await
            .unwrap();
        assert!(backend.list_recipes(None).await.is_err());
        backend.shutdown().await.unwrap();
    }
}
