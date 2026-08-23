use std::{
    ffi::OsString,
    io,
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
    CapabilityToolId, DaemonCompatibilitySnapshot, RawAdditionalArguments,
    RawConfirmedExecutionRequest, RawInteractionMode, RawJobId, RawOutputChunk, RawOutputStream,
    RawPreviewRequest, RawRequestId, RawSessionId, RawStreamId, builtin_raw_catalog,
};

use crate::output_text;

const RAW_JOB_EVENT_CHANNEL_CAPACITY: usize = 256;
const RAW_JOB_DEFAULT_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);
const RAW_JOB_DEFAULT_CANCELLATION_TIMEOUT: Duration = Duration::from_secs(5);
const RAW_JOB_SPAWN_ATTEMPTS: usize = 4;
const RAW_JOB_SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawJobCommandSpec {
    request_id: RawRequestId,
    job_id: RawJobId,
    stdout_stream: RawStreamId,
    stderr_stream: RawStreamId,
    executable: PathBuf,
    arguments: Vec<OsString>,
    current_directory: PathBuf,
    capability_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPtyCommandSpec {
    request_id: RawRequestId,
    session_id: RawSessionId,
    executable: PathBuf,
    arguments: Vec<String>,
    current_directory: PathBuf,
    capability_generation: u64,
}

impl RawPtyCommandSpec {
    pub fn request_id(&self) -> &RawRequestId {
        &self.request_id
    }

    pub fn session_id(&self) -> &RawSessionId {
        &self.session_id
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn current_directory(&self) -> &Path {
        &self.current_directory
    }

    pub const fn capability_generation(&self) -> u64 {
        self.capability_generation
    }
}

impl RawJobCommandSpec {
    pub fn request_id(&self) -> &RawRequestId {
        &self.request_id
    }

    pub fn job_id(&self) -> &RawJobId {
        &self.job_id
    }

    pub fn stdout_stream(&self) -> &RawStreamId {
        &self.stdout_stream
    }

    pub fn stderr_stream(&self) -> &RawStreamId {
        &self.stderr_stream
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

    pub const fn capability_generation(&self) -> u64 {
        self.capability_generation
    }
}

pub struct RawJobPlanner<'a> {
    compatibility: &'a DaemonCompatibilitySnapshot,
}

pub struct RawPtyPlanner<'a> {
    compatibility: &'a DaemonCompatibilitySnapshot,
}

impl<'a> RawPtyPlanner<'a> {
    pub fn new(compatibility: &'a DaemonCompatibilitySnapshot) -> Self {
        Self { compatibility }
    }

    pub fn plan(
        &self,
        request: &RawConfirmedExecutionRequest,
        session_id: RawSessionId,
    ) -> Result<RawPtyCommandSpec, RawJobPlannerError> {
        if request.interaction != RawInteractionMode::InteractivePty {
            return Err(RawJobPlannerError::NoninteractiveRequest);
        }
        let (preview, executable) = reconstruct_raw_command(self.compatibility, request)?;
        Ok(RawPtyCommandSpec {
            request_id: request.id.clone(),
            session_id,
            executable,
            arguments: preview.arguments,
            current_directory: request.build_directory.clone(),
            capability_generation: request.capability_generation,
        })
    }
}

impl<'a> RawJobPlanner<'a> {
    pub fn new(compatibility: &'a DaemonCompatibilitySnapshot) -> Self {
        Self { compatibility }
    }

    pub fn plan(
        &self,
        request: &RawConfirmedExecutionRequest,
        job_id: RawJobId,
        stdout_stream: RawStreamId,
        stderr_stream: RawStreamId,
    ) -> Result<RawJobCommandSpec, RawJobPlannerError> {
        if request.interaction != RawInteractionMode::NoninteractiveJob {
            return Err(RawJobPlannerError::InteractiveRequest);
        }
        if stdout_stream == stderr_stream {
            return Err(RawJobPlannerError::DuplicateStreamIdentity);
        }
        let (preview, executable) = reconstruct_raw_command(self.compatibility, request)?;
        Ok(RawJobCommandSpec {
            request_id: request.id.clone(),
            job_id,
            stdout_stream,
            stderr_stream,
            executable,
            arguments: preview.arguments.iter().map(OsString::from).collect(),
            current_directory: request.build_directory.clone(),
            capability_generation: request.capability_generation,
        })
    }
}

fn reconstruct_raw_command(
    compatibility: &DaemonCompatibilitySnapshot,
    request: &RawConfirmedExecutionRequest,
) -> Result<(yoctui_model::RawExecutionPreview, PathBuf), RawJobPlannerError> {
    request
        .validate()
        .map_err(|error| RawJobPlannerError::InvalidRequest(error.to_string()))?;
    let additional_arguments =
        RawAdditionalArguments::from_vec(request.additional_arguments.clone())
            .map_err(|error| RawJobPlannerError::InvalidRequest(error.to_string()))?;
    let preview_request = RawPreviewRequest {
        catalog_version: request.catalog_version,
        command: request.command.clone(),
        parameters: request.parameters.clone(),
        additional_arguments,
        capability_generation: request.capability_generation,
        build_directory: request.build_directory.clone(),
    };
    let catalog = builtin_raw_catalog();
    let preview = catalog
        .preview(&preview_request, Some(compatibility))
        .map_err(|error| RawJobPlannerError::Authorization(error.to_string()))?;
    let reconstructed = RawConfirmedExecutionRequest::from_reviewed_preview(
        request.id.clone(),
        catalog,
        &preview_request,
        &preview,
    )
    .map_err(|error| RawJobPlannerError::Authorization(error.to_string()))?;
    if reconstructed != *request {
        return Err(RawJobPlannerError::PreviewMismatch);
    }
    let executable = compatibility
        .snapshot
        .environment
        .available_tools
        .value()
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| tool.id == CapabilityToolId::BitBake.executable_name())
        })
        .map(|tool| tool.executable.clone())
        .ok_or(RawJobPlannerError::MissingExecutableAuthority)?;
    validate_directory(&request.build_directory)?;
    validate_executable(&executable)?;
    Ok((preview, executable))
}

fn validate_directory(path: &Path) -> Result<(), RawJobPlannerError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| RawJobPlannerError::UnsafeBuildDirectory(path.into(), error.kind()))?;
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| RawJobPlannerError::UnsafeBuildDirectory(path.into(), error.kind()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || canonical != path {
        return Err(RawJobPlannerError::UnsafeBuildDirectory(
            path.into(),
            io::ErrorKind::InvalidInput,
        ));
    }
    Ok(())
}

fn validate_executable(path: &Path) -> Result<(), RawJobPlannerError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| RawJobPlannerError::UnsafeExecutable(path.into(), error.kind()))?;
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| RawJobPlannerError::UnsafeExecutable(path.into(), error.kind()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || canonical != path {
        return Err(RawJobPlannerError::UnsafeExecutable(
            path.into(),
            io::ErrorKind::InvalidInput,
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(RawJobPlannerError::UnsafeExecutable(
                path.into(),
                io::ErrorKind::PermissionDenied,
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawJobPlannerError {
    #[error("invalid Raw job request: {0}")]
    InvalidRequest(String),
    #[error("interactive Raw request cannot enter the line-oriented job runner")]
    InteractiveRequest,
    #[error("noninteractive Raw request cannot enter the PTY runner")]
    NoninteractiveRequest,
    #[error("Raw stdout and stderr stream identities must be distinct")]
    DuplicateStreamIdentity,
    #[error("Raw job authorization failed: {0}")]
    Authorization(String),
    #[error("Raw job preview digest or reviewed typed intent changed")]
    PreviewMismatch,
    #[error("initialized environment has no authoritative BitBake executable")]
    MissingExecutableAuthority,
    #[error("unsafe Raw build directory {0}: {1:?}")]
    UnsafeBuildDirectory(PathBuf, io::ErrorKind),
    #[error("unsafe Raw executable {0}: {1:?}")]
    UnsafeExecutable(PathBuf, io::ErrorKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawJobRunnerEvent {
    Started,
    Output(RawOutputChunk),
    Completed {
        exit_code: i32,
    },
    Failed {
        exit_code: Option<i32>,
        message: String,
    },
    TimedOut {
        forced: bool,
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

#[derive(Debug)]
enum RawJobPipeEvent {
    Output {
        stream: RawOutputStream,
        text: String,
        truncated_bytes: u64,
    },
    Failed {
        stream: RawOutputStream,
        message: String,
    },
}

async fn read_raw_job_output<R>(
    stream: R,
    kind: RawOutputStream,
    sender: tokio::sync::mpsc::Sender<RawJobPipeEvent>,
) where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut retained = Vec::new();
    let mut truncated_bytes = 0_u64;
    loop {
        let buffer = match reader.fill_buf().await {
            Ok(buffer) => buffer,
            Err(error) => {
                let _ = sender
                    .send(RawJobPipeEvent::Failed {
                        stream: kind,
                        message: error.to_string(),
                    })
                    .await;
                return;
            }
        };
        if buffer.is_empty() {
            if (!retained.is_empty() || truncated_bytes > 0)
                && sender
                    .send(RawJobPipeEvent::Output {
                        stream: kind,
                        text: bounded_raw_output_text(&retained, &mut truncated_bytes),
                        truncated_bytes,
                    })
                    .await
                    .is_err()
            {
                return;
            }
            return;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        let remaining = yoctui_model::MAX_RAW_OUTPUT_CHUNK_BYTES.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..take.min(remaining)]);
        truncated_bytes = truncated_bytes.saturating_add(take.saturating_sub(remaining) as u64);
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(RawJobPipeEvent::Output {
                    stream: kind,
                    text: bounded_raw_output_text(&retained, &mut truncated_bytes),
                    truncated_bytes,
                })
                .await
                .is_err()
            {
                return;
            }
            retained.clear();
            truncated_bytes = 0;
        }
    }
}

fn bounded_raw_output_text(bytes: &[u8], truncated_bytes: &mut u64) -> String {
    let mut text = output_text(bytes);
    if text.len() <= yoctui_model::MAX_RAW_OUTPUT_CHUNK_BYTES {
        return text;
    }
    let mut boundary = yoctui_model::MAX_RAW_OUTPUT_CHUNK_BYTES;
    while !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    *truncated_bytes = truncated_bytes.saturating_add((text.len() - boundary) as u64);
    text.truncate(boundary);
    text
}

pub struct RawJobRunner {
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<RawJobPipeEvent>>,
    streams_drained: bool,
    started_pending: bool,
    terminal_pending: Option<RawJobRunnerEvent>,
    stdout_stream: Option<RawStreamId>,
    stderr_stream: Option<RawStreamId>,
    stdout_sequence: u64,
    stderr_sequence: u64,
    deadline: Option<Instant>,
    operation_timeout: Duration,
    cancellation_timeout: Duration,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl Default for RawJobRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl RawJobRunner {
    pub fn new() -> Self {
        Self {
            child: None,
            output: None,
            streams_drained: true,
            started_pending: false,
            terminal_pending: None,
            stdout_stream: None,
            stderr_stream: None,
            stdout_sequence: 1,
            stderr_sequence: 1,
            deadline: None,
            operation_timeout: RAW_JOB_DEFAULT_TIMEOUT,
            cancellation_timeout: RAW_JOB_DEFAULT_CANCELLATION_TIMEOUT,
            cancellation_requested: false,
            #[cfg(unix)]
            process_group: None,
        }
    }

    pub fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }

    pub fn is_active(&self) -> bool {
        self.child.is_some()
    }

    pub async fn start(&mut self, spec: RawJobCommandSpec) -> Result<(), RawJobRunnerError> {
        if self.child.is_some()
            || self.output.is_some()
            || self.started_pending
            || self.terminal_pending.is_some()
        {
            return Err(RawJobRunnerError::Busy);
        }
        validate_directory(&spec.current_directory)
            .map_err(|error| RawJobRunnerError::Authorization(error.to_string()))?;
        validate_executable(&spec.executable)
            .map_err(|error| RawJobRunnerError::Authorization(error.to_string()))?;
        let mut process = Command::new(&spec.executable);
        process
            .args(&spec.arguments)
            .current_dir(&spec.current_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        process.process_group(0);
        let mut child = spawn_raw_job_process(&mut process)
            .await
            .map_err(|error| RawJobRunnerError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(RawJobRunnerError::StreamUnavailable(
                RawOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(RawJobRunnerError::StreamUnavailable(
                RawOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(RAW_JOB_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_raw_job_output(
            stdout,
            RawOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_raw_job_output(
            stderr,
            RawOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.stdout_stream = Some(spec.stdout_stream);
        self.stderr_stream = Some(spec.stderr_stream);
        self.stdout_sequence = 1;
        self.stderr_sequence = 1;
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.started_pending = true;
        self.deadline = Some(Instant::now() + self.operation_timeout);
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<RawJobRunnerEvent, RawJobRunnerError> {
        if self.started_pending {
            self.started_pending = false;
            return Ok(RawJobRunnerEvent::Started);
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            self.terminate_now().await;
            return Ok(RawJobRunnerEvent::Lost {
                message: "Raw job output event channel was lost".into(),
            });
        }
        if let Some(receiver) = self.output.as_mut() {
            let event = if let Some(deadline) = self.deadline {
                match tokio::time::timeout_at(deadline, receiver.recv()).await {
                    Ok(event) => event,
                    Err(_) => {
                        let (forced, exit_code) = self.terminate_with_grace(false).await?;
                        return Ok(RawJobRunnerEvent::TimedOut { forced, exit_code });
                    }
                }
            } else {
                receiver.recv().await
            };
            match event {
                Some(RawJobPipeEvent::Output {
                    stream,
                    text,
                    truncated_bytes,
                }) => return self.output_event(stream, text, truncated_bytes),
                Some(RawJobPipeEvent::Failed { stream, message }) => {
                    self.terminate_now().await;
                    return Ok(RawJobRunnerEvent::Lost {
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
            return Err(RawJobRunnerError::NotRunning);
        };
        let status = match child.wait().await {
            Ok(status) => status,
            Err(error) => {
                let message = format!("Raw job wait failed: {error}");
                self.clear_live();
                return Ok(RawJobRunnerEvent::Lost { message });
            }
        };
        self.clear_live();
        match status.code() {
            Some(0) => Ok(RawJobRunnerEvent::Completed { exit_code: 0 }),
            exit_code => Ok(RawJobRunnerEvent::Failed {
                exit_code,
                message: "Raw BitBake command exited unsuccessfully".into(),
            }),
        }
    }

    fn output_event(
        &mut self,
        stream: RawOutputStream,
        text: String,
        truncated_bytes: u64,
    ) -> Result<RawJobRunnerEvent, RawJobRunnerError> {
        let (stream_id, sequence) = match stream {
            RawOutputStream::Stdout => {
                let sequence = self.stdout_sequence;
                self.stdout_sequence = self
                    .stdout_sequence
                    .checked_add(1)
                    .ok_or(RawJobRunnerError::SequenceExhausted)?;
                (self.stdout_stream.clone(), sequence)
            }
            RawOutputStream::Stderr => {
                let sequence = self.stderr_sequence;
                self.stderr_sequence = self
                    .stderr_sequence
                    .checked_add(1)
                    .ok_or(RawJobRunnerError::SequenceExhausted)?;
                (self.stderr_stream.clone(), sequence)
            }
        };
        let chunk = RawOutputChunk {
            stream_id: stream_id.ok_or(RawJobRunnerError::NotRunning)?,
            stream,
            sequence,
            text,
            truncated_bytes,
            dropped_lines: 0,
        };
        chunk
            .validate()
            .map_err(|error| RawJobRunnerError::Output(error.to_string()))?;
        Ok(RawJobRunnerEvent::Output(chunk))
    }

    pub async fn cancel(&mut self) -> Result<bool, RawJobRunnerError> {
        if self.cancellation_requested || self.child.is_none() {
            return Ok(false);
        }
        self.cancellation_requested = true;
        let (forced, exit_code) = self.terminate_with_grace(true).await?;
        self.terminal_pending = Some(RawJobRunnerEvent::Cancelled { forced, exit_code });
        Ok(true)
    }

    async fn terminate_with_grace(
        &mut self,
        retain_buffered_output: bool,
    ) -> Result<(bool, Option<i32>), RawJobRunnerError> {
        let Some(child) = self.child.as_mut() else {
            return Err(RawJobRunnerError::NotRunning);
        };
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(process_group) = self.process_group {
            // SAFETY: this process group is the child PID created by `process_group(0)`.
            if unsafe { libc::kill(-process_group, libc::SIGTERM) } != 0 {
                child
                    .start_kill()
                    .map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                Ok(status) => {
                    status.map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?
                }
                Err(_) => {
                    // SAFETY: this is the same child-owned process group.
                    let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                    forced = true;
                    child
                        .wait()
                        .await
                        .map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?
                }
            }
        } else {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?
        };
        let exit_code = status.code();
        if retain_buffered_output {
            self.clear_process();
        } else {
            self.clear_live();
        }
        Ok((forced, exit_code))
    }

    async fn terminate_now(&mut self) {
        #[cfg(unix)]
        if let Some(process_group) = self.process_group {
            // SAFETY: this process group is the child PID created by `process_group(0)`.
            let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
        self.clear_live();
    }

    fn clear_live(&mut self) {
        self.clear_process();
        self.output = None;
        self.streams_drained = true;
    }

    fn clear_process(&mut self) {
        self.child = None;
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

impl Drop for RawJobRunner {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(process_group) = self.process_group {
            // SAFETY: this process group is the child PID created by `start`.
            let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }
}

async fn spawn_raw_job_process(process: &mut Command) -> io::Result<Child> {
    for attempt in 1..=RAW_JOB_SPAWN_ATTEMPTS {
        match process.spawn() {
            Ok(child) => return Ok(child),
            Err(error) if attempt < RAW_JOB_SPAWN_ATTEMPTS && transient_spawn_error(&error) => {
                tokio::time::sleep(RAW_JOB_SPAWN_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("bounded Raw job spawn loop always returns")
}

#[cfg(unix)]
fn transient_spawn_error(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ETXTBSY)
}

#[cfg(not(unix))]
fn transient_spawn_error(_error: &io::Error) -> bool {
    false
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawJobRunnerError {
    #[error("a Raw process or unconsumed terminal event is already active")]
    Busy,
    #[error("Raw command authorization became stale: {0}")]
    Authorization(String),
    #[error("could not start Raw command: {0}")]
    Spawn(String),
    #[error("Raw process stream is unavailable: {0:?}")]
    StreamUnavailable(RawOutputStream),
    #[error("Raw runner is not active")]
    NotRunning,
    #[error("Raw process control failed: {0}")]
    ProcessControl(String),
    #[error("Raw output sequence space is exhausted")]
    SequenceExhausted,
    #[error("Raw output was invalid: {0}")]
    Output(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, fs};
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, RawExecutionPolicy,
        RawParameterValue, ToolIdentity, YoctoEnvironmentIdentity,
    };

    struct Fixture(PathBuf);

    impl Fixture {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-raw-job-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn executable(&self, body: &str) -> PathBuf {
            let path = self.0.join("bitbake");
            crate::test_support::write_executable(&path, body);
            path
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fixture_authority(build: &Path, executable: &Path) -> DaemonCompatibilitySnapshot {
        let catalog = builtin_raw_catalog();
        let command = catalog
            .commands
            .iter()
            .find(|command| {
                matches!(
                    command.execution,
                    RawExecutionPolicy::Executable { ref template }
                        if template.interaction == RawInteractionMode::NoninteractiveJob
                )
            })
            .unwrap();
        let RawExecutionPolicy::Executable { template } = &command.execution else {
            unreachable!();
        };
        let required_capabilities = match &template.capabilities {
            yoctui_model::RawCapabilityRequirement::All { capabilities }
            | yoctui_model::RawCapabilityRequirement::Any { capabilities } => capabilities.clone(),
        };
        let capabilities = required_capabilities
            .iter()
            .copied()
            .map(|id| CapabilityRecord {
                id,
                state: CapabilityState::Available,
                evidence: vec![CapabilityEvidence {
                    kind: CapabilityEvidenceKind::DirectProbe,
                    outcome: CapabilityEvidenceOutcome::Positive,
                    subject: id.as_str().into(),
                    detail: "Raw job fixture".into(),
                    argv: vec!["bitbake".into(), "--help".into()],
                }],
            })
            .collect::<Vec<_>>();
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation: 7,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        build.into(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    available_tools: AuthoritativeValue::detected(
                        vec![ToolIdentity {
                            id: "bitbake".into(),
                            executable: executable.into(),
                            version: Some("2.18.0".into()),
                        }],
                        IdentityAuthority::ExecutableProbe,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities,
            },
            implementations: required_capabilities
                .into_iter()
                .map(|id| {
                    (
                        id,
                        CapabilityImplementation {
                            id: format!("{}.fixture", id.as_str()),
                            kind: CapabilityImplementationKind::Command,
                        },
                    )
                })
                .collect(),
        }
        .normalize()
        .unwrap()
    }

    fn request(authority: &DaemonCompatibilitySnapshot) -> RawConfirmedExecutionRequest {
        let catalog = builtin_raw_catalog();
        let command = catalog
            .commands
            .iter()
            .find(|command| {
                matches!(
                    command.execution,
                    RawExecutionPolicy::Executable { ref template }
                        if template.interaction == RawInteractionMode::NoninteractiveJob
                            && command.parameters.is_empty()
                )
            })
            .unwrap();
        let preview_request = RawPreviewRequest {
            catalog_version: catalog.version,
            command: command.id.clone(),
            parameters: BTreeMap::<_, RawParameterValue>::new(),
            additional_arguments: RawAdditionalArguments::from_vec(vec!["extra-target".into()])
                .unwrap(),
            capability_generation: authority.snapshot.generation,
            build_directory: authority
                .snapshot
                .environment
                .build_directory
                .value()
                .unwrap()
                .clone(),
        };
        let preview = catalog.preview(&preview_request, Some(authority)).unwrap();
        RawConfirmedExecutionRequest::from_reviewed_preview(
            RawRequestId::new("raw-request:fixture-1").unwrap(),
            catalog,
            &preview_request,
            &preview,
        )
        .unwrap()
    }

    fn spec(authority: &DaemonCompatibilitySnapshot) -> RawJobCommandSpec {
        RawJobPlanner::new(authority)
            .plan(
                &request(authority),
                RawJobId::new("raw-job:fixture-1").unwrap(),
                RawStreamId::new("raw-stream:fixture-stdout").unwrap(),
                RawStreamId::new("raw-stream:fixture-stderr").unwrap(),
            )
            .unwrap()
    }

    #[test]
    fn raw_job_planner_reconstructs_exact_native_argv_and_rejects_tampering_before_spawn() {
        let fixture = Fixture::new("planner");
        let executable = fixture.executable("#!/bin/sh\nexit 0\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let request = request(&authority);
        let command = RawJobPlanner::new(&authority)
            .plan(
                &request,
                RawJobId::new("raw-job:planner").unwrap(),
                RawStreamId::new("raw-stream:planner-out").unwrap(),
                RawStreamId::new("raw-stream:planner-err").unwrap(),
            )
            .unwrap();
        assert_eq!(command.executable(), executable);
        assert_eq!(command.current_directory(), fixture.0);
        assert_eq!(
            command.arguments().last().map(OsString::as_os_str),
            Some(std::ffi::OsStr::new("extra-target"))
        );
        assert!(!command.arguments().iter().any(|argument| argument == "sh"));

        let mut tampered = request;
        tampered.preview_digest.0[0] ^= 0xff;
        assert_eq!(
            RawJobPlanner::new(&authority).plan(
                &tampered,
                RawJobId::new("raw-job:tampered").unwrap(),
                RawStreamId::new("raw-stream:tampered-out").unwrap(),
                RawStreamId::new("raw-stream:tampered-err").unwrap(),
            ),
            Err(RawJobPlannerError::PreviewMismatch)
        );
    }

    #[tokio::test]
    async fn raw_job_runner_streams_bounded_unicode_and_reports_success_and_nonzero() {
        let fixture = Fixture::new("outcomes");
        let executable = fixture
            .executable("#!/bin/sh\nprintf 'hello 界\\n'\nprintf 'warning\\n' >&2\nexit 0\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let mut runner = RawJobRunner::new();
        runner.start(spec(&authority)).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            RawJobRunnerEvent::Started
        );
        let mut streams = Vec::new();
        loop {
            match runner.next_event().await.unwrap() {
                RawJobRunnerEvent::Output(chunk) => streams.push((chunk.stream, chunk.text)),
                RawJobRunnerEvent::Completed { exit_code } => {
                    assert_eq!(exit_code, 0);
                    break;
                }
                other => panic!("unexpected event: {other:?}"),
            }
        }
        assert!(streams.contains(&(RawOutputStream::Stdout, "hello 界".into())));
        assert!(streams.contains(&(RawOutputStream::Stderr, "warning".into())));

        let executable = fixture.executable("#!/bin/sh\nexit 9\n");
        let authority = fixture_authority(&fixture.0, &executable);
        runner.start(spec(&authority)).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            RawJobRunnerEvent::Started
        );
        assert!(matches!(
            runner.next_event().await.unwrap(),
            RawJobRunnerEvent::Failed {
                exit_code: Some(9),
                ..
            }
        ));
    }

    #[tokio::test]
    async fn raw_job_runner_cancels_gracefully_forcibly_times_out_and_reports_loss() {
        let fixture = Fixture::new("terminal");
        let executable = fixture
            .executable("#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let mut graceful = RawJobRunner::new().with_cancellation_timeout(Duration::from_secs(1));
        graceful.start(spec(&authority)).await.unwrap();
        graceful.next_event().await.unwrap();
        assert!(matches!(
            graceful.next_event().await.unwrap(),
            RawJobRunnerEvent::Output(_)
        ));
        assert!(graceful.cancel().await.unwrap());
        assert!(!graceful.cancel().await.unwrap());
        assert!(matches!(
            graceful.next_event().await.unwrap(),
            RawJobRunnerEvent::Cancelled { forced: false, .. }
        ));

        let executable =
            fixture.executable("#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let mut forced = RawJobRunner::new().with_cancellation_timeout(Duration::from_millis(20));
        forced.start(spec(&authority)).await.unwrap();
        forced.next_event().await.unwrap();
        assert!(matches!(
            forced.next_event().await.unwrap(),
            RawJobRunnerEvent::Output(_)
        ));
        assert!(forced.cancel().await.unwrap());
        assert!(matches!(
            forced.next_event().await.unwrap(),
            RawJobRunnerEvent::Cancelled { forced: true, .. }
        ));

        let executable = fixture.executable("#!/bin/sh\nwhile :; do sleep 1; done\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let mut timed = RawJobRunner::new()
            .with_operation_timeout(Duration::from_millis(20))
            .with_cancellation_timeout(Duration::from_millis(20));
        timed.start(spec(&authority)).await.unwrap();
        timed.next_event().await.unwrap();
        assert!(matches!(
            timed.next_event().await.unwrap(),
            RawJobRunnerEvent::TimedOut { .. }
        ));

        let executable = fixture.executable("#!/bin/sh\nwhile :; do sleep 1; done\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let mut lost = RawJobRunner::new();
        lost.start(spec(&authority)).await.unwrap();
        lost.next_event().await.unwrap();
        lost.lose_output_channel();
        assert!(matches!(
            lost.next_event().await.unwrap(),
            RawJobRunnerEvent::Lost { .. }
        ));
    }

    #[tokio::test]
    async fn raw_job_runner_truncates_oversized_lines_and_revalidates_at_start() {
        let fixture = Fixture::new("bounds");
        let executable = fixture.executable(&format!(
            "#!/bin/sh\nprintf '%s\\n' '{}'\n",
            "界".repeat(yoctui_model::MAX_RAW_OUTPUT_CHUNK_BYTES)
        ));
        let authority = fixture_authority(&fixture.0, &executable);
        let mut runner = RawJobRunner::new();
        runner.start(spec(&authority)).await.unwrap();
        runner.next_event().await.unwrap();
        let RawJobRunnerEvent::Output(chunk) = runner.next_event().await.unwrap() else {
            panic!("expected bounded output");
        };
        assert!(chunk.text.len() <= yoctui_model::MAX_RAW_OUTPUT_CHUNK_BYTES);
        assert!(chunk.truncated_bytes > 0);

        let command = spec(&authority);
        fs::remove_file(executable).unwrap();
        let mut denied = RawJobRunner::new();
        assert!(matches!(
            denied.start(command).await,
            Err(RawJobRunnerError::Authorization(_))
        ));
        assert!(!denied.is_active());

        let executable = fixture.executable("not an executable image\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let mut rejected = RawJobRunner::new();
        assert!(matches!(
            rejected.start(spec(&authority)).await,
            Err(RawJobRunnerError::Spawn(_))
        ));
        assert!(!rejected.is_active());
    }
}
