use std::{
    collections::{BTreeSet, VecDeque},
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, SystemTime},
};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
    time::Instant,
};
use yoctui_model::{
    ResultToolCapability, TestCaseIdentity, TestCaseOutcome, TestCaseRecord, TestComparisonPreview,
    TestComparisonRequest, TestFamily, TestJunitDestinationInspection, TestJunitExportPreview,
    TestJunitExportRequest, TestMetadata, TestOutputStream, TestResultIdentity,
    TestResultImportRequest, TestResultRecord, TestSuiteRecord, normalize_test_results,
};

use crate::{
    output_text,
    test_runner::{discover_executable, validate_path_directories},
};

const MAX_RESULT_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_RESULT_SCAN_DIRECTORIES: usize = 1_024;
const MAX_RESULT_SCAN_ENTRIES: usize = 16_384;
const MAX_RESULT_FILES: usize = 256;
const MAX_RESULTTOOL_LINE_BYTES: usize = 64 * 1024;
const RESULTTOOL_EVENT_CHANNEL_CAPACITY: usize = 256;
const RESULTTOOL_OPERATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TestResultAdapterError {
    #[error("resulttool capability inspection failed: {0}")]
    Capability(String),
    #[error("test-result root is unsafe or unavailable: {0}")]
    UnsafeRoot(PathBuf),
    #[error("test-result file is unsafe or unavailable: {0}")]
    UnsafeResult(PathBuf),
    #[error("test-result input exceeds a safety bound: {0}")]
    Bound(String),
    #[error("test-result JSON is malformed: {0}")]
    Malformed(String),
    #[error("resulttool executable identity is stale or unsafe: {0}")]
    UnsafeResultTool(PathBuf),
    #[error("test-result identity changed before operation: {0}")]
    StaleResult(PathBuf),
    #[error("resulttool preview does not match its exact reconstructed command")]
    PreviewMismatch,
    #[error("JUnit destination is unsafe or would overwrite data: {0}")]
    UnsafeDestination(PathBuf),
    #[error("a resulttool process or unconsumed event is already active")]
    Busy,
    #[error("could not start resulttool: {0}")]
    Spawn(String),
    #[error("resulttool process stream is unavailable: {0:?}")]
    StreamUnavailable(TestOutputStream),
    #[error("resulttool runner is not active")]
    NotRunning,
    #[error("resulttool process control failed: {0}")]
    ProcessControl(String),
}

#[derive(Debug, Clone)]
pub struct ResultToolCapabilityInspector {
    path_directories: Vec<PathBuf>,
}

impl ResultToolCapabilityInspector {
    pub fn new(path_directories: Vec<PathBuf>) -> Self {
        Self { path_directories }
    }

    pub fn inspect(&self) -> ResultToolCapability {
        let directories = match validate_path_directories(&self.path_directories) {
            Ok(directories) => directories,
            Err(error) => return ResultToolCapability::Failed(error.to_string()),
        };
        discover_executable(&directories, "resulttool").map_or_else(
            ResultToolCapability::Failed,
            |path| {
                path.map_or(
                    ResultToolCapability::Missing,
                    ResultToolCapability::Available,
                )
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResultImportResponse {
    pub request: TestResultImportRequest,
    pub records: Vec<TestResultRecord>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TestResultAdapter {
    path_directories: Vec<PathBuf>,
}

impl TestResultAdapter {
    pub fn new(path_directories: Vec<PathBuf>) -> Self {
        Self { path_directories }
    }

    pub fn capability(&self) -> ResultToolCapability {
        ResultToolCapabilityInspector::new(self.path_directories.clone()).inspect()
    }

    pub fn inspect_junit_destination(
        &self,
        destination: PathBuf,
    ) -> TestJunitDestinationInspection {
        let destination_metadata = fs::symlink_metadata(&destination).ok();
        let parent = destination.parent().map(Path::to_path_buf);
        let parent_metadata = parent
            .as_deref()
            .and_then(|path| fs::symlink_metadata(path).ok());
        TestJunitDestinationInspection {
            requested: destination,
            canonical_parent: parent
                .as_deref()
                .and_then(|path| fs::canonicalize(path).ok()),
            parent_exists: parent_metadata.is_some(),
            parent_is_directory: parent_metadata
                .as_ref()
                .is_some_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink()),
            destination_exists: destination_metadata.is_some(),
            destination_is_symlink: destination_metadata
                .as_ref()
                .is_some_and(|metadata| metadata.file_type().is_symlink()),
        }
    }

    pub fn import(
        &self,
        request: &TestResultImportRequest,
    ) -> Result<TestResultImportResponse, TestResultAdapterError> {
        let validated = TestResultImportRequest::new(request.generation, request.roots.clone())
            .map_err(|message| TestResultAdapterError::Malformed(message.into()))?;
        if &validated != request {
            return Err(TestResultAdapterError::Malformed(
                "test-result import request is not canonical".into(),
            ));
        }
        let paths = collect_result_files(&request.roots)?;
        let mut records = Vec::new();
        let mut limitations = Vec::new();
        for path in paths {
            match parse_result_file(&path) {
                Ok((record, file_limitations)) => {
                    records.push(record);
                    limitations.extend(file_limitations);
                }
                Err(error @ TestResultAdapterError::Bound(_))
                | Err(error @ TestResultAdapterError::Malformed(_)) => {
                    limitations.push(format!("skipped {}: {error}", path.display()));
                }
                Err(error) => return Err(error),
            }
        }
        let (records, limitations) = normalize_test_results(records, limitations);
        Ok(TestResultImportResponse {
            request: request.clone(),
            records,
            limitations,
        })
    }

    pub fn comparison_command(
        &self,
        preview: &TestComparisonPreview,
        baseline: &TestResultRecord,
        candidate: &TestResultRecord,
    ) -> Result<TestResultCommandSpec, TestResultAdapterError> {
        let executable = self.resulttool_executable()?;
        validate_result_identity(&baseline.identity)?;
        validate_result_identity(&candidate.identity)?;
        if preview.request.baseline != baseline.identity
            || preview.request.candidate != candidate.identity
        {
            return Err(TestResultAdapterError::PreviewMismatch);
        }
        let expected = TestComparisonPreview::new(executable.clone(), preview.request.clone())
            .map_err(|_| TestResultAdapterError::PreviewMismatch)?;
        if expected != *preview {
            return Err(TestResultAdapterError::PreviewMismatch);
        }
        Ok(TestResultCommandSpec {
            operation: TestResultOperation::Comparison(preview.request.clone()),
            executable: executable.clone(),
            arguments: expected
                .argv
                .iter()
                .skip(1)
                .map(|value| value.as_os_str().to_owned())
                .collect(),
            current_directory: baseline
                .identity
                .path
                .parent()
                .expect("validated result has a parent")
                .into(),
            executable_identity: FileIdentity::capture_executable(&executable)?,
            result_identities: vec![
                FileIdentity::capture_result(&baseline.identity)?,
                FileIdentity::capture_result(&candidate.identity)?,
            ],
            destination_parent: None,
        })
    }

    pub fn junit_command(
        &self,
        preview: &TestJunitExportPreview,
        result: &TestResultRecord,
    ) -> Result<TestResultCommandSpec, TestResultAdapterError> {
        let executable = self.resulttool_executable()?;
        validate_result_identity(&result.identity)?;
        if preview.request.result != result.identity {
            return Err(TestResultAdapterError::PreviewMismatch);
        }
        let parent = validate_junit_destination(&preview.request.destination)?;
        let expected = TestJunitExportPreview::new(executable.clone(), preview.request.clone())
            .map_err(|_| TestResultAdapterError::PreviewMismatch)?;
        if expected != *preview {
            return Err(TestResultAdapterError::PreviewMismatch);
        }
        Ok(TestResultCommandSpec {
            operation: TestResultOperation::Junit(preview.request.clone()),
            executable: executable.clone(),
            arguments: expected
                .argv
                .iter()
                .skip(1)
                .map(|value| value.as_os_str().to_owned())
                .collect(),
            current_directory: parent.clone(),
            executable_identity: FileIdentity::capture_executable(&executable)?,
            result_identities: vec![FileIdentity::capture_result(&result.identity)?],
            destination_parent: Some(parent),
        })
    }

    fn resulttool_executable(&self) -> Result<PathBuf, TestResultAdapterError> {
        match self.capability() {
            ResultToolCapability::Available(path) => Ok(path),
            ResultToolCapability::Missing => Err(TestResultAdapterError::Capability(
                "resulttool is missing".into(),
            )),
            ResultToolCapability::NotInspected => Err(TestResultAdapterError::Capability(
                "resulttool was not inspected".into(),
            )),
            ResultToolCapability::Failed(message) => {
                Err(TestResultAdapterError::Capability(message))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestResultOperation {
    Comparison(TestComparisonRequest),
    Junit(TestJunitExportRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResultCommandSpec {
    operation: TestResultOperation,
    executable: PathBuf,
    arguments: Vec<OsString>,
    current_directory: PathBuf,
    executable_identity: FileIdentity,
    result_identities: Vec<FileIdentity>,
    destination_parent: Option<PathBuf>,
}

impl TestResultCommandSpec {
    pub fn operation(&self) -> &TestResultOperation {
        &self.operation
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

    fn revalidate(&self) -> Result<(), TestResultAdapterError> {
        self.executable_identity.revalidate_executable()?;
        for identity in &self.result_identities {
            identity.revalidate_result()?;
        }
        match &self.operation {
            TestResultOperation::Comparison(request) => {
                let expected = vec![
                    OsString::from("regression-file"),
                    request.baseline.path.as_os_str().to_owned(),
                    request.candidate.path.as_os_str().to_owned(),
                ];
                if self.arguments != expected || self.destination_parent.is_some() {
                    return Err(TestResultAdapterError::PreviewMismatch);
                }
            }
            TestResultOperation::Junit(request) => {
                let parent = validate_junit_destination(&request.destination)?;
                let expected = vec![
                    OsString::from("junit"),
                    request.result.path.as_os_str().to_owned(),
                    OsString::from("-j"),
                    request.destination.as_os_str().to_owned(),
                ];
                if self.arguments != expected || self.destination_parent.as_ref() != Some(&parent) {
                    return Err(TestResultAdapterError::PreviewMismatch);
                }
            }
        }
        Ok(())
    }

    pub fn comparison(&self) -> Option<TestComparisonRequest> {
        match &self.operation {
            TestResultOperation::Comparison(request) => Some(request.clone()),
            TestResultOperation::Junit(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileIdentity {
    path: PathBuf,
    size_bytes: u64,
    modified_at: SystemTime,
    fingerprint: Option<String>,
}

impl FileIdentity {
    fn capture_executable(path: &Path) -> Result<Self, TestResultAdapterError> {
        let metadata = safe_regular_file(path)
            .map_err(|_| TestResultAdapterError::UnsafeResultTool(path.into()))?;
        if !is_executable(&metadata) {
            return Err(TestResultAdapterError::UnsafeResultTool(path.into()));
        }
        Ok(Self {
            path: path.into(),
            size_bytes: metadata.len(),
            modified_at: metadata
                .modified()
                .map_err(|_| TestResultAdapterError::UnsafeResultTool(path.into()))?,
            fingerprint: None,
        })
    }

    fn capture_result(identity: &TestResultIdentity) -> Result<Self, TestResultAdapterError> {
        validate_result_identity(identity)?;
        Ok(Self {
            path: identity.path.clone(),
            size_bytes: identity.byte_size,
            modified_at: identity.modified_at,
            fingerprint: Some(identity.fingerprint.clone()),
        })
    }

    fn revalidate_executable(&self) -> Result<(), TestResultAdapterError> {
        let current = Self::capture_executable(&self.path)?;
        if current != *self {
            return Err(TestResultAdapterError::UnsafeResultTool(self.path.clone()));
        }
        Ok(())
    }

    fn revalidate_result(&self) -> Result<(), TestResultAdapterError> {
        let identity = TestResultIdentity::new(
            self.path.clone(),
            self.size_bytes,
            self.modified_at,
            self.fingerprint.clone().unwrap_or_default(),
        )
        .map_err(|_| TestResultAdapterError::StaleResult(self.path.clone()))?;
        validate_result_identity(&identity)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestResultRunnerEvent {
    Started {
        operation: TestResultOperation,
    },
    Output {
        stream: TestOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        operation: TestResultOperation,
        exit_code: Option<i32>,
    },
    Failed {
        operation: TestResultOperation,
        exit_code: Option<i32>,
    },
    Cancelled {
        operation: TestResultOperation,
        forced: bool,
        exit_code: Option<i32>,
    },
    CancellationRejected {
        message: String,
    },
    TimedOut {
        operation: TestResultOperation,
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        operation: Option<TestResultOperation>,
        message: String,
    },
}

#[derive(Debug)]
enum ResultToolPipeEvent {
    Output {
        stream: TestOutputStream,
        line: String,
        truncated: bool,
    },
    Failed {
        stream: TestOutputStream,
        message: String,
    },
}

async fn read_resulttool_output<R>(
    stream: R,
    kind: TestOutputStream,
    sender: tokio::sync::mpsc::Sender<ResultToolPipeEvent>,
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
                    .send(ResultToolPipeEvent::Failed {
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
                    .send(ResultToolPipeEvent::Output {
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
            let remaining = MAX_RESULTTOOL_LINE_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(ResultToolPipeEvent::Output {
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

pub struct TestResultJob {
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<ResultToolPipeEvent>>,
    streams_drained: bool,
    started_pending: bool,
    terminal_pending: VecDeque<TestResultRunnerEvent>,
    operation: Option<TestResultOperation>,
    cancellation_timeout: Duration,
    operation_timeout: Duration,
    deadline: Option<Instant>,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl Default for TestResultJob {
    fn default() -> Self {
        Self::new()
    }
}

impl TestResultJob {
    pub fn new() -> Self {
        Self {
            child: None,
            output: None,
            streams_drained: true,
            started_pending: false,
            terminal_pending: VecDeque::new(),
            operation: None,
            cancellation_timeout: Duration::from_secs(5),
            operation_timeout: RESULTTOOL_OPERATION_TIMEOUT,
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

    pub async fn start(
        &mut self,
        command: TestResultCommandSpec,
    ) -> Result<(), TestResultAdapterError> {
        if self.child.is_some()
            || self.started_pending
            || !self.terminal_pending.is_empty()
            || self.output.is_some()
        {
            return Err(TestResultAdapterError::Busy);
        }
        command.revalidate()?;
        let operation = command.operation.clone();
        let mut process = Command::new(&command.executable);
        process
            .args(&command.arguments)
            .current_dir(&command.current_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        process.process_group(0);
        let mut child = process
            .spawn()
            .map_err(|error| TestResultAdapterError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            self.clear_process_state();
            return Err(TestResultAdapterError::StreamUnavailable(
                TestOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            self.clear_process_state();
            return Err(TestResultAdapterError::StreamUnavailable(
                TestOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(RESULTTOOL_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_resulttool_output(
            stdout,
            TestOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_resulttool_output(
            stderr,
            TestOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.started_pending = true;
        self.operation = Some(operation);
        self.deadline = Some(Instant::now() + self.operation_timeout);
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<TestResultRunnerEvent, TestResultAdapterError> {
        if self.started_pending {
            self.started_pending = false;
            return Ok(TestResultRunnerEvent::Started {
                operation: self.operation()?,
            });
        }
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            let operation = self.operation.clone();
            self.kill_and_clear().await;
            return Ok(TestResultRunnerEvent::Lost {
                operation,
                message: "resulttool output event channel was lost".into(),
            });
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return self.timeout_active().await;
        }
        if let Some(receiver) = self.output.as_mut() {
            let deadline = self.deadline.ok_or(TestResultAdapterError::NotRunning)?;
            let event = tokio::select! {
                event = receiver.recv() => Some(event),
                _ = tokio::time::sleep_until(deadline) => None,
            };
            match event {
                Some(Some(ResultToolPipeEvent::Output {
                    stream,
                    line,
                    truncated,
                })) => {
                    return Ok(TestResultRunnerEvent::Output {
                        stream,
                        line,
                        truncated,
                    });
                }
                Some(Some(ResultToolPipeEvent::Failed { stream, message })) => {
                    let operation = self.operation.clone();
                    self.kill_and_clear().await;
                    return Ok(TestResultRunnerEvent::Lost {
                        operation,
                        message: format!("{stream:?} stream failed: {message}"),
                    });
                }
                Some(None) => {
                    self.output = None;
                    self.streams_drained = true;
                }
                None => return self.timeout_active().await,
            }
        }
        let deadline = self.deadline.ok_or(TestResultAdapterError::NotRunning)?;
        let status = {
            let child = self
                .child
                .as_mut()
                .ok_or(TestResultAdapterError::NotRunning)?;
            match tokio::time::timeout_at(deadline, child.wait()).await {
                Ok(Ok(status)) => status,
                Ok(Err(error)) => {
                    let operation = self.operation.clone();
                    self.kill_and_clear().await;
                    return Ok(TestResultRunnerEvent::Lost {
                        operation,
                        message: format!("resulttool process wait failed: {error}"),
                    });
                }
                Err(_) => return self.timeout_active().await,
            }
        };
        let operation = self.operation()?;
        self.clear_process_state();
        if status.success() {
            Ok(TestResultRunnerEvent::Completed {
                operation,
                exit_code: status.code(),
            })
        } else {
            Ok(TestResultRunnerEvent::Failed {
                operation,
                exit_code: status.code(),
            })
        }
    }

    pub async fn cancel(&mut self) -> Result<bool, TestResultAdapterError> {
        if self.cancellation_requested || self.child.is_none() {
            self.terminal_pending
                .push_back(TestResultRunnerEvent::CancellationRejected {
                    message: "no cancellable resulttool process is active".into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        let operation = self.operation()?;
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state();
        self.terminal_pending
            .push_back(TestResultRunnerEvent::Cancelled {
                operation,
                forced,
                exit_code: status.and_then(|status| status.code()),
            });
        Ok(true)
    }

    fn operation(&self) -> Result<TestResultOperation, TestResultAdapterError> {
        self.operation
            .clone()
            .ok_or(TestResultAdapterError::NotRunning)
    }

    async fn timeout_active(&mut self) -> Result<TestResultRunnerEvent, TestResultAdapterError> {
        let operation = self.operation()?;
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state();
        Ok(TestResultRunnerEvent::TimedOut {
            operation,
            forced,
            exit_code: status.and_then(|status| status.code()),
        })
    }

    async fn terminate_active(
        &mut self,
    ) -> Result<(Option<std::process::ExitStatus>, bool), TestResultAdapterError> {
        let Some(child) = self.child.as_mut() else {
            return Ok((None, false));
        };
        let mut forced = false;
        #[cfg(unix)]
        let status =
            if let Some(process_group) = self.process_group {
                // SAFETY: the negative PID targets only the child-owned process group.
                if unsafe { libc::kill(-process_group, libc::SIGTERM) } != 0 {
                    child.start_kill().map_err(|error| {
                        TestResultAdapterError::ProcessControl(error.to_string())
                    })?;
                    forced = true;
                }
                match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                    Ok(result) => Some(result.map_err(|error| {
                        TestResultAdapterError::ProcessControl(error.to_string())
                    })?),
                    Err(_) => {
                        // SAFETY: same child-owned process group as the graceful signal.
                        let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                        forced = true;
                        Some(child.wait().await.map_err(|error| {
                            TestResultAdapterError::ProcessControl(error.to_string())
                        })?)
                    }
                }
            } else {
                forced = true;
                child
                    .kill()
                    .await
                    .map_err(|error| TestResultAdapterError::ProcessControl(error.to_string()))?;
                Some(
                    child.wait().await.map_err(|error| {
                        TestResultAdapterError::ProcessControl(error.to_string())
                    })?,
                )
            };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| TestResultAdapterError::ProcessControl(error.to_string()))?;
            Some(
                child
                    .wait()
                    .await
                    .map_err(|error| TestResultAdapterError::ProcessControl(error.to_string()))?,
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
        self.child = None;
        self.output = None;
        self.streams_drained = true;
        self.operation = None;
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

impl Drop for TestResultJob {
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

fn collect_result_files(roots: &[PathBuf]) -> Result<Vec<PathBuf>, TestResultAdapterError> {
    let mut files = BTreeSet::new();
    let mut directories_seen = 0usize;
    let mut entries_seen = 0usize;
    for root in roots {
        let metadata = fs::symlink_metadata(root)
            .map_err(|_| TestResultAdapterError::UnsafeRoot(root.clone()))?;
        if metadata.file_type().is_symlink() {
            return Err(TestResultAdapterError::UnsafeRoot(root.clone()));
        }
        let canonical =
            fs::canonicalize(root).map_err(|_| TestResultAdapterError::UnsafeRoot(root.clone()))?;
        if canonical != *root {
            return Err(TestResultAdapterError::UnsafeRoot(root.clone()));
        }
        if metadata.is_file() {
            if root.file_name().and_then(|value| value.to_str()) != Some("testresults.json") {
                return Err(TestResultAdapterError::UnsafeResult(root.clone()));
            }
            files.insert(root.clone());
            continue;
        }
        if !metadata.is_dir() {
            return Err(TestResultAdapterError::UnsafeRoot(root.clone()));
        }
        let mut pending = vec![root.clone()];
        while let Some(directory) = pending.pop() {
            directories_seen += 1;
            if directories_seen > MAX_RESULT_SCAN_DIRECTORIES {
                return Err(TestResultAdapterError::Bound(
                    "too many result directories".into(),
                ));
            }
            let mut entries = fs::read_dir(&directory)
                .map_err(|_| TestResultAdapterError::UnsafeRoot(directory.clone()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| TestResultAdapterError::UnsafeRoot(directory.clone()))?;
            entries.sort_by_key(fs::DirEntry::file_name);
            for entry in entries {
                entries_seen += 1;
                if entries_seen > MAX_RESULT_SCAN_ENTRIES {
                    return Err(TestResultAdapterError::Bound(
                        "too many result directory entries".into(),
                    ));
                }
                let path = entry.path();
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|_| TestResultAdapterError::UnsafeRoot(path.clone()))?;
                if metadata.file_type().is_symlink() {
                    continue;
                }
                if metadata.is_dir() {
                    pending.push(path);
                } else if metadata.is_file()
                    && path.file_name().and_then(|value| value.to_str()) == Some("testresults.json")
                {
                    files.insert(path);
                    if files.len() > MAX_RESULT_FILES {
                        return Err(TestResultAdapterError::Bound(
                            "too many testresults.json files".into(),
                        ));
                    }
                }
            }
        }
    }
    Ok(files.into_iter().collect())
}

fn parse_result_file(
    path: &Path,
) -> Result<(TestResultRecord, Vec<String>), TestResultAdapterError> {
    let metadata = safe_regular_file(path)?;
    if metadata.len() == 0 || metadata.len() > MAX_RESULT_FILE_BYTES {
        return Err(TestResultAdapterError::Bound(format!(
            "{} has an invalid byte size",
            path.display()
        )));
    }
    let bytes = fs::read(path).map_err(|_| TestResultAdapterError::UnsafeResult(path.into()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| TestResultAdapterError::Malformed(error.to_string()))?;
    let runs = value.as_object().ok_or_else(|| {
        TestResultAdapterError::Malformed("top-level result must be an object".into())
    })?;
    let identity = TestResultIdentity::new(
        path.into(),
        metadata.len(),
        metadata
            .modified()
            .map_err(|_| TestResultAdapterError::UnsafeResult(path.into()))?,
        fingerprint(&bytes),
    )
    .map_err(|_| TestResultAdapterError::UnsafeResult(path.into()))?;
    let mut suites = Vec::new();
    let mut limitations = Vec::new();
    let mut all_metadata = Vec::new();
    let mut family = None;
    let mut machine = None;
    let mut image = None;
    let mut revision = None;
    for (run_id, run) in runs {
        let Some(run) = run.as_object() else {
            limitations.push(format!("skipped malformed run {run_id}"));
            continue;
        };
        let configuration = run.get("configuration").and_then(Value::as_object);
        let cases = run.get("result").and_then(Value::as_object);
        let Some(cases) = cases else {
            limitations.push(format!("skipped run {run_id} without a result object"));
            continue;
        };
        if let Some(configuration) = configuration {
            family = family.or_else(|| family_from_configuration(configuration));
            machine = machine.or_else(|| configuration_string(configuration, "MACHINE"));
            image = image.or_else(|| {
                configuration_string(configuration, "IMAGE_BASENAME")
                    .or_else(|| configuration_string(configuration, "IMAGE_NAME"))
            });
            revision = revision.or_else(|| {
                configuration_string(configuration, "OECOREREV")
                    .or_else(|| configuration_string(configuration, "revision"))
            });
            all_metadata.extend(configuration_metadata(
                run_id,
                configuration,
                &mut limitations,
            ));
        }
        let mut typed_cases = Vec::new();
        for (case_name, case) in cases {
            match parse_case(path, run_id, case_name, case) {
                Ok((case, case_limitations)) => {
                    typed_cases.push(case);
                    limitations.extend(case_limitations);
                }
                Err(message) => {
                    limitations.push(format!(
                        "skipped malformed case {run_id}/{case_name}: {message}"
                    ));
                }
            }
        }
        let (suite, suite_limitations) =
            TestSuiteRecord::new(run_id.clone(), None, Vec::new(), typed_cases)
                .map_err(|message| TestResultAdapterError::Malformed(message.into()))?;
        suites.push(suite);
        limitations.extend(suite_limitations);
    }
    if suites.is_empty() {
        return Err(TestResultAdapterError::Malformed(
            "result file contains no typed result runs".into(),
        ));
    }
    let (record, _normalization) = TestResultRecord::new(
        identity,
        family,
        machine,
        image,
        revision,
        None,
        all_metadata,
        suites,
        None,
        limitations,
    );
    let response_limitations = record.limitations.clone();
    Ok((record, response_limitations))
}

fn parse_case(
    result_path: &Path,
    suite: &str,
    name: &str,
    value: &Value,
) -> Result<(TestCaseRecord, Vec<String>), String> {
    let case = value
        .as_object()
        .ok_or_else(|| "case value is not an object".to_string())?;
    let identity = TestCaseIdentity::new(suite.into(), name.into()).map_err(str::to_owned)?;
    let outcome = case
        .get("status")
        .and_then(Value::as_str)
        .map(|status| match status.to_ascii_uppercase().as_str() {
            "PASSED" | "PASS" => TestCaseOutcome::Passed,
            "FAILED" | "FAIL" | "EXPECTEDFAIL" => TestCaseOutcome::Failed,
            "SKIPPED" | "SKIP" => TestCaseOutcome::Skipped,
            "ERROR" => TestCaseOutcome::Error,
            _ => TestCaseOutcome::Unknown,
        })
        .unwrap_or(TestCaseOutcome::Unknown);
    let duration = case
        .get("duration")
        .and_then(Value::as_f64)
        .filter(|duration| duration.is_finite() && *duration >= 0.0)
        .and_then(|duration| Duration::try_from_secs_f64(duration).ok());
    let mut limitations = Vec::new();
    let log_path = case
        .get("log_path")
        .and_then(Value::as_str)
        .and_then(|value| {
            let path = PathBuf::from(value);
            match safe_regular_file(&path) {
                Ok(_) if path.is_absolute() => Some(path),
                _ => {
                    limitations.push(format!(
                        "ignored unsafe related log for {}",
                        result_path.display()
                    ));
                    None
                }
            }
        });
    let metadata = case
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "status" | "duration" | "log_path"))
        .filter_map(|(key, value)| {
            scalar_string(value)
                .and_then(|value| TestMetadata::new(key.clone(), value).map_err(|_| ()).ok())
        })
        .collect();
    TestCaseRecord::new(identity, outcome, duration, metadata, log_path)
        .map_err(str::to_owned)
        .map(|(record, normalized)| {
            limitations.extend(normalized);
            (record, limitations)
        })
}

fn configuration_metadata(
    run_id: &str,
    configuration: &Map<String, Value>,
    limitations: &mut Vec<String>,
) -> Vec<TestMetadata> {
    configuration
        .iter()
        .filter_map(|(key, value)| {
            let value = scalar_string(value)?;
            match TestMetadata::new(format!("{run_id}.{key}"), value) {
                Ok(metadata) => Some(metadata),
                Err(_) => {
                    limitations.push(format!(
                        "ignored oversized configuration field {run_id}.{key}"
                    ));
                    None
                }
            }
        })
        .collect()
}

fn family_from_configuration(configuration: &Map<String, Value>) -> Option<TestFamily> {
    let test_type = configuration_string(configuration, "TEST_TYPE")?.to_ascii_lowercase();
    match test_type.as_str() {
        "runtime" | "testimage" => Some(TestFamily::TestImage),
        "sdk" | "testsdk" => Some(TestFamily::TestSdk),
        "sdkext" | "testsdkext" => Some(TestFamily::TestSdkExt),
        "ptest" => Some(TestFamily::Ptest),
        "oeselftest" | "oe-selftest" | "selftest" => Some(TestFamily::OeSelftest),
        "bitbake-selftest" | "bitbakeselftest" => Some(TestFamily::BitbakeSelftest),
        _ => None,
    }
}

fn configuration_string(configuration: &Map<String, Value>, key: &str) -> Option<String> {
    configuration.get(key).and_then(scalar_string)
}

fn scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn safe_regular_file(path: &Path) -> Result<fs::Metadata, TestResultAdapterError> {
    if !path.is_absolute() {
        return Err(TestResultAdapterError::UnsafeResult(path.into()));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| TestResultAdapterError::UnsafeResult(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(TestResultAdapterError::UnsafeResult(path.into()));
    }
    let canonical =
        fs::canonicalize(path).map_err(|_| TestResultAdapterError::UnsafeResult(path.into()))?;
    if canonical != path {
        return Err(TestResultAdapterError::UnsafeResult(path.into()));
    }
    Ok(metadata)
}

fn validate_result_identity(identity: &TestResultIdentity) -> Result<(), TestResultAdapterError> {
    if !identity.is_valid() {
        return Err(TestResultAdapterError::StaleResult(identity.path.clone()));
    }
    let metadata = safe_regular_file(&identity.path)
        .map_err(|_| TestResultAdapterError::StaleResult(identity.path.clone()))?;
    let bytes = fs::read(&identity.path)
        .map_err(|_| TestResultAdapterError::StaleResult(identity.path.clone()))?;
    if metadata.len() != identity.byte_size
        || metadata.modified().ok() != Some(identity.modified_at)
        || fingerprint(&bytes) != identity.fingerprint
    {
        return Err(TestResultAdapterError::StaleResult(identity.path.clone()));
    }
    Ok(())
}

fn validate_junit_destination(destination: &Path) -> Result<PathBuf, TestResultAdapterError> {
    if !destination.is_absolute()
        || destination.extension().and_then(|value| value.to_str()) != Some("xml")
    {
        return Err(TestResultAdapterError::UnsafeDestination(
            destination.into(),
        ));
    }
    match fs::symlink_metadata(destination) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        _ => {
            return Err(TestResultAdapterError::UnsafeDestination(
                destination.into(),
            ));
        }
    }
    let parent = destination
        .parent()
        .ok_or_else(|| TestResultAdapterError::UnsafeDestination(destination.into()))?;
    let metadata = fs::symlink_metadata(parent)
        .map_err(|_| TestResultAdapterError::UnsafeDestination(destination.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(TestResultAdapterError::UnsafeDestination(
            destination.into(),
        ));
    }
    let canonical = fs::canonicalize(parent)
        .map_err(|_| TestResultAdapterError::UnsafeDestination(destination.into()))?;
    if canonical != parent {
        return Err(TestResultAdapterError::UnsafeDestination(
            destination.into(),
        ));
    }
    Ok(canonical)
}

fn fingerprint(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        ffi::OsStr,
        sync::atomic::{AtomicU64, Ordering},
    };
    use yoctui_model::{TestComparison, TestJunitDestinationInspection, TestResultInventoryState};

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-test-results-{name}-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn executable(path: &Path, body: &str) {
        fs::write(path, body).unwrap();
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).unwrap();
        }
    }

    fn result_json(status: &str) -> String {
        format!(
            r#"{{
                "runtime-qemu": {{
                    "configuration": {{
                        "TEST_TYPE": "runtime",
                        "MACHINE": "qemux86-64",
                        "IMAGE_BASENAME": "core-image-minimal",
                        "OECOREREV": "abc123"
                    }},
                    "result": {{
                        "runtime.Case.test_one": {{
                            "status": "{status}",
                            "duration": 1.25,
                            "log": "bounded diagnostic"
                        }}
                    }}
                }}
            }}"#
        )
    }

    fn fixture(name: &str) -> (TestDirectory, TestResultAdapter, PathBuf, PathBuf) {
        let directory = TestDirectory::new(name);
        let bin = directory.path().join("bin");
        let results = directory.path().join("results");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&results).unwrap();
        let resulttool = bin.join("resulttool");
        executable(&resulttool, "#!/bin/sh\nprintf '%s\\n' \"$@\"\nexit 0\n");
        let adapter = TestResultAdapter::new(vec![bin]);
        (directory, adapter, resulttool, results)
    }

    fn imported(
        adapter: &TestResultAdapter,
        generation: u64,
        paths: Vec<PathBuf>,
    ) -> TestResultImportResponse {
        adapter
            .import(&TestResultImportRequest::new(generation, paths).unwrap())
            .unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn test_results_capability_is_independent_canonical_and_fail_closed() {
        let directory = TestDirectory::new("capability");
        let bin = directory.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let inspector = ResultToolCapabilityInspector::new(vec![bin.clone()]);
        assert_eq!(inspector.inspect(), ResultToolCapability::Missing);

        let tool = bin.join("resulttool");
        executable(&tool, "#!/bin/sh\nexit 0\n");
        assert_eq!(
            inspector.inspect(),
            ResultToolCapability::Available(tool.clone())
        );

        fs::remove_file(&tool).unwrap();
        let outside = directory.path().join("outside-resulttool");
        executable(&outside, "#!/bin/sh\nexit 0\n");
        symlink(&outside, &tool).unwrap();
        assert!(matches!(
            inspector.inspect(),
            ResultToolCapability::Failed(_)
        ));
        assert!(matches!(
            ResultToolCapabilityInspector::new(vec![directory.path().join("missing")]).inspect(),
            ResultToolCapability::Failed(_)
        ));
    }

    #[test]
    fn test_results_imports_official_shape_and_preserves_partial_failures() {
        let (_directory, adapter, _tool, results) = fixture("import");
        let valid = results.join("valid").join("testresults.json");
        let malformed = results.join("malformed").join("testresults.json");
        let empty = results.join("empty").join("testresults.json");
        for path in [&valid, &malformed, &empty] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        fs::write(&valid, result_json("PASSED")).unwrap();
        fs::write(&malformed, b"{not json").unwrap();
        fs::write(&empty, b"{}").unwrap();

        let response = imported(&adapter, 7, vec![results]);
        assert_eq!(response.records.len(), 1);
        let record = &response.records[0];
        assert_eq!(record.family, Some(TestFamily::TestImage));
        assert_eq!(record.machine.as_deref(), Some("qemux86-64"));
        assert_eq!(record.image.as_deref(), Some("core-image-minimal"));
        assert_eq!(record.revision.as_deref(), Some("abc123"));
        assert_eq!(record.counts().passed, 1);
        assert_eq!(record.identity.fingerprint.len(), 64);
        assert!(record.is_valid());
        assert_eq!(response.limitations.len(), 2);
        assert!(
            response
                .limitations
                .iter()
                .any(|message| message.contains("malformed"))
        );
        assert!(
            response
                .limitations
                .iter()
                .any(|message| message.contains("no typed result runs"))
        );
        let state = if response.limitations.is_empty() {
            TestResultInventoryState::Available {
                request: response.request,
                records: response.records,
            }
        } else {
            TestResultInventoryState::Partial {
                request: response.request,
                records: response.records,
                limitations: response.limitations,
            }
        };
        assert!(matches!(state, TestResultInventoryState::Partial { .. }));
    }

    #[test]
    fn test_results_import_rejects_unsafe_and_bounds_oversized_files() {
        let (directory, adapter, _tool, results) = fixture("bounds");
        let oversized = results.join("testresults.json");
        let file = fs::File::create(&oversized).unwrap();
        file.set_len(MAX_RESULT_FILE_BYTES + 1).unwrap();
        let response = imported(&adapter, 1, vec![oversized]);
        assert!(response.records.is_empty());
        assert_eq!(response.limitations.len(), 1);
        assert!(response.limitations[0].contains("invalid byte size"));

        let wrong_name = directory.path().join("arbitrary.json");
        fs::write(&wrong_name, result_json("PASSED")).unwrap();
        let request = TestResultImportRequest::new(2, vec![wrong_name]).unwrap();
        assert!(matches!(
            adapter.import(&request),
            Err(TestResultAdapterError::UnsafeResult(_))
        ));
    }

    #[test]
    fn test_results_construct_exact_comparison_and_non_overwriting_junit_vectors() {
        let (directory, adapter, tool, results) = fixture("vectors");
        let baseline_path = results.join("baseline").join("testresults.json");
        let candidate_path = results.join("candidate").join("testresults.json");
        for path in [&baseline_path, &candidate_path] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        fs::write(&baseline_path, result_json("PASSED")).unwrap();
        fs::write(&candidate_path, result_json("FAILED")).unwrap();
        let response = imported(
            &adapter,
            1,
            vec![baseline_path.clone(), candidate_path.clone()],
        );
        let baseline = response
            .records
            .iter()
            .find(|record| record.identity.path == baseline_path)
            .unwrap();
        let candidate = response
            .records
            .iter()
            .find(|record| record.identity.path == candidate_path)
            .unwrap();
        let comparison = TestComparison::between(baseline, candidate).unwrap();
        assert_eq!(comparison.baseline, baseline.identity);
        assert_eq!(comparison.candidate, candidate.identity);
        let request =
            TestComparisonRequest::new(4, baseline.identity.clone(), candidate.identity.clone())
                .unwrap();
        let preview = TestComparisonPreview::new(tool.clone(), request.clone()).unwrap();
        let command = adapter
            .comparison_command(&preview, baseline, candidate)
            .unwrap();
        assert_eq!(
            command.arguments(),
            [
                OsStr::new("regression-file"),
                baseline_path.as_os_str(),
                candidate_path.as_os_str()
            ]
        );
        assert_eq!(
            command.operation(),
            &TestResultOperation::Comparison(request)
        );

        let export_directory = directory.path().join("export");
        fs::create_dir(&export_directory).unwrap();
        let destination = export_directory.join("results.xml");
        let inspection = TestJunitDestinationInspection {
            requested: destination.clone(),
            canonical_parent: Some(export_directory),
            parent_exists: true,
            parent_is_directory: true,
            destination_exists: false,
            destination_is_symlink: false,
        };
        let request =
            TestJunitExportRequest::new(5, candidate.identity.clone(), &inspection).unwrap();
        let preview = TestJunitExportPreview::new(tool, request.clone()).unwrap();
        let command = adapter.junit_command(&preview, candidate).unwrap();
        assert_eq!(
            command.arguments(),
            [
                OsStr::new("junit"),
                candidate_path.as_os_str(),
                OsStr::new("-j"),
                destination.as_os_str()
            ]
        );
        fs::write(&destination, b"do not overwrite").unwrap();
        assert!(matches!(
            command.revalidate(),
            Err(TestResultAdapterError::UnsafeDestination(_))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_results_revalidate_tampering_stream_output_and_nonzero() {
        let (_directory, adapter, tool, results) = fixture("runner");
        let baseline_path = results.join("baseline").join("testresults.json");
        let candidate_path = results.join("candidate").join("testresults.json");
        for path in [&baseline_path, &candidate_path] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        fs::write(&baseline_path, result_json("PASSED")).unwrap();
        fs::write(&candidate_path, result_json("FAILED")).unwrap();
        let response = imported(
            &adapter,
            1,
            vec![baseline_path.clone(), candidate_path.clone()],
        );
        let baseline = &response.records[0];
        let candidate = &response.records[1];
        let request =
            TestComparisonRequest::new(1, baseline.identity.clone(), candidate.identity.clone())
                .unwrap();
        let preview = TestComparisonPreview::new(tool.clone(), request.clone()).unwrap();
        let stale = adapter
            .comparison_command(&preview, baseline, candidate)
            .unwrap();
        fs::write(&candidate.identity.path, result_json("ERROR")).unwrap();
        assert!(matches!(
            TestResultJob::new().start(stale).await,
            Err(TestResultAdapterError::StaleResult(_))
        ));

        let response = imported(
            &adapter,
            2,
            vec![baseline_path.clone(), candidate_path.clone()],
        );
        let baseline = &response.records[0];
        let candidate = &response.records[1];
        let request =
            TestComparisonRequest::new(2, baseline.identity.clone(), candidate.identity.clone())
                .unwrap();
        executable(
            &tool,
            "#!/bin/sh\nprintf 'stdout\\n'\nprintf 'stderr\\n' >&2\nexit 7\n",
        );
        let preview = TestComparisonPreview::new(tool, request.clone()).unwrap();
        let command = adapter
            .comparison_command(&preview, baseline, candidate)
            .unwrap();
        let mut runner = TestResultJob::new();
        runner.start(command.clone()).await.unwrap();
        assert_eq!(
            runner.start(command).await,
            Err(TestResultAdapterError::Busy)
        );
        assert_eq!(
            runner.next_event().await.unwrap(),
            TestResultRunnerEvent::Started {
                operation: TestResultOperation::Comparison(request.clone())
            }
        );
        let mut stdout = false;
        let mut stderr = false;
        loop {
            match runner.next_event().await.unwrap() {
                TestResultRunnerEvent::Output { stream, .. } => {
                    stdout |= stream == TestOutputStream::Stdout;
                    stderr |= stream == TestOutputStream::Stderr;
                }
                TestResultRunnerEvent::Failed {
                    operation,
                    exit_code,
                } => {
                    assert_eq!(operation, TestResultOperation::Comparison(request));
                    assert_eq!(exit_code, Some(7));
                    break;
                }
                event => panic!("unexpected resulttool event: {event:?}"),
            }
        }
        assert!(stdout && stderr);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_results_report_cancellation_timeout_and_worker_loss() {
        let (_directory, adapter, tool, results) = fixture("control");
        let baseline_path = results.join("baseline").join("testresults.json");
        let candidate_path = results.join("candidate").join("testresults.json");
        for path in [&baseline_path, &candidate_path] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        fs::write(&baseline_path, result_json("PASSED")).unwrap();
        fs::write(&candidate_path, result_json("FAILED")).unwrap();
        let response = imported(&adapter, 1, vec![baseline_path, candidate_path]);
        let baseline = &response.records[0];
        let candidate = &response.records[1];
        let request =
            TestComparisonRequest::new(1, baseline.identity.clone(), candidate.identity.clone())
                .unwrap();

        executable(
            &tool,
            "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n",
        );
        let preview = TestComparisonPreview::new(tool.clone(), request.clone()).unwrap();
        let command = adapter
            .comparison_command(&preview, baseline, candidate)
            .unwrap();
        let mut cancelled = TestResultJob::new().with_cancellation_timeout(Duration::from_secs(1));
        cancelled.start(command).await.unwrap();
        let _ = cancelled.next_event().await.unwrap();
        let _ = cancelled.next_event().await.unwrap();
        assert!(cancelled.cancel().await.unwrap());
        assert!(matches!(
            cancelled.next_event().await.unwrap(),
            TestResultRunnerEvent::Cancelled { forced: false, .. }
        ));

        executable(
            &tool,
            "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
        );
        let preview = TestComparisonPreview::new(tool.clone(), request.clone()).unwrap();
        let command = adapter
            .comparison_command(&preview, baseline, candidate)
            .unwrap();
        let mut timed_out = TestResultJob::new()
            .with_cancellation_timeout(Duration::from_millis(20))
            .with_operation_timeout(Duration::from_millis(20));
        timed_out.start(command.clone()).await.unwrap();
        let _ = timed_out.next_event().await.unwrap();
        loop {
            if matches!(
                timed_out.next_event().await.unwrap(),
                TestResultRunnerEvent::TimedOut { forced: true, .. }
            ) {
                break;
            }
        }

        let mut lost = TestResultJob::new();
        lost.start(command).await.unwrap();
        let _ = lost.next_event().await.unwrap();
        lost.lose_output_channel();
        assert!(matches!(
            lost.next_event().await.unwrap(),
            TestResultRunnerEvent::Lost { .. }
        ));
        assert!(!lost.cancel().await.unwrap());
        assert!(matches!(
            lost.next_event().await.unwrap(),
            TestResultRunnerEvent::CancellationRejected { .. }
        ));
    }
}
