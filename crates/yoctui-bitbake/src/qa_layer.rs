use std::{
    collections::{BTreeSet, VecDeque},
    ffi::{OsStr, OsString},
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
    time::Instant,
};
use yoctui_model::{
    MAX_QA_LAYER_ARGUMENTS, MAX_QA_REPORT_PATHS, MAX_QA_SCOPES, MAX_QA_TEXT_BYTES, QaCheckId,
    QaConfiguredLayerCapability, QaExecutableIdentity, QaLayerCapabilitySnapshot, QaLayerIdentity,
    QaLayerOperationId, QaLayerOperationPreview, QaLayerRunCapability, QaLayerSessionId,
    QaOutputStream,
};

use crate::output_text;

const QA_LAYER_EVENT_CHANNEL_CAPACITY: usize = 256;
const QA_LAYER_OPERATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const QA_LAYER_SPAWN_ATTEMPTS: usize = 4;
const QA_LAYER_SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);

#[cfg(unix)]
fn is_transient_qa_layer_spawn_error(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ETXTBSY)
}

#[cfg(not(unix))]
fn is_transient_qa_layer_spawn_error(_error: &io::Error) -> bool {
    false
}

async fn spawn_qa_layer_process(process: &mut Command) -> io::Result<Child> {
    for attempt in 1..=QA_LAYER_SPAWN_ATTEMPTS {
        match process.spawn() {
            Ok(child) => return Ok(child),
            Err(error)
                if attempt < QA_LAYER_SPAWN_ATTEMPTS
                    && is_transient_qa_layer_spawn_error(&error) =>
            {
                tokio::time::sleep(QA_LAYER_SPAWN_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the bounded layer-QA process spawn loop always returns")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaConfiguredLayerInput {
    pub check: QaCheckId,
    pub identity: QaLayerIdentity,
    pub compatible_series: Vec<String>,
    pub report_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaLayerCapabilityInput {
    pub release: Option<String>,
    pub build_directory: PathBuf,
    pub selected_layer: QaLayerIdentity,
    pub layers: Vec<QaConfiguredLayerInput>,
    pub executable_search_path: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaLayerCapabilityResponse {
    Available(QaLayerCapabilitySnapshot),
    Partial(QaLayerCapabilitySnapshot),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum QaLayerAdapterError {
    #[error("invalid layer-QA capability input: {0}")]
    InvalidInput(String),
    #[error("layer-QA preview is invalid: {0}")]
    InvalidPreview(String),
    #[error("layer-QA preview was modified after confirmation")]
    PreviewMismatch,
    #[error("layer-QA executable is unsafe: {0}")]
    UnsafeExecutable(PathBuf),
    #[error("configured layer identity is unsafe: {0}")]
    UnsafeLayer(PathBuf),
    #[error("layer-QA report root is unsafe: {0}")]
    UnsafeReportRoot(PathBuf),
    #[error("layer-QA identity became stale: {0}")]
    StaleIdentity(PathBuf),
    #[error("a layer-QA process or unconsumed event is already active")]
    Busy,
    #[error("could not start layer QA: {0}")]
    Spawn(String),
    #[error("layer-QA process stream is unavailable: {0:?}")]
    StreamUnavailable(QaOutputStream),
    #[error("layer-QA runner is not active")]
    NotRunning,
    #[error("layer-QA process control failed: {0}")]
    ProcessControl(String),
}

#[derive(Debug, Default)]
pub struct QaLayerCapabilityInspector;

impl QaLayerCapabilityInspector {
    pub fn inspect(
        input: QaLayerCapabilityInput,
    ) -> Result<QaLayerCapabilityResponse, QaLayerAdapterError> {
        if input.layers.is_empty()
            || input.layers.len() > MAX_QA_SCOPES
            || input.executable_search_path.len() > MAX_QA_REPORT_PATHS
            || !input.selected_layer.is_valid()
        {
            return Err(QaLayerAdapterError::InvalidInput(
                "configured layers, selected layer, or search path are invalid".into(),
            ));
        }
        let build_directory = canonical_directory(&input.build_directory)
            .map_err(|_| QaLayerAdapterError::InvalidInput("build directory is unsafe".into()))?;
        if build_directory != input.build_directory {
            return Err(QaLayerAdapterError::InvalidInput(
                "build directory must be canonical".into(),
            ));
        }
        let mut identities = BTreeSet::new();
        if input.layers.iter().any(|layer| {
            !layer.check.is_valid()
                || !layer.identity.is_valid()
                || !identities.insert(layer.identity.clone())
        }) || !input
            .layers
            .iter()
            .any(|layer| layer.identity == input.selected_layer)
        {
            return Err(QaLayerAdapterError::InvalidInput(
                "configured layer identities must be valid, unique, and include the selection"
                    .into(),
            ));
        }

        let mut limitations = Vec::new();
        let executable = discover_executable(&input.executable_search_path, &mut limitations);
        let mut layers = Vec::new();
        for layer in input.layers {
            let mut layer_limitations = Vec::new();
            let canonical_layer = canonical_directory(&layer.identity.root).ok();
            let roots = validate_report_roots(&layer.report_roots, &mut layer_limitations);
            let run = match canonical_layer {
                None => QaLayerRunCapability::Disabled("configured layer root is unsafe".into()),
                Some(root) if root != layer.identity.root => {
                    QaLayerRunCapability::Disabled("configured layer root is not canonical".into())
                }
                Some(root) => match &executable {
                    Some(executable) => QaLayerRunCapability::Available {
                        executable: executable.clone(),
                        arguments: vec![root.display().to_string()],
                        report_roots: roots,
                    },
                    None => QaLayerRunCapability::Disabled(
                        "yocto-check-layer was not found as a canonical executable".into(),
                    ),
                },
            };
            if let Some(reason) = run.disabled_reason() {
                layer_limitations.push(reason.into());
            }
            let capability = QaConfiguredLayerCapability::new(
                layer.check,
                layer.identity,
                layer.compatible_series,
                run,
                layer_limitations.clone(),
            )
            .map_err(|message| QaLayerAdapterError::InvalidInput(message.into()))?;
            limitations.extend(layer_limitations);
            layers.push(capability);
        }
        let snapshot = QaLayerCapabilitySnapshot::new(
            input.release,
            build_directory,
            input.selected_layer,
            layers,
            limitations.clone(),
        )
        .map_err(|message| QaLayerAdapterError::InvalidInput(message.into()))?;
        if limitations.is_empty() {
            Ok(QaLayerCapabilityResponse::Available(snapshot))
        } else {
            Ok(QaLayerCapabilityResponse::Partial(snapshot))
        }
    }
}

fn discover_executable(
    search_path: &[PathBuf],
    limitations: &mut Vec<String>,
) -> Option<QaExecutableIdentity> {
    for directory in search_path {
        let Ok(canonical) = canonical_directory(directory) else {
            limitations.push(format!(
                "ignored unsafe layer-QA executable search directory: {}",
                directory.display()
            ));
            continue;
        };
        if canonical != *directory {
            limitations.push(format!(
                "ignored non-canonical layer-QA executable search directory: {}",
                directory.display()
            ));
            continue;
        }
        let candidate = directory.join("yocto-check-layer");
        if !candidate.exists() {
            continue;
        }
        match executable_identity(&candidate) {
            Ok(identity) => return Some(identity),
            Err(_) => limitations.push(format!(
                "ignored unsafe yocto-check-layer candidate: {}",
                candidate.display()
            )),
        }
    }
    None
}

fn validate_report_roots(paths: &[PathBuf], limitations: &mut Vec<String>) -> Vec<PathBuf> {
    if paths.len() > MAX_QA_REPORT_PATHS {
        limitations.push(format!(
            "report roots exceeded the {MAX_QA_REPORT_PATHS}-path bound"
        ));
    }
    let mut roots = Vec::new();
    for path in paths.iter().take(MAX_QA_REPORT_PATHS) {
        match canonical_file_or_directory(path) {
            Ok(canonical) if canonical == *path => roots.push(canonical),
            _ => limitations.push(format!("ignored unsafe QA report root: {}", path.display())),
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

fn executable_identity(path: &Path) -> Result<QaExecutableIdentity, QaLayerAdapterError> {
    if path.file_name() != Some(OsStr::new("yocto-check-layer")) {
        return Err(QaLayerAdapterError::UnsafeExecutable(path.into()));
    }
    let metadata = safe_metadata(path, false)
        .map_err(|_| QaLayerAdapterError::UnsafeExecutable(path.into()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(QaLayerAdapterError::UnsafeExecutable(path.into()));
        }
    }
    let canonical =
        fs::canonicalize(path).map_err(|_| QaLayerAdapterError::UnsafeExecutable(path.into()))?;
    if canonical != path {
        return Err(QaLayerAdapterError::UnsafeExecutable(path.into()));
    }
    QaExecutableIdentity::new(
        canonical,
        metadata.len(),
        metadata
            .modified()
            .map_err(|_| QaLayerAdapterError::UnsafeExecutable(path.into()))?,
    )
    .map_err(|_| QaLayerAdapterError::UnsafeExecutable(path.into()))
}

fn revalidate_executable(identity: &QaExecutableIdentity) -> Result<(), QaLayerAdapterError> {
    let current = executable_identity(&identity.path)?;
    if &current != identity {
        return Err(QaLayerAdapterError::StaleIdentity(identity.path.clone()));
    }
    Ok(())
}

fn safe_metadata(path: &Path, allow_directory: bool) -> Result<fs::Metadata, ()> {
    if !path.is_absolute() || path == Path::new("/") {
        return Err(());
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if metadata.file_type().is_symlink()
        || (!metadata.is_file() && !(allow_directory && metadata.is_dir()))
    {
        return Err(());
    }
    Ok(metadata)
}

fn canonical_directory(path: &Path) -> Result<PathBuf, ()> {
    let metadata = safe_metadata(path, true)?;
    if !metadata.is_dir() {
        return Err(());
    }
    let canonical = fs::canonicalize(path).map_err(|_| ())?;
    (canonical == path).then_some(canonical).ok_or(())
}

fn canonical_file_or_directory(path: &Path) -> Result<PathBuf, ()> {
    safe_metadata(path, true)?;
    let canonical = fs::canonicalize(path).map_err(|_| ())?;
    (canonical == path).then_some(canonical).ok_or(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaLayerCommandSpec {
    id: QaLayerSessionId,
    preview: QaLayerOperationPreview,
    executable: PathBuf,
    arguments: Vec<OsString>,
    current_directory: PathBuf,
}

impl QaLayerCommandSpec {
    /// Reconstruct a confirmed command at the daemon boundary.
    ///
    /// The wire request intentionally carries paths and bounded arguments rather
    /// than a serialized filesystem identity.  The executable identity is
    /// re-read on the daemon host immediately before validation, so replacement
    /// or symlink attacks still fail the normal `from_preview` checks.
    pub fn from_paths(
        session: QaLayerSessionId,
        operation: QaLayerOperationId,
        check: QaCheckId,
        layer: QaLayerIdentity,
        executable: PathBuf,
        arguments: Vec<String>,
        report_roots: Vec<PathBuf>,
    ) -> Result<Self, QaLayerAdapterError> {
        if session.0 == 0 || operation.0 == 0 || !check.is_valid() || !layer.is_valid() {
            return Err(QaLayerAdapterError::InvalidPreview(
                "session, operation, check, or layer is invalid".into(),
            ));
        }
        let executable = executable_identity(&executable)?;
        let indexed_arguments = indexed_arguments(&executable.path, &arguments);
        let preview = QaLayerOperationPreview {
            id: operation,
            check,
            layer,
            executable,
            indexed_arguments,
            arguments,
            report_roots,
            limitations: Vec::new(),
        };
        Self::from_preview(session, &preview)
    }

    pub fn from_preview(
        session: QaLayerSessionId,
        preview: &QaLayerOperationPreview,
    ) -> Result<Self, QaLayerAdapterError> {
        if session.0 == 0
            || preview.id.0 == 0
            || !preview.check.is_valid()
            || !preview.layer.is_valid()
            || !preview.executable.is_valid()
            || preview.arguments.is_empty()
            || preview.arguments.len() > MAX_QA_LAYER_ARGUMENTS
            || preview.arguments.iter().any(|value| !bounded_text(value))
            || preview.report_roots.len() > MAX_QA_REPORT_PATHS
        {
            return Err(QaLayerAdapterError::InvalidPreview(
                "session, operation, layer, executable, arguments, or roots are invalid".into(),
            ));
        }
        if preview.arguments != vec![preview.layer.root.display().to_string()] {
            return Err(QaLayerAdapterError::InvalidPreview(
                "layer-QA arguments are not the exact configured-layer vector".into(),
            ));
        }
        revalidate_executable(&preview.executable)?;
        let layer = canonical_directory(&preview.layer.root)
            .map_err(|_| QaLayerAdapterError::UnsafeLayer(preview.layer.root.clone()))?;
        let mut roots = preview
            .report_roots
            .iter()
            .map(|path| {
                canonical_file_or_directory(path)
                    .map_err(|_| QaLayerAdapterError::UnsafeReportRoot(path.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        roots.sort();
        roots.dedup();
        if roots != preview.report_roots {
            return Err(QaLayerAdapterError::InvalidPreview(
                "report roots must be canonical, sorted, and unique".into(),
            ));
        }
        let expected_indexed = indexed_arguments(&preview.executable.path, &preview.arguments);
        if preview.indexed_arguments != expected_indexed {
            return Err(QaLayerAdapterError::PreviewMismatch);
        }
        Ok(Self {
            id: session,
            preview: preview.clone(),
            executable: preview.executable.path.clone(),
            arguments: preview.arguments.iter().map(OsString::from).collect(),
            current_directory: layer,
        })
    }

    pub fn id(&self) -> QaLayerSessionId {
        self.id
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn current_directory(&self) -> &Path {
        &self.current_directory
    }

    fn revalidate(&self) -> Result<(), QaLayerAdapterError> {
        let reconstructed = Self::from_preview(self.id, &self.preview)?;
        if reconstructed != *self {
            return Err(QaLayerAdapterError::PreviewMismatch);
        }
        Ok(())
    }
}

fn indexed_arguments(executable: &Path, arguments: &[String]) -> Vec<String> {
    std::iter::once(format!("0: {}", executable.display()))
        .chain(
            arguments
                .iter()
                .enumerate()
                .map(|(index, argument)| format!("{}: {argument}", index + 1)),
        )
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaLayerRunnerEvent {
    Started {
        id: QaLayerSessionId,
    },
    Output {
        id: QaLayerSessionId,
        stream: QaOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        id: QaLayerSessionId,
        exit_code: Option<i32>,
    },
    Failed {
        id: QaLayerSessionId,
        exit_code: Option<i32>,
    },
    CancellationRequested {
        id: QaLayerSessionId,
    },
    Cancelled {
        id: QaLayerSessionId,
        forced: bool,
        exit_code: Option<i32>,
    },
    CancellationRejected {
        id: QaLayerSessionId,
        message: String,
    },
    TimedOut {
        id: QaLayerSessionId,
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        id: QaLayerSessionId,
        message: String,
    },
}

#[derive(Debug)]
enum PipeEvent {
    Output {
        stream: QaOutputStream,
        line: String,
        truncated: bool,
    },
    Failed {
        stream: QaOutputStream,
        message: String,
    },
}

async fn read_output<R>(
    stream: R,
    kind: QaOutputStream,
    sender: tokio::sync::mpsc::Sender<PipeEvent>,
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
                    .send(PipeEvent::Failed {
                        stream: kind,
                        message: error.to_string(),
                    })
                    .await;
                break;
            }
        };
        if buffer.is_empty() {
            if !bytes.is_empty() || truncated {
                let _ = sender
                    .send(PipeEvent::Output {
                        stream: kind,
                        line: output_text(&bytes),
                        truncated,
                    })
                    .await;
            }
            break;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        if !truncated {
            let remaining = MAX_QA_TEXT_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(PipeEvent::Output {
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

pub struct QaLayerJobRunner {
    id: Option<QaLayerSessionId>,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<PipeEvent>>,
    streams_drained: bool,
    started_pending: bool,
    terminal_pending: VecDeque<QaLayerRunnerEvent>,
    cancellation_timeout: Duration,
    operation_timeout: Duration,
    deadline: Option<Instant>,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl Default for QaLayerJobRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl QaLayerJobRunner {
    pub fn new() -> Self {
        Self {
            id: None,
            child: None,
            output: None,
            streams_drained: true,
            started_pending: false,
            terminal_pending: VecDeque::new(),
            cancellation_timeout: Duration::from_secs(5),
            operation_timeout: QA_LAYER_OPERATION_TIMEOUT,
            deadline: None,
            cancellation_requested: false,
            #[cfg(unix)]
            process_group: None,
        }
    }

    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }

    pub fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    pub fn is_active(&self) -> bool {
        self.child.is_some()
    }

    pub async fn start(&mut self, command: QaLayerCommandSpec) -> Result<(), QaLayerAdapterError> {
        if self.child.is_some()
            || self.started_pending
            || !self.terminal_pending.is_empty()
            || self.output.is_some()
        {
            return Err(QaLayerAdapterError::Busy);
        }
        command.revalidate()?;
        let mut process = Command::new(&command.executable);
        process
            .args(&command.arguments)
            .current_dir(&command.current_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        process.process_group(0);
        let mut child = spawn_qa_layer_process(&mut process)
            .await
            .map_err(|error| QaLayerAdapterError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            self.clear_process_state();
            return Err(QaLayerAdapterError::StreamUnavailable(
                QaOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            self.clear_process_state();
            return Err(QaLayerAdapterError::StreamUnavailable(
                QaOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(QA_LAYER_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_output(stdout, QaOutputStream::Stdout, sender.clone()));
        tokio::spawn(read_output(stderr, QaOutputStream::Stderr, sender.clone()));
        drop(sender);
        self.id = Some(command.id);
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.started_pending = true;
        self.deadline = Some(Instant::now() + self.operation_timeout);
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<QaLayerRunnerEvent, QaLayerAdapterError> {
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        let id = self.id.ok_or(QaLayerAdapterError::NotRunning)?;
        if self.started_pending {
            self.started_pending = false;
            return Ok(QaLayerRunnerEvent::Started { id });
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            self.kill_and_clear().await;
            return Ok(QaLayerRunnerEvent::Lost {
                id,
                message: "layer-QA output event channel was lost".into(),
            });
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return self.timeout_active(id).await;
        }
        if let Some(receiver) = self.output.as_mut() {
            let deadline = self.deadline.ok_or(QaLayerAdapterError::NotRunning)?;
            let event = tokio::select! {
                event = receiver.recv() => Some(event),
                _ = tokio::time::sleep_until(deadline) => None,
            };
            match event {
                Some(Some(PipeEvent::Output {
                    stream,
                    line,
                    truncated,
                })) => {
                    return Ok(QaLayerRunnerEvent::Output {
                        id,
                        stream,
                        line,
                        truncated,
                    });
                }
                Some(Some(PipeEvent::Failed { stream, message })) => {
                    self.kill_and_clear().await;
                    return Ok(QaLayerRunnerEvent::Lost {
                        id,
                        message: format!("{stream:?} stream failed: {message}"),
                    });
                }
                Some(None) => {
                    self.output = None;
                    self.streams_drained = true;
                }
                None => return self.timeout_active(id).await,
            }
        }
        let deadline = self.deadline.ok_or(QaLayerAdapterError::NotRunning)?;
        let status = {
            let child = self.child.as_mut().ok_or(QaLayerAdapterError::NotRunning)?;
            match tokio::time::timeout_at(deadline, child.wait()).await {
                Ok(Ok(status)) => status,
                Ok(Err(error)) => {
                    self.kill_and_clear().await;
                    return Ok(QaLayerRunnerEvent::Lost {
                        id,
                        message: format!("layer-QA wait failed: {error}"),
                    });
                }
                Err(_) => return self.timeout_active(id).await,
            }
        };
        self.clear_process_state();
        if status.success() {
            Ok(QaLayerRunnerEvent::Completed {
                id,
                exit_code: status.code(),
            })
        } else {
            Ok(QaLayerRunnerEvent::Failed {
                id,
                exit_code: status.code(),
            })
        }
    }

    pub async fn cancel(
        &mut self,
        requested_id: QaLayerSessionId,
    ) -> Result<bool, QaLayerAdapterError> {
        if self.cancellation_requested || self.child.is_none() || self.id != Some(requested_id) {
            self.terminal_pending
                .push_back(QaLayerRunnerEvent::CancellationRejected {
                    id: requested_id,
                    message: "no matching cancellable layer-QA process is active".into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        self.terminal_pending
            .push_back(QaLayerRunnerEvent::CancellationRequested { id: requested_id });
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state_preserving_events();
        self.terminal_pending
            .push_back(QaLayerRunnerEvent::Cancelled {
                id: requested_id,
                forced,
                exit_code: status.and_then(|status| status.code()),
            });
        Ok(true)
    }

    async fn timeout_active(
        &mut self,
        id: QaLayerSessionId,
    ) -> Result<QaLayerRunnerEvent, QaLayerAdapterError> {
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state();
        Ok(QaLayerRunnerEvent::TimedOut {
            id,
            forced,
            exit_code: status.and_then(|status| status.code()),
        })
    }

    async fn terminate_active(
        &mut self,
    ) -> Result<(Option<std::process::ExitStatus>, bool), QaLayerAdapterError> {
        let Some(child) = self.child.as_mut() else {
            return Ok((None, false));
        };
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(process_group) = self.process_group {
            // SAFETY: the negative PID targets only the process group created for this child.
            if unsafe { libc::kill(-process_group, libc::SIGTERM) } != 0 {
                child
                    .start_kill()
                    .map_err(|error| QaLayerAdapterError::ProcessControl(error.to_string()))?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                Ok(result) => Some(
                    result
                        .map_err(|error| QaLayerAdapterError::ProcessControl(error.to_string()))?,
                ),
                Err(_) => {
                    // SAFETY: same child-owned process group as the graceful signal.
                    let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                    forced = true;
                    Some(
                        child.wait().await.map_err(|error| {
                            QaLayerAdapterError::ProcessControl(error.to_string())
                        })?,
                    )
                }
            }
        } else {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| QaLayerAdapterError::ProcessControl(error.to_string()))?;
            Some(
                child
                    .wait()
                    .await
                    .map_err(|error| QaLayerAdapterError::ProcessControl(error.to_string()))?,
            )
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| QaLayerAdapterError::ProcessControl(error.to_string()))?;
            Some(
                child
                    .wait()
                    .await
                    .map_err(|error| QaLayerAdapterError::ProcessControl(error.to_string()))?,
            )
        };
        Ok((status, forced))
    }

    async fn kill_and_clear(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.clear_process_state();
    }

    fn clear_process_state(&mut self) {
        self.clear_process_state_preserving_events();
        self.terminal_pending.clear();
    }

    fn clear_process_state_preserving_events(&mut self) {
        self.id = None;
        self.child = None;
        self.output = None;
        self.streams_drained = true;
        self.started_pending = false;
        self.deadline = None;
        self.cancellation_requested = false;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
    }

    #[cfg(test)]
    fn lose_output_channel(&mut self) {
        self.output = None;
    }
}

impl Drop for QaLayerJobRunner {
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

fn bounded_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_QA_TEXT_BYTES && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-qa-layer-{name}-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(unix)]
    fn write_executable(path: &Path, body: &str) {
        crate::test_support::write_executable(path, body);
    }

    #[cfg(unix)]
    fn fixture(name: &str, body: &str) -> (TestDirectory, QaLayerCapabilitySnapshot) {
        let root = TestDirectory::new(name);
        let bin = root.0.join("bin");
        let layer = root.0.join("meta-demo");
        let reports = root.0.join("reports");
        fs::create_dir(&bin).unwrap();
        fs::create_dir(&layer).unwrap();
        fs::create_dir(&reports).unwrap();
        write_executable(&bin.join("yocto-check-layer"), body);
        let identity = QaLayerIdentity::new("meta-demo".into(), layer).unwrap();
        let input = QaLayerCapabilityInput {
            release: Some("6.0".into()),
            build_directory: root.0.clone(),
            selected_layer: identity.clone(),
            layers: vec![QaConfiguredLayerInput {
                check: QaCheckId::new("layer-meta-demo".into()).unwrap(),
                identity,
                compatible_series: vec!["walnascar".into()],
                report_roots: vec![reports],
            }],
            executable_search_path: vec![bin],
        };
        let response = QaLayerCapabilityInspector::inspect(input).unwrap();
        let snapshot = match response {
            QaLayerCapabilityResponse::Available(snapshot) => snapshot,
            QaLayerCapabilityResponse::Partial(_) => panic!("expected complete capability"),
        };
        (root, snapshot)
    }

    fn preview(snapshot: &QaLayerCapabilitySnapshot) -> QaLayerOperationPreview {
        let layer = &snapshot.layers[0];
        let QaLayerRunCapability::Available {
            executable,
            arguments,
            report_roots,
        } = &layer.run
        else {
            panic!("expected runnable layer");
        };
        QaLayerOperationPreview {
            id: yoctui_model::QaLayerOperationId(4),
            check: layer.check.clone(),
            layer: layer.identity.clone(),
            executable: executable.clone(),
            arguments: arguments.clone(),
            indexed_arguments: indexed_arguments(&executable.path, arguments),
            report_roots: report_roots.clone(),
            limitations: Vec::new(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn qa_layer_capability_discovers_exact_configured_layers_and_partial_inputs() {
        let (root, snapshot) = fixture("capability", "#!/bin/sh\nexit 0\n");
        assert_eq!(snapshot.layers.len(), 1);
        assert!(matches!(
            snapshot.layers[0].run,
            QaLayerRunCapability::Available { .. }
        ));
        let missing = root.0.join("missing-bin");
        fs::create_dir(&missing).unwrap();
        let identity = snapshot.selected_layer.clone();
        let response = QaLayerCapabilityInspector::inspect(QaLayerCapabilityInput {
            release: None,
            build_directory: root.0.clone(),
            selected_layer: identity.clone(),
            layers: vec![QaConfiguredLayerInput {
                check: QaCheckId::new("layer-meta-demo".into()).unwrap(),
                identity,
                compatible_series: Vec::new(),
                report_roots: Vec::new(),
            }],
            executable_search_path: vec![missing],
        })
        .unwrap();
        assert!(matches!(response, QaLayerCapabilityResponse::Partial(_)));
    }

    #[cfg(unix)]
    #[test]
    fn qa_layer_capability_and_command_reject_symlink_tampering_and_preview_changes() {
        let (root, snapshot) = fixture("safety", "#!/bin/sh\nexit 0\n");
        let mut exact = preview(&snapshot);
        let command = QaLayerCommandSpec::from_preview(QaLayerSessionId(8), &exact).unwrap();
        assert_eq!(
            command.arguments(),
            &[OsString::from(exact.layer.root.as_os_str())]
        );
        exact.indexed_arguments.push("2: injected".into());
        assert!(matches!(
            QaLayerCommandSpec::from_preview(QaLayerSessionId(8), &exact),
            Err(QaLayerAdapterError::PreviewMismatch)
        ));
        let executable = command.executable().to_owned();
        write_executable(&executable, "#!/bin/sh\nexit 1\n");
        assert!(matches!(
            command.revalidate(),
            Err(QaLayerAdapterError::StaleIdentity(path)) if path == executable
        ));

        let real = root.0.join("real-bin");
        fs::create_dir(&real).unwrap();
        write_executable(&real.join("yocto-check-layer"), "#!/bin/sh\nexit 0\n");
        let link = root.0.join("linked-bin");
        symlink(&real, &link).unwrap();
        let mut limitations = Vec::new();
        assert!(discover_executable(&[link], &mut limitations).is_none());
        assert!(!limitations.is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qa_layer_runner_streams_bounded_output_and_reports_success_and_nonzero() {
        let (root, snapshot) = fixture(
            "runner",
            &format!(
                "#!/bin/sh\nprintf 'out\\n'\nprintf '%*s\\n' {} x >&2\nexit 0\n",
                MAX_QA_TEXT_BYTES + 64
            ),
        );
        let command =
            QaLayerCommandSpec::from_preview(QaLayerSessionId(9), &preview(&snapshot)).unwrap();
        let mut runner = QaLayerJobRunner::new();
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::Started {
                id: QaLayerSessionId(9)
            }
        );
        let mut saw_truncated = false;
        loop {
            match runner.next_event().await.unwrap() {
                QaLayerRunnerEvent::Output { truncated, .. } => saw_truncated |= truncated,
                QaLayerRunnerEvent::Completed { exit_code, .. } => {
                    assert_eq!(exit_code, Some(0));
                    break;
                }
                event => panic!("unexpected event: {event:?}"),
            }
        }
        assert!(saw_truncated);

        let executable = root.0.join("bin/yocto-check-layer");
        write_executable(&executable, "#!/bin/sh\nexit 7\n");
        let response = QaLayerCapabilityInspector::inspect(QaLayerCapabilityInput {
            release: None,
            build_directory: root.0.clone(),
            selected_layer: snapshot.selected_layer.clone(),
            layers: vec![QaConfiguredLayerInput {
                check: snapshot.layers[0].check.clone(),
                identity: snapshot.selected_layer.clone(),
                compatible_series: Vec::new(),
                report_roots: Vec::new(),
            }],
            executable_search_path: vec![root.0.join("bin")],
        })
        .unwrap();
        let QaLayerCapabilityResponse::Available(snapshot) = response else {
            panic!("expected capability");
        };
        let mut runner = QaLayerJobRunner::new();
        runner
            .start(
                QaLayerCommandSpec::from_preview(QaLayerSessionId(10), &preview(&snapshot))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::Started { .. }
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::Failed {
                exit_code: Some(7),
                ..
            }
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qa_layer_runner_rejects_duplicate_and_cancels_gracefully_or_forcibly() {
        let (_root, snapshot) = fixture(
            "cancel",
            "#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n",
        );
        let command =
            QaLayerCommandSpec::from_preview(QaLayerSessionId(11), &preview(&snapshot)).unwrap();
        let mut runner = QaLayerJobRunner::new();
        runner.start(command.clone()).await.unwrap();
        assert!(matches!(
            runner.start(command).await,
            Err(QaLayerAdapterError::Busy)
        ));
        assert!(runner.cancel(QaLayerSessionId(11)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::CancellationRequested { .. }
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::Cancelled { forced: false, .. }
        ));

        let (_root, snapshot) = fixture(
            "forced",
            "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
        );
        let command =
            QaLayerCommandSpec::from_preview(QaLayerSessionId(12), &preview(&snapshot)).unwrap();
        let mut runner =
            QaLayerJobRunner::new().with_cancellation_timeout(Duration::from_millis(10));
        runner.start(command).await.unwrap();
        runner.next_event().await.unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert!(runner.cancel(QaLayerSessionId(12)).await.unwrap());
        runner.next_event().await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::Cancelled { forced: true, .. }
        ));
    }

    #[cfg(unix)]
    #[test]
    fn qa_layer_spawn_retry_classifies_only_text_file_busy_as_transient() {
        assert!(is_transient_qa_layer_spawn_error(
            &io::Error::from_raw_os_error(libc::ETXTBSY,)
        ));
        assert!(!is_transient_qa_layer_spawn_error(
            &io::Error::from_raw_os_error(libc::ENOENT),
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qa_layer_runner_preserves_timeout_rejection_and_channel_loss() {
        let (_root, snapshot) = fixture(
            "terminal",
            "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
        );
        let command =
            QaLayerCommandSpec::from_preview(QaLayerSessionId(13), &preview(&snapshot)).unwrap();
        let mut runner = QaLayerJobRunner::new()
            .with_operation_timeout(Duration::from_millis(1))
            .with_cancellation_timeout(Duration::from_millis(1));
        runner.start(command).await.unwrap();
        runner.next_event().await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::TimedOut { .. }
        ));
        assert!(!runner.cancel(QaLayerSessionId(99)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::CancellationRejected { .. }
        ));

        let (_root, snapshot) = fixture("loss", "#!/bin/sh\nsleep 30\n");
        let command =
            QaLayerCommandSpec::from_preview(QaLayerSessionId(14), &preview(&snapshot)).unwrap();
        let mut runner = QaLayerJobRunner::new();
        runner.start(command).await.unwrap();
        runner.next_event().await.unwrap();
        runner.lose_output_channel();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::Lost { .. }
        ));
    }
}
