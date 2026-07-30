use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, SystemTime},
};

use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
    time::Instant,
};
use yoctui_model::{
    PtestCapability, TestCapability, TestExecutableCapability, TestFamily, TestOutputStream,
    TestSelftestRequest,
};

use crate::output_text;

const MAX_TEST_RUNNER_PATH_DIRECTORIES: usize = 256;
const MAX_TEST_RUNNER_LINE_BYTES: usize = 64 * 1024;
const TEST_RUNNER_EVENT_CHANNEL_CAPACITY: usize = 256;
const TEST_RUNNER_OPERATION_TIMEOUT: Duration = Duration::from_secs(12 * 60 * 60);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TestRunnerAdapterError {
    #[error("Testing PATH directory is unsafe: {0}")]
    UnsafePathDirectory(PathBuf),
    #[error("Testing executable is unsafe or unavailable: {0}")]
    UnsafeExecutable(PathBuf),
    #[error("Testing build directory is unsafe: {0}")]
    UnsafeBuildDirectory(PathBuf),
    #[error("Testing request is invalid: {0}")]
    InvalidRequest(String),
    #[error("Testing executable inspection failed: {0}")]
    Inspection(String),
    #[error("Testing executable identity changed before launch: {0}")]
    StaleExecutable(PathBuf),
    #[error("a Testing process or unconsumed event is already active")]
    Busy,
    #[error("could not start Testing process: {0}")]
    Spawn(String),
    #[error("Testing process stream is unavailable: {0:?}")]
    StreamUnavailable(TestOutputStream),
    #[error("Testing runner is not active")]
    NotRunning,
    #[error("Testing process control failed: {0}")]
    ProcessControl(String),
}

#[derive(Debug, Clone)]
pub struct TestRunnerCapabilityInspector {
    path_directories: Vec<PathBuf>,
    ptest: PtestCapability,
}

impl TestRunnerCapabilityInspector {
    pub fn new(path_directories: Vec<PathBuf>, ptest: PtestCapability) -> Self {
        Self {
            path_directories,
            ptest,
        }
    }

    pub fn inspect(&self) -> TestCapability {
        let directories = match validate_path_directories(&self.path_directories) {
            Ok(directories) => directories,
            Err(error) => {
                let message = error.to_string();
                return TestCapability {
                    oe_selftest: TestExecutableCapability::Failed(message.clone()),
                    bitbake_selftest: TestExecutableCapability::Failed(message),
                    ptest: self.ptest.clone(),
                };
            }
        };
        TestCapability {
            oe_selftest: discover_executable(&directories, "oe-selftest").map_or_else(
                TestExecutableCapability::Failed,
                |path| {
                    path.map_or(
                        TestExecutableCapability::Missing,
                        TestExecutableCapability::Available,
                    )
                },
            ),
            bitbake_selftest: discover_executable(&directories, "bitbake-selftest").map_or_else(
                TestExecutableCapability::Failed,
                |path| {
                    path.map_or(
                        TestExecutableCapability::Missing,
                        TestExecutableCapability::Available,
                    )
                },
            ),
            ptest: self.ptest.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestRunnerAdapter {
    build_directory: PathBuf,
    path_directories: Vec<PathBuf>,
    ptest: PtestCapability,
}

impl TestRunnerAdapter {
    pub fn new(
        build_directory: PathBuf,
        path_directories: Vec<PathBuf>,
        ptest: PtestCapability,
    ) -> Self {
        Self {
            build_directory,
            path_directories,
            ptest,
        }
    }

    pub fn capability(&self) -> TestCapability {
        TestRunnerCapabilityInspector::new(self.path_directories.clone(), self.ptest.clone())
            .inspect()
    }

    pub fn command(
        &self,
        request: &TestSelftestRequest,
    ) -> Result<TestCommandSpec, TestRunnerAdapterError> {
        let directories = validate_path_directories(&self.path_directories)?;
        let build_directory = validate_directory(
            &self.build_directory,
            TestRunnerAdapterError::UnsafeBuildDirectory(self.build_directory.clone()),
        )?;
        let expected_name = match request.family {
            TestFamily::OeSelftest => "oe-selftest",
            TestFamily::BitbakeSelftest => "bitbake-selftest",
            _ => {
                return Err(TestRunnerAdapterError::InvalidRequest(
                    "managed BitBake test families do not use the selftest runner".into(),
                ));
            }
        };
        if request.family == TestFamily::OeSelftest && (request.verbose || request.skip_network) {
            return Err(TestRunnerAdapterError::InvalidRequest(
                "OE selftest cannot carry BitBake-selftest-only choices".into(),
            ));
        }
        let discovered = discover_executable(&directories, expected_name)
            .map_err(TestRunnerAdapterError::Inspection)?
            .ok_or_else(|| TestRunnerAdapterError::UnsafeExecutable(request.executable.clone()))?;
        if discovered != request.executable {
            return Err(TestRunnerAdapterError::UnsafeExecutable(
                request.executable.clone(),
            ));
        }
        let reconstructed = TestSelftestRequest::new(
            request.executable.clone(),
            request.family,
            request.selector.clone(),
            request.parallelism,
            request.verbose,
            request.skip_network,
        )
        .map_err(|message| TestRunnerAdapterError::InvalidRequest(message.into()))?;
        if &reconstructed != request {
            return Err(TestRunnerAdapterError::InvalidRequest(
                "selftest request is not canonical".into(),
            ));
        }
        let argv = request.argv();
        if argv.first() != Some(&request.executable) {
            return Err(TestRunnerAdapterError::InvalidRequest(
                "selftest argv does not retain executable identity".into(),
            ));
        }
        let environment = test_child_environment(request);
        Ok(TestCommandSpec {
            request: request.clone(),
            executable: request.executable.clone(),
            arguments: argv
                .into_iter()
                .skip(1)
                .map(PathBuf::into_os_string)
                .collect(),
            current_directory: build_directory,
            environment,
            executable_identity: ExecutableIdentity::capture(&request.executable)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExecutableIdentity {
    canonical_path: PathBuf,
    size_bytes: u64,
    modified_at: SystemTime,
}

impl ExecutableIdentity {
    fn capture(path: &Path) -> Result<Self, TestRunnerAdapterError> {
        let metadata = safe_executable_metadata(path)?;
        let canonical_path = fs::canonicalize(path)
            .map_err(|_| TestRunnerAdapterError::UnsafeExecutable(path.into()))?;
        if canonical_path != path {
            return Err(TestRunnerAdapterError::UnsafeExecutable(path.into()));
        }
        Ok(Self {
            canonical_path,
            size_bytes: metadata.len(),
            modified_at: metadata
                .modified()
                .map_err(|_| TestRunnerAdapterError::UnsafeExecutable(path.into()))?,
        })
    }

    fn revalidate(&self) -> Result<(), TestRunnerAdapterError> {
        let current = Self::capture(&self.canonical_path)?;
        if current != *self {
            return Err(TestRunnerAdapterError::StaleExecutable(
                self.canonical_path.clone(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCommandSpec {
    request: TestSelftestRequest,
    executable: PathBuf,
    arguments: Vec<OsString>,
    current_directory: PathBuf,
    environment: BTreeMap<OsString, OsString>,
    executable_identity: ExecutableIdentity,
}

impl TestCommandSpec {
    pub fn request(&self) -> &TestSelftestRequest {
        &self.request
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

    pub fn environment(&self) -> &BTreeMap<OsString, OsString> {
        &self.environment
    }

    fn revalidate(&self) -> Result<(), TestRunnerAdapterError> {
        self.executable_identity.revalidate()?;
        let build = validate_directory(
            &self.current_directory,
            TestRunnerAdapterError::UnsafeBuildDirectory(self.current_directory.clone()),
        )?;
        if build != self.current_directory {
            return Err(TestRunnerAdapterError::UnsafeBuildDirectory(
                self.current_directory.clone(),
            ));
        }
        let reconstructed = TestSelftestRequest::new(
            self.executable.clone(),
            self.request.family,
            self.request.selector.clone(),
            self.request.parallelism,
            self.request.verbose,
            self.request.skip_network,
        )
        .map_err(|message| TestRunnerAdapterError::InvalidRequest(message.into()))?;
        let argv = reconstructed.argv();
        let arguments = argv
            .into_iter()
            .skip(1)
            .map(PathBuf::into_os_string)
            .collect::<Vec<_>>();
        if reconstructed != self.request || arguments != self.arguments {
            return Err(TestRunnerAdapterError::InvalidRequest(
                "Testing command no longer matches its typed request".into(),
            ));
        }
        let expected_environment = test_child_environment(&self.request);
        if self.environment != expected_environment {
            return Err(TestRunnerAdapterError::InvalidRequest(
                "Testing child environment is inconsistent".into(),
            ));
        }
        Ok(())
    }
}

fn test_child_environment(request: &TestSelftestRequest) -> BTreeMap<OsString, OsString> {
    if request.family == TestFamily::BitbakeSelftest && request.skip_network {
        BTreeMap::from([(OsString::from("BB_SKIP_NETTESTS"), OsString::from("yes"))])
    } else {
        BTreeMap::new()
    }
}

fn validate_path_directories(
    directories: &[PathBuf],
) -> Result<Vec<PathBuf>, TestRunnerAdapterError> {
    if directories.is_empty() || directories.len() > MAX_TEST_RUNNER_PATH_DIRECTORIES {
        return Err(TestRunnerAdapterError::UnsafePathDirectory(PathBuf::new()));
    }
    let mut validated = Vec::with_capacity(directories.len());
    for directory in directories {
        if !directory.is_absolute() {
            return Err(TestRunnerAdapterError::UnsafePathDirectory(
                directory.clone(),
            ));
        }
        let canonical = fs::canonicalize(directory)
            .map_err(|_| TestRunnerAdapterError::UnsafePathDirectory(directory.clone()))?;
        if !fs::metadata(&canonical)
            .map_err(|_| TestRunnerAdapterError::UnsafePathDirectory(directory.clone()))?
            .is_dir()
        {
            return Err(TestRunnerAdapterError::UnsafePathDirectory(
                directory.clone(),
            ));
        }
        if !validated.contains(&canonical) {
            validated.push(canonical);
        }
    }
    Ok(validated)
}

fn validate_directory(
    path: &Path,
    error: TestRunnerAdapterError,
) -> Result<PathBuf, TestRunnerAdapterError> {
    if !path.is_absolute() {
        return Err(error);
    }
    let link_metadata = fs::symlink_metadata(path).map_err(|_| error.clone())?;
    if link_metadata.file_type().is_symlink() || !link_metadata.is_dir() {
        return Err(error);
    }
    let canonical = fs::canonicalize(path).map_err(|_| error.clone())?;
    if canonical != path {
        return Err(error);
    }
    Ok(canonical)
}

fn discover_executable(directories: &[PathBuf], name: &str) -> Result<Option<PathBuf>, String> {
    for directory in directories {
        let candidate = directory.join(name);
        match fs::symlink_metadata(&candidate) {
            Ok(_) => {
                safe_executable_metadata(&candidate).map_err(|error| error.to_string())?;
                let canonical = fs::canonicalize(&candidate).map_err(|error| error.to_string())?;
                if canonical != candidate {
                    return Err(format!(
                        "Testing executable is not an exact canonical path: {}",
                        candidate.display()
                    ));
                }
                return Ok(Some(canonical));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Testing executable inspection failed for {}: {error}",
                    candidate.display()
                ));
            }
        }
    }
    Ok(None)
}

fn safe_executable_metadata(path: &Path) -> Result<fs::Metadata, TestRunnerAdapterError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| TestRunnerAdapterError::UnsafeExecutable(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(TestRunnerAdapterError::UnsafeExecutable(path.into()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(TestRunnerAdapterError::UnsafeExecutable(path.into()));
        }
    }
    Ok(metadata)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestRunnerEvent {
    Started,
    Output {
        stream: TestOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        exit_code: Option<i32>,
        result_paths: Vec<PathBuf>,
    },
    Failed {
        exit_code: Option<i32>,
    },
    Cancelled {
        forced: bool,
        exit_code: Option<i32>,
    },
    CancellationRejected {
        message: String,
    },
    TimedOut {
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        message: String,
    },
}

#[derive(Debug)]
enum TestPipeEvent {
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

async fn read_test_output<R>(
    stream: R,
    kind: TestOutputStream,
    sender: tokio::sync::mpsc::Sender<TestPipeEvent>,
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
                    .send(TestPipeEvent::Failed {
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
                    .send(TestPipeEvent::Output {
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
            let remaining = MAX_TEST_RUNNER_LINE_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(TestPipeEvent::Output {
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

pub struct TestRunnerJob {
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<TestPipeEvent>>,
    streams_drained: bool,
    started_pending: bool,
    terminal_pending: VecDeque<TestRunnerEvent>,
    cancellation_timeout: Duration,
    operation_timeout: Duration,
    deadline: Option<Instant>,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl Default for TestRunnerJob {
    fn default() -> Self {
        Self::new()
    }
}

impl TestRunnerJob {
    pub fn new() -> Self {
        Self {
            child: None,
            output: None,
            streams_drained: true,
            started_pending: false,
            terminal_pending: VecDeque::new(),
            cancellation_timeout: Duration::from_secs(5),
            operation_timeout: TEST_RUNNER_OPERATION_TIMEOUT,
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

    pub async fn start(&mut self, command: TestCommandSpec) -> Result<(), TestRunnerAdapterError> {
        if self.child.is_some()
            || self.started_pending
            || !self.terminal_pending.is_empty()
            || self.output.is_some()
        {
            return Err(TestRunnerAdapterError::Busy);
        }
        command.revalidate()?;
        let mut process = Command::new(&command.executable);
        process
            .args(&command.arguments)
            .current_dir(&command.current_directory)
            .envs(&command.environment)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        process.process_group(0);
        let mut child = process
            .spawn()
            .map_err(|error| TestRunnerAdapterError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            self.clear_process_state();
            return Err(TestRunnerAdapterError::StreamUnavailable(
                TestOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            self.clear_process_state();
            return Err(TestRunnerAdapterError::StreamUnavailable(
                TestOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(TEST_RUNNER_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_test_output(
            stdout,
            TestOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_test_output(
            stderr,
            TestOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.started_pending = true;
        self.deadline = Some(Instant::now() + self.operation_timeout);
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<TestRunnerEvent, TestRunnerAdapterError> {
        if self.started_pending {
            self.started_pending = false;
            return Ok(TestRunnerEvent::Started);
        }
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            self.kill_and_clear().await;
            return Ok(TestRunnerEvent::Lost {
                message: "Testing output event channel was lost".into(),
            });
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return self.timeout_active().await;
        }
        if let Some(receiver) = self.output.as_mut() {
            let deadline = self.deadline.ok_or(TestRunnerAdapterError::NotRunning)?;
            let event = tokio::select! {
                event = receiver.recv() => Some(event),
                _ = tokio::time::sleep_until(deadline) => None,
            };
            match event {
                Some(Some(TestPipeEvent::Output {
                    stream,
                    line,
                    truncated,
                })) => {
                    return Ok(TestRunnerEvent::Output {
                        stream,
                        line,
                        truncated,
                    });
                }
                Some(Some(TestPipeEvent::Failed { stream, message })) => {
                    self.kill_and_clear().await;
                    return Ok(TestRunnerEvent::Lost {
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
        let deadline = self.deadline.ok_or(TestRunnerAdapterError::NotRunning)?;
        let status = {
            let child = self
                .child
                .as_mut()
                .ok_or(TestRunnerAdapterError::NotRunning)?;
            match tokio::time::timeout_at(deadline, child.wait()).await {
                Ok(Ok(status)) => status,
                Ok(Err(error)) => {
                    self.kill_and_clear().await;
                    return Ok(TestRunnerEvent::Lost {
                        message: format!("Testing process wait failed: {error}"),
                    });
                }
                Err(_) => return self.timeout_active().await,
            }
        };
        self.clear_process_state();
        if status.success() {
            Ok(TestRunnerEvent::Completed {
                exit_code: status.code(),
                result_paths: Vec::new(),
            })
        } else {
            Ok(TestRunnerEvent::Failed {
                exit_code: status.code(),
            })
        }
    }

    pub async fn cancel(&mut self) -> Result<bool, TestRunnerAdapterError> {
        if self.cancellation_requested || self.child.is_none() {
            self.terminal_pending
                .push_back(TestRunnerEvent::CancellationRejected {
                    message: "no cancellable Testing process is active".into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state();
        self.terminal_pending.push_back(TestRunnerEvent::Cancelled {
            forced,
            exit_code: status.and_then(|status| status.code()),
        });
        Ok(true)
    }

    async fn timeout_active(&mut self) -> Result<TestRunnerEvent, TestRunnerAdapterError> {
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state();
        Ok(TestRunnerEvent::TimedOut {
            forced,
            exit_code: status.and_then(|status| status.code()),
        })
    }

    async fn terminate_active(
        &mut self,
    ) -> Result<(Option<std::process::ExitStatus>, bool), TestRunnerAdapterError> {
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
                        TestRunnerAdapterError::ProcessControl(error.to_string())
                    })?;
                    forced = true;
                }
                match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                    Ok(result) => Some(result.map_err(|error| {
                        TestRunnerAdapterError::ProcessControl(error.to_string())
                    })?),
                    Err(_) => {
                        // SAFETY: same child-owned process group as the graceful signal.
                        let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                        forced = true;
                        Some(child.wait().await.map_err(|error| {
                            TestRunnerAdapterError::ProcessControl(error.to_string())
                        })?)
                    }
                }
            } else {
                forced = true;
                child
                    .kill()
                    .await
                    .map_err(|error| TestRunnerAdapterError::ProcessControl(error.to_string()))?;
                Some(
                    child.wait().await.map_err(|error| {
                        TestRunnerAdapterError::ProcessControl(error.to_string())
                    })?,
                )
            };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| TestRunnerAdapterError::ProcessControl(error.to_string()))?;
            Some(
                child
                    .wait()
                    .await
                    .map_err(|error| TestRunnerAdapterError::ProcessControl(error.to_string()))?,
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

impl Drop for TestRunnerJob {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        ffi::OsStr,
        sync::atomic::{AtomicU64, Ordering},
    };

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-test-runner-{name}-{}-{}",
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

    fn fixture(name: &str) -> (TestDirectory, TestRunnerAdapter) {
        let directory = TestDirectory::new(name);
        let bin = directory.path().join("bin");
        let build = directory.path().join("build");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&build).unwrap();
        for tool in ["oe-selftest", "bitbake-selftest"] {
            executable(&bin.join(tool), "#!/bin/sh\nprintf '%s\\n' \"$@\"\n");
        }
        let adapter = TestRunnerAdapter::new(build, vec![bin], PtestCapability::Configured);
        (directory, adapter)
    }

    fn request(adapter: &TestRunnerAdapter, family: TestFamily) -> TestSelftestRequest {
        let executable = adapter.capability().executable_for(family).unwrap();
        TestSelftestRequest::new(
            executable,
            family,
            (family == TestFamily::OeSelftest).then(|| "tinfoil.Case.test_one".into()),
            4,
            family == TestFamily::BitbakeSelftest,
            family == TestFamily::BitbakeSelftest,
        )
        .unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn test_runner_capability_distinguishes_missing_and_unsafe_executables() {
        let directory = TestDirectory::new("capability");
        let bin = directory.path().join("bin");
        fs::create_dir(&bin).unwrap();
        executable(&bin.join("oe-selftest"), "#!/bin/sh\nexit 0\n");
        let inspector =
            TestRunnerCapabilityInspector::new(vec![bin.clone()], PtestCapability::Configured);
        assert!(matches!(
            inspector.inspect(),
            TestCapability {
                oe_selftest: TestExecutableCapability::Available(_),
                bitbake_selftest: TestExecutableCapability::Missing,
                ptest: PtestCapability::Configured,
            }
        ));

        let outside = directory.path().join("outside");
        executable(&outside, "#!/bin/sh\nexit 0\n");
        symlink(&outside, bin.join("bitbake-selftest")).unwrap();
        assert!(matches!(
            inspector.inspect().bitbake_selftest,
            TestExecutableCapability::Failed(_)
        ));
        assert!(matches!(
            TestRunnerCapabilityInspector::new(
                vec![directory.path().join("missing")],
                PtestCapability::NotInspected
            )
            .inspect()
            .oe_selftest,
            TestExecutableCapability::Failed(_)
        ));
    }

    #[test]
    fn test_runner_commands_are_exact_revalidated_and_child_environment_only() {
        let (_directory, adapter) = fixture("commands");
        let oe = request(&adapter, TestFamily::OeSelftest);
        let oe_command = adapter.command(&oe).unwrap();
        assert_eq!(
            oe_command.arguments(),
            ["-r", "tinfoil.Case.test_one", "-j", "4"]
        );
        assert!(oe_command.environment().is_empty());

        let bitbake = request(&adapter, TestFamily::BitbakeSelftest);
        let command = adapter.command(&bitbake).unwrap();
        assert_eq!(command.arguments(), ["-v"]);
        assert_eq!(
            command.environment().get(OsStr::new("BB_SKIP_NETTESTS")),
            Some(&OsString::from("yes"))
        );
        assert!(std::env::var_os("BB_SKIP_NETTESTS").is_none());

        let mut invalid = oe;
        invalid.skip_network = true;
        assert!(matches!(
            adapter.command(&invalid),
            Err(TestRunnerAdapterError::InvalidRequest(_))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_runner_rejects_tampering_streams_bounded_output_and_completes() {
        let (_directory, adapter) = fixture("stream");
        let request = request(&adapter, TestFamily::BitbakeSelftest);
        let tool = request.executable.clone();
        let command = adapter.command(&request).unwrap();
        executable(&tool, "#!/bin/sh\nexit 0\n");
        let mut stale = TestRunnerJob::new();
        assert!(matches!(
            stale.start(command).await,
            Err(TestRunnerAdapterError::StaleExecutable(_))
        ));

        executable(
            &tool,
            &format!(
                "#!/bin/sh\nprintf 'env=%s\\n' \"$BB_SKIP_NETTESTS\"\nprintf 'stderr\\n' >&2\nprintf '{}\\n'\nexit 0\n",
                "x".repeat(MAX_TEST_RUNNER_LINE_BYTES + 8)
            ),
        );
        let refreshed = TestSelftestRequest::new(
            tool,
            request.family,
            request.selector,
            request.parallelism,
            request.verbose,
            request.skip_network,
        )
        .unwrap();
        let command = adapter.command(&refreshed).unwrap();
        let mut runner = TestRunnerJob::new();
        runner.start(command.clone()).await.unwrap();
        assert_eq!(
            runner.start(command).await,
            Err(TestRunnerAdapterError::Busy)
        );
        assert_eq!(runner.next_event().await.unwrap(), TestRunnerEvent::Started);
        let mut stdout = false;
        let mut stderr = false;
        let mut truncated = false;
        loop {
            match runner.next_event().await.unwrap() {
                TestRunnerEvent::Output {
                    stream,
                    line,
                    truncated: line_truncated,
                } => {
                    stdout |= stream == TestOutputStream::Stdout;
                    stderr |= stream == TestOutputStream::Stderr;
                    truncated |= line_truncated;
                    if line.starts_with("env=") {
                        assert_eq!(line, "env=yes");
                    }
                }
                TestRunnerEvent::Completed {
                    exit_code,
                    result_paths,
                } => {
                    assert_eq!(exit_code, Some(0));
                    assert!(result_paths.is_empty());
                    break;
                }
                event => panic!("unexpected event: {event:?}"),
            }
        }
        assert!(stdout && stderr && truncated);
        assert!(std::env::var_os("BB_SKIP_NETTESTS").is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_runner_reports_nonzero_worker_loss_and_cancellation_rejection() {
        let (_directory, adapter) = fixture("outcomes");
        let mut request = request(&adapter, TestFamily::OeSelftest);
        executable(&request.executable, "#!/bin/sh\nexit 7\n");
        request = TestSelftestRequest::new(
            request.executable,
            request.family,
            request.selector,
            request.parallelism,
            request.verbose,
            request.skip_network,
        )
        .unwrap();
        let command = adapter.command(&request).unwrap();
        let mut runner = TestRunnerJob::new();
        runner.start(command).await.unwrap();
        assert_eq!(runner.next_event().await.unwrap(), TestRunnerEvent::Started);
        assert_eq!(
            runner.next_event().await.unwrap(),
            TestRunnerEvent::Failed { exit_code: Some(7) }
        );
        assert!(!runner.cancel().await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            TestRunnerEvent::CancellationRejected { .. }
        ));

        executable(&request.executable, "#!/bin/sh\nsleep 2\n");
        let request = TestSelftestRequest::new(
            request.executable,
            request.family,
            request.selector,
            request.parallelism,
            request.verbose,
            request.skip_network,
        )
        .unwrap();
        runner
            .start(adapter.command(&request).unwrap())
            .await
            .unwrap();
        assert_eq!(runner.next_event().await.unwrap(), TestRunnerEvent::Started);
        runner.lose_output_channel();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            TestRunnerEvent::Lost { .. }
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_runner_cancels_gracefully_forcibly_and_times_out() {
        let (_directory, adapter) = fixture("control");
        let mut request = request(&adapter, TestFamily::OeSelftest);
        executable(
            &request.executable,
            "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n",
        );
        request = TestSelftestRequest::new(
            request.executable,
            request.family,
            request.selector,
            request.parallelism,
            request.verbose,
            request.skip_network,
        )
        .unwrap();
        let mut graceful = TestRunnerJob::new().with_cancellation_timeout(Duration::from_secs(1));
        graceful
            .start(adapter.command(&request).unwrap())
            .await
            .unwrap();
        assert_eq!(
            graceful.next_event().await.unwrap(),
            TestRunnerEvent::Started
        );
        assert!(matches!(
            graceful.next_event().await.unwrap(),
            TestRunnerEvent::Output { .. }
        ));
        assert!(graceful.cancel().await.unwrap());
        assert!(matches!(
            graceful.next_event().await.unwrap(),
            TestRunnerEvent::Cancelled { forced: false, .. }
        ));

        executable(
            &request.executable,
            "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
        );
        let request = TestSelftestRequest::new(
            request.executable,
            request.family,
            request.selector,
            request.parallelism,
            request.verbose,
            request.skip_network,
        )
        .unwrap();
        let command = adapter.command(&request).unwrap();
        let mut forced = TestRunnerJob::new().with_cancellation_timeout(Duration::from_millis(20));
        forced.start(command.clone()).await.unwrap();
        assert_eq!(forced.next_event().await.unwrap(), TestRunnerEvent::Started);
        assert!(matches!(
            forced.next_event().await.unwrap(),
            TestRunnerEvent::Output { .. }
        ));
        assert!(forced.cancel().await.unwrap());
        assert!(matches!(
            forced.next_event().await.unwrap(),
            TestRunnerEvent::Cancelled { forced: true, .. }
        ));

        let mut timed_out = TestRunnerJob::new()
            .with_cancellation_timeout(Duration::from_millis(20))
            .with_operation_timeout(Duration::from_millis(20));
        timed_out.start(command).await.unwrap();
        assert_eq!(
            timed_out.next_event().await.unwrap(),
            TestRunnerEvent::Started
        );
        assert!(matches!(
            timed_out.next_event().await.unwrap(),
            TestRunnerEvent::Output { .. }
        ));
        assert!(matches!(
            timed_out.next_event().await.unwrap(),
            TestRunnerEvent::TimedOut { forced: true, .. }
        ));
    }
}
