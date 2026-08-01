//! BitBake adapters. They execute BitBake; they never evaluate metadata themselves.
mod image;
mod maintenance_optional;
mod maintenance_release;
mod maintenance_service;
mod maintenance_sstate;
mod package;
mod qa_layer;
mod qa_report;
mod qa_task;
mod qemu;
mod sdk;
mod sdk_tool;
mod security;
mod security_mapper;
mod security_report;
mod signature;
mod test_results;
mod test_runner;
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
    PackageDataAdapter, PackageDataAdapterError, PackageDataCancellation, PackageDataCommandSpec,
    PackageDetailResponse, PackageInventoryResponse,
};
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
pub use sdk::{
    SdkArtifactAdapter, SdkArtifactAdapterError, SdkArtifactCancellation, SdkArtifactResponse,
    SdkArtifactScanOutcome,
};
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
pub use signature::{
    SignatureAdapter, SignatureAdapterError, SignatureCancellation, SignatureCommandSpec,
    SignatureComparisonResponse, SignatureDumpResponse,
};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
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
pub use wic::{
    WicAdapterError, WicCapabilityInspector, WicCreateCommandSpec, WicDeviceInspector,
    WicDeviceInventoryResponse, WicJobRunner, WicWriteCommandSpec,
};
use yoctui_model::{
    BuildRequest, DependencyEdge, DependencyEdgeKind, DependencyGraph, DependencyNode,
    DependencyNodeId, DevtoolCapability, DevtoolGitState, DevtoolOperation, DevtoolOperationError,
    DevtoolStatus, DevtoolStatusError, DevtoolWorkspace, ImageArtifactInventory,
    ImageArtifactRequest, Layer, LogEntry, PackageDetail, PackageDetailRequest,
    PackageInventoryRequest, PackageSummary, Recipe, RecipeBuildStatus, RecipeIdentity,
    RecipeMetadata, RecipeWorkspaceStatus, Severity, SignatureComparisonRequest,
    SignatureDifference, SignatureRecord, SignatureTarget, TaskStats, VariableOperation, Workspace,
};
use yoctui_protocol::{
    Command, DependencyEdgeData, DependencyEdgeKindData, DependencyGraphData, DependencyNodeData,
    DependencyNodeIdData, Envelope, Event, LayerData, LayerRelationshipData, MAX_LINE_BYTES,
    ProtocolError, RecipeBuildStatusData, RecipeData, RecipeWorkspaceStatusData, TaskStatsData,
    VERSION, decode_line, encode_line,
};

const MAX_PROCESS_LINE_BYTES: usize = 1024 * 1024;
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
    #[error("backend is not running")]
    NotRunning,
}

#[derive(Debug, Clone)]
pub struct DevtoolInspector {
    devtool_program: PathBuf,
    git_program: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
}
impl DevtoolCommandSpec {
    pub fn from_operation(operation: &DevtoolOperation) -> Result<Self, DevtoolOperationError> {
        Self::with_executable(PathBuf::from("devtool"), operation)
    }

    pub fn with_executable(
        executable: PathBuf,
        operation: &DevtoolOperation,
    ) -> Result<Self, DevtoolOperationError> {
        operation.validate()?;
        let recipe = OsString::from(operation.recipe());
        let arguments = match operation {
            DevtoolOperation::Modify { .. } => vec![OsString::from("modify"), recipe],
            DevtoolOperation::UpdateRecipe { .. } => {
                vec![OsString::from("update-recipe"), recipe]
            }
            DevtoolOperation::Finish { destination, .. } => vec![
                OsString::from("finish"),
                recipe,
                destination.as_os_str().to_owned(),
            ],
            DevtoolOperation::DeployTarget { target, .. } => vec![
                OsString::from("deploy-target"),
                recipe,
                OsString::from(target),
            ],
            DevtoolOperation::UndeployTarget { target, .. } => vec![
                OsString::from("undeploy-target"),
                recipe,
                OsString::from(target),
            ],
            DevtoolOperation::Reset { .. } => vec![OsString::from("reset"), recipe],
        };
        Ok(Self {
            executable,
            arguments,
        })
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
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
            devtool_program: "devtool".into(),
            git_program: "git".into(),
        }
    }
}

impl DevtoolInspector {
    pub fn with_programs(devtool_program: PathBuf, git_program: PathBuf) -> Self {
        Self {
            devtool_program,
            git_program,
        }
    }

    pub async fn inspect(&self, build_dir: &Path, identity: RecipeIdentity) -> DevtoolStatus {
        if !identity.file.is_absolute() {
            return DevtoolStatus {
                identity,
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: DevtoolGitState::NotApplicable,
                error: Some(DevtoolStatusError::InvalidRecipeIdentity),
            };
        }

        let output = TokioCommand::new(&self.devtool_program)
            .arg("status")
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

        let mut command = TokioCommand::new(&self.executable);
        command
            .args(&self.arguments)
            .arg("-g")
            .arg(&recipe)
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
        let mut cmd = TokioCommand::new(&self.executable);
        cmd.args(&self.arguments);
        if request.force {
            cmd.arg("-f");
        }
        if let Some(task) = request.task.as_ref() {
            cmd.args(["-c", task]);
        }
        cmd.args(&request.targets)
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
    signature_adapter: SignatureAdapter,
}
impl BridgeBackend {
    pub async fn spawn(
        python: &str,
        script: PathBuf,
        build_dir: PathBuf,
    ) -> Result<Self, BackendError> {
        let mut child = TokioCommand::new(python)
            .arg(script)
            .current_dir(&build_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| BackendError::Bridge("bridge stdin unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BackendError::Bridge("bridge stdout unavailable".into()))?;
        let mut backend = Self {
            child,
            stdin,
            lines: BufReader::new(stdout),
            sequence: 0,
            last_sequence: 0,
            signature_adapter: SignatureAdapter::new(build_dir),
        };
        backend.handshake().await?;
        Ok(backend)
    }
    async fn command(&mut self, message: Command) -> Result<(), BackendError> {
        self.sequence += 1;
        let bytes = encode_line(&Envelope {
            protocol_version: VERSION,
            sequence: self.sequence,
            correlation_id: Some(self.sequence.to_string()),
            message,
        })?;
        self.stdin.write_all(&bytes).await?;
        self.stdin.flush().await?;
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
        self.command(Command::Hello).await?;
        let Some(line) = self.next_line().await? else {
            return Err(BackendError::Bridge(
                "bridge disconnected during protocol handshake".into(),
            ));
        };
        let envelope: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.last_sequence = envelope.sequence;
        match envelope.message {
            Event::HelloAck { .. } => Ok(()),
            Event::ProtocolError { code, message } | Event::CommandFailed { code, message } => Err(
                BackendError::Bridge(format!("handshake rejected: {code}: {message}")),
            ),
            _ => Err(BackendError::Bridge(
                "bridge sent an unexpected handshake event".into(),
            )),
        }
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
            Event::BridgeShutdown => BackendEvent::Disconnected,
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
        self.command(Command::InspectWorkspace).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Workspace(workspace) => return Ok(workspace),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(BackendError::Bridge(
                        "bridge disconnected during inspection".into(),
                    ));
                }
                _ => {}
            }
        }
    }
    async fn list_recipes(&mut self, filter: Option<String>) -> Result<Vec<Recipe>, BackendError> {
        self.command(Command::ListRecipes { filter }).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Recipes(recipes) => return Ok(recipes),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(BackendError::Bridge(
                        "bridge disconnected while listing recipes".into(),
                    ));
                }
                _ => {}
            }
        }
    }
    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError> {
        self.command(Command::ListLayers).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Layers(layers) => return Ok(layers),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(BackendError::Bridge(
                        "bridge disconnected while listing layers".into(),
                    ));
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
                    return Err(BackendError::Bridge(
                        "bridge disconnected while reading a variable".into(),
                    ));
                }
                _ => {}
            }
        }
    }
    async fn get_dependencies(
        &mut self,
        recipe: String,
    ) -> Result<RecipeDependencies, BackendError> {
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
                    return Err(BackendError::Bridge(
                        "bridge disconnected while reading recipe dependencies".into(),
                    ));
                }
                _ => continue,
            }
        }
    }
    async fn get_dependency_graph(
        &mut self,
        recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError> {
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
                    return Err(BackendError::Bridge(
                        "bridge disconnected while reading the dependency graph".into(),
                    ));
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
                    return Err(BackendError::Bridge(
                        "bridge disconnected while reading recipe source paths".into(),
                    ));
                }
                _ => continue,
            }
        }
    }
    async fn get_recipe_metadata(
        &mut self,
        recipe: String,
    ) -> Result<RecipeMetadata, BackendError> {
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
                    return Err(BackendError::Bridge(
                        "bridge disconnected while reading recipe metadata".into(),
                    ));
                }
                _ => continue,
            }
        }
    }
    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError> {
        self.command(Command::GetLayerRelationships).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::LayerRelationships(layers) => return Ok(layers),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(BackendError::Bridge(
                        "bridge disconnected while reading layer relationships".into(),
                    ));
                }
                _ => continue,
            }
        }
    }
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError> {
        self.command(Command::StartBuild {
            targets: request.targets,
            task: request.task,
            force: request.force,
        })
        .await
    }
    async fn cancel_build(&mut self) -> Result<(), BackendError> {
        self.command(Command::CancelBuild).await
    }
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {
        let Some(line) = self.next_line().await? else {
            return Ok(BackendEvent::Disconnected);
        };
        let e: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
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

    fn shell_backend(script: PathBuf) -> ProcessBackend {
        ProcessBackend::with_command(
            std::env::temp_dir(),
            PathBuf::from("/bin/sh"),
            vec![script.into_os_string()],
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
        let status = DevtoolInspector::with_programs(devtool, git)
            .inspect(&root, identity.clone())
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
            let command = DevtoolCommandSpec::from_operation(&operation).unwrap();
            assert_eq!(command.executable(), Path::new("devtool"));
            assert_eq!(
                command.arguments(),
                expected.into_iter().map(OsString::from).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn devtool_job_spec_rejects_invalid_operations_before_process_construction() {
        assert_eq!(
            DevtoolCommandSpec::from_operation(&DevtoolOperation::Reset {
                recipe: "--help".into(),
            }),
            Err(DevtoolOperationError::InvalidRecipe)
        );
        assert_eq!(
            DevtoolCommandSpec::from_operation(&DevtoolOperation::DeployTarget {
                recipe: "busybox".into(),
                target: "root@host\n--help".into(),
            }),
            Err(DevtoolOperationError::InvalidTarget)
        );
        assert_eq!(
            DevtoolCommandSpec::from_operation(&DevtoolOperation::Finish {
                recipe: "busybox".into(),
                destination: "meta-custom".into(),
            }),
            Err(DevtoolOperationError::RelativeFinishDestination)
        );
    }

    #[test]
    fn devtool_publish_update_uses_exact_shell_free_arguments() {
        let command = DevtoolCommandSpec::from_operation(&DevtoolOperation::UpdateRecipe {
            recipe: "busybox".into(),
        })
        .unwrap();
        assert_eq!(command.executable(), Path::new("devtool"));
        assert_eq!(
            command.arguments(),
            [OsString::from("update-recipe"), OsString::from("busybox")]
        );
    }

    #[test]
    fn devtool_publish_finish_uses_exact_shell_free_arguments() {
        let command = DevtoolCommandSpec::from_operation(&DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: "/layers/meta-demo".into(),
        })
        .unwrap();
        assert_eq!(command.executable(), Path::new("devtool"));
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
        let command = DevtoolCommandSpec::from_operation(&DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: PathBuf::from(OsString::from_vec(bytes.clone())),
        })
        .unwrap();
        assert_eq!(command.arguments()[2], OsString::from_vec(bytes));
    }

    #[test]
    fn devtool_target_deploy_validates_before_exact_shell_free_arguments() {
        let operation = DevtoolOperation::DeployTarget {
            recipe: "busybox".into(),
            target: "root@192.0.2.1:/opt/demo".into(),
        };
        let command = DevtoolCommandSpec::from_operation(&operation).unwrap();
        assert_eq!(command.executable(), Path::new("devtool"));
        assert_eq!(
            command.arguments(),
            [
                OsString::from("deploy-target"),
                OsString::from("busybox"),
                OsString::from("root@192.0.2.1:/opt/demo"),
            ]
        );
        assert_eq!(
            DevtoolCommandSpec::from_operation(&DevtoolOperation::DeployTarget {
                recipe: "busybox".into(),
                target: "--help".into(),
            }),
            Err(DevtoolOperationError::InvalidTarget)
        );
    }

    #[test]
    fn devtool_target_reset_uses_exact_shell_free_arguments() {
        let command = DevtoolCommandSpec::from_operation(&DevtoolOperation::Reset {
            recipe: "busybox".into(),
        })
        .unwrap();
        assert_eq!(command.executable(), Path::new("devtool"));
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
        let command = DevtoolCommandSpec::from_operation(&DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination,
        })
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
        let missing = DevtoolInspector::with_programs(
            root.join("does-not-exist"),
            root.join("does-not-exist-either"),
        )
        .inspect(&root, identity.clone())
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
        let status = DevtoolInspector::with_programs(devtool, root.join("git"))
            .inspect(&root, identity)
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
        let mut backend = ProcessBackend::with_executable(root.clone(), script.clone());
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

        let mut unavailable =
            ProcessBackend::with_executable(root.clone(), root.join("missing-bitbake"));
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
        let mut backend = ProcessBackend::with_executable(root.clone(), script);
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

    #[tokio::test]
    async fn bridge_backend_negotiates_before_workspace_inspection() {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bridge/yoctui_bridge.py");
        let mut backend = BridgeBackend::spawn("python3", script, std::env::temp_dir())
            .await
            .unwrap();
        let workspace = backend.inspect_workspace().await.unwrap();
        assert_eq!(workspace.build_dir, Some(std::env::temp_dir()));
        backend.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn bridge_backend_waits_for_shutdown_acknowledgement() {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bridge/yoctui_bridge.py");
        let mut backend = BridgeBackend::spawn("python3", script, std::env::temp_dir())
            .await
            .unwrap();
        backend.shutdown().await.unwrap();
        assert!(backend.child.try_wait().unwrap().is_some());
    }

    #[tokio::test]
    async fn bridge_backend_decodes_typed_workspace_queries() {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bridge/yoctui_bridge.py");
        let mut backend = BridgeBackend::spawn("python3", script, std::env::temp_dir())
            .await
            .unwrap();
        assert!(backend.list_recipes(None).await.unwrap().is_empty());
        let _layers = backend.list_layers().await.unwrap();
        assert!(
            backend
                .get_variable("PATH".into(), None)
                .await
                .unwrap()
                .value
                .is_some()
        );
        backend.shutdown().await.unwrap();
    }
}
