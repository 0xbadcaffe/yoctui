use std::{
    collections::VecDeque,
    ffi::{OsStr, OsString},
    fs,
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
    MAX_SECURITY_PATHS, MAX_SECURITY_TEXT_BYTES, SecurityOperation, SecurityOperationPreview,
    SecurityOutputStream, SecuritySessionId,
};

use crate::output_text;

const MAX_SECURITY_MAPPER_ARGUMENTS: usize = 64;
const MAX_SECURITY_MAPPER_LINE_BYTES: usize = MAX_SECURITY_TEXT_BYTES;
const SECURITY_MAPPER_EVENT_CHANNEL_CAPACITY: usize = 256;
const SECURITY_MAPPER_OPERATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SecurityMapperAdapterError {
    #[error("Security package-mapping preview is invalid: {0}")]
    InvalidPreview(String),
    #[error("Security package-mapping preview was modified after confirmation")]
    PreviewMismatch,
    #[error("Security package-mapping executable is unsafe: {0}")]
    UnsafeExecutable(PathBuf),
    #[error("Security package-mapping input is unsafe: {0}")]
    UnsafeInput(PathBuf),
    #[error("Security package-mapping identity became stale: {0}")]
    StaleIdentity(PathBuf),
    #[error("a Security package-mapping process or unconsumed event is already active")]
    Busy,
    #[error("could not start Security package mapping: {0}")]
    Spawn(String),
    #[error("Security package-mapping process stream is unavailable: {0:?}")]
    StreamUnavailable(SecurityOutputStream),
    #[error("Security package-mapping runner is not active")]
    NotRunning,
    #[error("Security package-mapping process control failed: {0}")]
    ProcessControl(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileIdentity {
    path: PathBuf,
    size_bytes: u64,
    modified_at: SystemTime,
    directory: bool,
}

impl FileIdentity {
    fn executable(path: &Path) -> Result<Self, SecurityMapperAdapterError> {
        if path.file_name() != Some(OsStr::new("cve-check-map-pkgs")) {
            return Err(SecurityMapperAdapterError::UnsafeExecutable(path.into()));
        }
        let metadata = safe_metadata(path, false)
            .map_err(|_| SecurityMapperAdapterError::UnsafeExecutable(path.into()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 == 0 {
                return Err(SecurityMapperAdapterError::UnsafeExecutable(path.into()));
            }
        }
        Self::from_metadata(path, metadata, false)
            .map_err(|_| SecurityMapperAdapterError::UnsafeExecutable(path.into()))
    }

    fn input(path: &Path) -> Result<Self, SecurityMapperAdapterError> {
        let metadata = safe_metadata(path, true)
            .map_err(|_| SecurityMapperAdapterError::UnsafeInput(path.into()))?;
        let directory = metadata.is_dir();
        Self::from_metadata(path, metadata, directory)
            .map_err(|_| SecurityMapperAdapterError::UnsafeInput(path.into()))
    }

    fn from_metadata(path: &Path, metadata: fs::Metadata, directory: bool) -> Result<Self, ()> {
        let canonical = fs::canonicalize(path).map_err(|_| ())?;
        if canonical != path {
            return Err(());
        }
        Ok(Self {
            path: canonical,
            size_bytes: metadata.len(),
            modified_at: metadata.modified().map_err(|_| ())?,
            directory,
        })
    }

    fn revalidate_executable(&self) -> Result<(), SecurityMapperAdapterError> {
        let current = Self::executable(&self.path)?;
        if current != *self {
            return Err(SecurityMapperAdapterError::StaleIdentity(self.path.clone()));
        }
        Ok(())
    }

    fn revalidate_input(&self) -> Result<(), SecurityMapperAdapterError> {
        let current = Self::input(&self.path)?;
        if current != *self {
            return Err(SecurityMapperAdapterError::StaleIdentity(self.path.clone()));
        }
        Ok(())
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityMapperCommandSpec {
    id: SecuritySessionId,
    preview: SecurityOperationPreview,
    executable: PathBuf,
    arguments: Vec<OsString>,
    current_directory: PathBuf,
    executable_identity: FileIdentity,
    input_identities: Vec<FileIdentity>,
}

impl SecurityMapperCommandSpec {
    pub fn from_preview(
        preview: &SecurityOperationPreview,
    ) -> Result<Self, SecurityMapperAdapterError> {
        if preview.id.0 == 0
            || !preview.scope.is_valid()
            || preview.report_roots.is_empty()
            || preview.report_roots.len() > MAX_SECURITY_PATHS
        {
            return Err(SecurityMapperAdapterError::InvalidPreview(
                "session, scope, or report roots are invalid".into(),
            ));
        }
        let SecurityOperation::PackageMap {
            executable,
            arguments,
        } = &preview.operation
        else {
            return Err(SecurityMapperAdapterError::InvalidPreview(
                "operation is not package mapping".into(),
            ));
        };
        if arguments.is_empty() || arguments.len() > MAX_SECURITY_MAPPER_ARGUMENTS {
            return Err(SecurityMapperAdapterError::InvalidPreview(
                "package-mapping arguments are empty or excessive".into(),
            ));
        }
        if arguments.iter().any(|argument| {
            argument.is_empty()
                || argument.len() > MAX_SECURITY_TEXT_BYTES
                || argument.chars().any(char::is_control)
        }) {
            return Err(SecurityMapperAdapterError::InvalidPreview(
                "package-mapping arguments contain an invalid field".into(),
            ));
        }
        let executable_identity = FileIdentity::executable(executable)?;
        let mut report_roots = preview
            .report_roots
            .iter()
            .map(|path| FileIdentity::input(path))
            .collect::<Result<Vec<_>, _>>()?;
        report_roots.sort_by(|left, right| left.path.cmp(&right.path));
        report_roots.dedup_by(|left, right| left.path == right.path);
        if report_roots
            .iter()
            .map(|identity| &identity.path)
            .ne(preview.report_roots.iter())
        {
            return Err(SecurityMapperAdapterError::InvalidPreview(
                "report roots must be sorted and unique".into(),
            ));
        }
        let input_identities = arguments
            .iter()
            .map(PathBuf::from)
            .map(|path| {
                if !preview.report_roots.contains(&path) {
                    return Err(SecurityMapperAdapterError::UnsafeInput(path));
                }
                FileIdentity::input(&path)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let unique_inputs = input_identities
            .iter()
            .map(|identity| &identity.path)
            .collect::<std::collections::BTreeSet<_>>();
        if unique_inputs.len() != input_identities.len() {
            return Err(SecurityMapperAdapterError::InvalidPreview(
                "package-mapping inputs must be unique".into(),
            ));
        }
        let expected_indexed = indexed_arguments(executable, arguments);
        if preview.indexed_arguments != expected_indexed {
            return Err(SecurityMapperAdapterError::PreviewMismatch);
        }
        let current_directory = input_identities[0]
            .directory
            .then(|| input_identities[0].path.clone())
            .or_else(|| input_identities[0].path.parent().map(Path::to_path_buf))
            .ok_or_else(|| {
                SecurityMapperAdapterError::UnsafeInput(input_identities[0].path.clone())
            })?;
        Ok(Self {
            id: preview.id,
            preview: preview.clone(),
            executable: executable.clone(),
            arguments: arguments.iter().map(OsString::from).collect(),
            current_directory,
            executable_identity,
            input_identities,
        })
    }

    pub fn id(&self) -> SecuritySessionId {
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

    fn revalidate(&self) -> Result<(), SecurityMapperAdapterError> {
        self.executable_identity.revalidate_executable()?;
        for identity in &self.input_identities {
            identity.revalidate_input()?;
        }
        let reconstructed = Self::from_preview(&self.preview)?;
        if reconstructed.id != self.id
            || reconstructed.executable != self.executable
            || reconstructed.arguments != self.arguments
            || reconstructed.current_directory != self.current_directory
            || reconstructed.executable_identity != self.executable_identity
            || reconstructed.input_identities != self.input_identities
        {
            return Err(SecurityMapperAdapterError::PreviewMismatch);
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
pub enum SecurityMapperRunnerEvent {
    Started {
        id: SecuritySessionId,
    },
    Output {
        id: SecuritySessionId,
        stream: SecurityOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        id: SecuritySessionId,
        exit_code: Option<i32>,
    },
    Failed {
        id: SecuritySessionId,
        exit_code: Option<i32>,
    },
    CancellationRequested {
        id: SecuritySessionId,
    },
    Cancelled {
        id: SecuritySessionId,
        forced: bool,
        exit_code: Option<i32>,
    },
    CancellationRejected {
        id: SecuritySessionId,
        message: String,
    },
    TimedOut {
        id: SecuritySessionId,
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        id: SecuritySessionId,
        message: String,
    },
}

#[derive(Debug)]
enum SecurityMapperPipeEvent {
    Output {
        stream: SecurityOutputStream,
        line: String,
        truncated: bool,
    },
    Failed {
        stream: SecurityOutputStream,
        message: String,
    },
}

async fn read_mapper_output<R>(
    stream: R,
    kind: SecurityOutputStream,
    sender: tokio::sync::mpsc::Sender<SecurityMapperPipeEvent>,
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
                    .send(SecurityMapperPipeEvent::Failed {
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
                    .send(SecurityMapperPipeEvent::Output {
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
            let remaining = MAX_SECURITY_MAPPER_LINE_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(SecurityMapperPipeEvent::Output {
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

pub struct SecurityMapperJobRunner {
    id: Option<SecuritySessionId>,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<SecurityMapperPipeEvent>>,
    streams_drained: bool,
    started_pending: bool,
    terminal_pending: VecDeque<SecurityMapperRunnerEvent>,
    cancellation_timeout: Duration,
    operation_timeout: Duration,
    deadline: Option<Instant>,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl Default for SecurityMapperJobRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityMapperJobRunner {
    pub fn new() -> Self {
        Self {
            id: None,
            child: None,
            output: None,
            streams_drained: true,
            started_pending: false,
            terminal_pending: VecDeque::new(),
            cancellation_timeout: Duration::from_secs(5),
            operation_timeout: SECURITY_MAPPER_OPERATION_TIMEOUT,
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
        command: SecurityMapperCommandSpec,
    ) -> Result<(), SecurityMapperAdapterError> {
        if self.child.is_some()
            || self.started_pending
            || !self.terminal_pending.is_empty()
            || self.output.is_some()
        {
            return Err(SecurityMapperAdapterError::Busy);
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
        let mut child = process
            .spawn()
            .map_err(|error| SecurityMapperAdapterError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            self.clear_process_state();
            return Err(SecurityMapperAdapterError::StreamUnavailable(
                SecurityOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            self.clear_process_state();
            return Err(SecurityMapperAdapterError::StreamUnavailable(
                SecurityOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(SECURITY_MAPPER_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_mapper_output(
            stdout,
            SecurityOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_mapper_output(
            stderr,
            SecurityOutputStream::Stderr,
            sender.clone(),
        ));
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

    pub async fn next_event(
        &mut self,
    ) -> Result<SecurityMapperRunnerEvent, SecurityMapperAdapterError> {
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        let id = self.id.ok_or(SecurityMapperAdapterError::NotRunning)?;
        if self.started_pending {
            self.started_pending = false;
            return Ok(SecurityMapperRunnerEvent::Started { id });
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            self.kill_and_clear().await;
            return Ok(SecurityMapperRunnerEvent::Lost {
                id,
                message: "Security package-mapping output event channel was lost".into(),
            });
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return self.timeout_active(id).await;
        }
        if let Some(receiver) = self.output.as_mut() {
            let deadline = self
                .deadline
                .ok_or(SecurityMapperAdapterError::NotRunning)?;
            let event = tokio::select! {
                event = receiver.recv() => Some(event),
                _ = tokio::time::sleep_until(deadline) => None,
            };
            match event {
                Some(Some(SecurityMapperPipeEvent::Output {
                    stream,
                    line,
                    truncated,
                })) => {
                    return Ok(SecurityMapperRunnerEvent::Output {
                        id,
                        stream,
                        line,
                        truncated,
                    });
                }
                Some(Some(SecurityMapperPipeEvent::Failed { stream, message })) => {
                    self.kill_and_clear().await;
                    return Ok(SecurityMapperRunnerEvent::Lost {
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
        let deadline = self
            .deadline
            .ok_or(SecurityMapperAdapterError::NotRunning)?;
        let status = {
            let child = self
                .child
                .as_mut()
                .ok_or(SecurityMapperAdapterError::NotRunning)?;
            match tokio::time::timeout_at(deadline, child.wait()).await {
                Ok(Ok(status)) => status,
                Ok(Err(error)) => {
                    self.kill_and_clear().await;
                    return Ok(SecurityMapperRunnerEvent::Lost {
                        id,
                        message: format!("Security package-mapping wait failed: {error}"),
                    });
                }
                Err(_) => return self.timeout_active(id).await,
            }
        };
        self.clear_process_state();
        if status.success() {
            Ok(SecurityMapperRunnerEvent::Completed {
                id,
                exit_code: status.code(),
            })
        } else {
            Ok(SecurityMapperRunnerEvent::Failed {
                id,
                exit_code: status.code(),
            })
        }
    }

    pub async fn cancel(
        &mut self,
        requested_id: SecuritySessionId,
    ) -> Result<bool, SecurityMapperAdapterError> {
        if self.cancellation_requested || self.child.is_none() || self.id != Some(requested_id) {
            self.terminal_pending
                .push_back(SecurityMapperRunnerEvent::CancellationRejected {
                    id: requested_id,
                    message: "no matching cancellable Security package-mapping process is active"
                        .into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        self.terminal_pending
            .push_back(SecurityMapperRunnerEvent::CancellationRequested { id: requested_id });
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state_preserving_events();
        self.terminal_pending
            .push_back(SecurityMapperRunnerEvent::Cancelled {
                id: requested_id,
                forced,
                exit_code: status.and_then(|status| status.code()),
            });
        Ok(true)
    }

    async fn timeout_active(
        &mut self,
        id: SecuritySessionId,
    ) -> Result<SecurityMapperRunnerEvent, SecurityMapperAdapterError> {
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state();
        Ok(SecurityMapperRunnerEvent::TimedOut {
            id,
            forced,
            exit_code: status.and_then(|status| status.code()),
        })
    }

    async fn terminate_active(
        &mut self,
    ) -> Result<(Option<std::process::ExitStatus>, bool), SecurityMapperAdapterError> {
        let Some(child) = self.child.as_mut() else {
            return Ok((None, false));
        };
        let mut forced = false;
        #[cfg(unix)]
        let status =
            if let Some(process_group) = self.process_group {
                // SAFETY: the negative PID targets only the process group created for this child.
                if unsafe { libc::kill(-process_group, libc::SIGTERM) } != 0 {
                    child.start_kill().map_err(|error| {
                        SecurityMapperAdapterError::ProcessControl(error.to_string())
                    })?;
                    forced = true;
                }
                match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                    Ok(result) => Some(result.map_err(|error| {
                        SecurityMapperAdapterError::ProcessControl(error.to_string())
                    })?),
                    Err(_) => {
                        // SAFETY: same child-owned process group as the graceful signal.
                        let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                        forced = true;
                        Some(child.wait().await.map_err(|error| {
                            SecurityMapperAdapterError::ProcessControl(error.to_string())
                        })?)
                    }
                }
            } else {
                forced = true;
                child.kill().await.map_err(|error| {
                    SecurityMapperAdapterError::ProcessControl(error.to_string())
                })?;
                Some(child.wait().await.map_err(|error| {
                    SecurityMapperAdapterError::ProcessControl(error.to_string())
                })?)
            };
        #[cfg(not(unix))]
        let status =
            {
                forced = true;
                child.kill().await.map_err(|error| {
                    SecurityMapperAdapterError::ProcessControl(error.to_string())
                })?;
                Some(child.wait().await.map_err(|error| {
                    SecurityMapperAdapterError::ProcessControl(error.to_string())
                })?)
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

impl Drop for SecurityMapperJobRunner {
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
    use std::sync::atomic::{AtomicU64, Ordering};
    use yoctui_model::SecurityScope;

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-security-mapper-{name}-{}-{}",
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

    #[cfg(unix)]
    fn write_executable(path: &Path, body: &str) {
        let temporary = path.with_extension(format!(
            "fixture-write-{}",
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&temporary, body).unwrap();
        let mut permissions = fs::metadata(&temporary).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&temporary, permissions).unwrap();
        fs::rename(temporary, path).unwrap();
    }

    fn preview(directory: &TestDirectory) -> SecurityOperationPreview {
        let executable = directory.path().join("cve-check-map-pkgs");
        let reports = directory.path().join("reports");
        fs::create_dir_all(&reports).unwrap();
        #[cfg(unix)]
        write_executable(&executable, "#!/bin/sh\nexit 0\n");
        let arguments = vec![reports.display().to_string()];
        SecurityOperationPreview {
            id: SecuritySessionId(7),
            scope: SecurityScope::Image {
                target: "core-image-minimal".into(),
                machine: "qemux86-64".into(),
                distro: "poky".into(),
            },
            operation: SecurityOperation::PackageMap {
                executable: executable.clone(),
                arguments: arguments.clone(),
            },
            indexed_arguments: indexed_arguments(&executable, &arguments),
            report_roots: vec![reports],
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn security_mapper_reconstructs_exact_argv_and_streams_bounded_output() {
        let directory = TestDirectory::new("output");
        let preview = preview(&directory);
        let SecurityOperation::PackageMap { executable, .. } = &preview.operation else {
            unreachable!();
        };
        write_executable(
            executable,
            &format!(
                "#!/bin/sh\nprintf 'arg=%s\\n' \"$1\"\nprintf 'stderr\\n' >&2\nprintf '{}\\n'\nprintf '\\377\\n'\n",
                "x".repeat(MAX_SECURITY_MAPPER_LINE_BYTES + 8)
            ),
        );
        let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
        assert_eq!(command.arguments().len(), 1);
        let mut runner = SecurityMapperJobRunner::new();
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::Started {
                id: SecuritySessionId(7)
            }
        );
        let mut saw_argument = false;
        let mut saw_stderr = false;
        let mut saw_truncated = false;
        let mut saw_invalid_utf8 = false;
        loop {
            match runner.next_event().await.unwrap() {
                SecurityMapperRunnerEvent::Output {
                    id,
                    stream,
                    line,
                    truncated,
                } => {
                    assert_eq!(id, SecuritySessionId(7));
                    saw_argument |= line == format!("arg={}", preview.report_roots[0].display());
                    saw_stderr |= stream == SecurityOutputStream::Stderr;
                    saw_truncated |= truncated;
                    saw_invalid_utf8 |= line.contains('\u{fffd}');
                }
                SecurityMapperRunnerEvent::Completed { id, exit_code } => {
                    assert_eq!(id, SecuritySessionId(7));
                    assert_eq!(exit_code, Some(0));
                    break;
                }
                event => panic!("unexpected event: {event:?}"),
            }
        }
        assert!(saw_argument && saw_stderr && saw_truncated && saw_invalid_utf8);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn security_mapper_rejects_tampering_symlinks_and_stale_identity() {
        let directory = TestDirectory::new("validation");
        let mut tampered = preview(&directory);
        tampered.indexed_arguments.push("2: injected".into());
        assert_eq!(
            SecurityMapperCommandSpec::from_preview(&tampered),
            Err(SecurityMapperAdapterError::PreviewMismatch)
        );

        let safe = preview(&directory);
        let SecurityOperation::PackageMap {
            executable: tool, ..
        } = &safe.operation
        else {
            unreachable!();
        };
        let linked = directory.path().join("linked");
        symlink(tool, &linked).unwrap();
        let mut linked_preview = safe.clone();
        let SecurityOperation::PackageMap { executable, .. } = &mut linked_preview.operation else {
            unreachable!();
        };
        *executable = linked.clone();
        linked_preview.indexed_arguments[0] = format!("0: {}", linked.display());
        assert!(matches!(
            SecurityMapperCommandSpec::from_preview(&linked_preview),
            Err(SecurityMapperAdapterError::UnsafeExecutable(_))
        ));

        let linked_reports = directory.path().join("linked-reports");
        symlink(&safe.report_roots[0], &linked_reports).unwrap();
        let mut linked_input = safe.clone();
        linked_input.report_roots = vec![linked_reports.clone()];
        let SecurityOperation::PackageMap { arguments, .. } = &mut linked_input.operation else {
            unreachable!();
        };
        *arguments = vec![linked_reports.display().to_string()];
        let SecurityOperation::PackageMap {
            executable,
            arguments,
        } = &linked_input.operation
        else {
            unreachable!();
        };
        linked_input.indexed_arguments = indexed_arguments(executable, arguments);
        assert!(matches!(
            SecurityMapperCommandSpec::from_preview(&linked_input),
            Err(SecurityMapperAdapterError::UnsafeInput(_))
        ));

        let command = SecurityMapperCommandSpec::from_preview(&safe).unwrap();
        write_executable(tool, "#!/bin/sh\nprintf 'changed\\n'\n");
        assert!(matches!(
            SecurityMapperJobRunner::new().start(command).await,
            Err(SecurityMapperAdapterError::StaleIdentity(_))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn security_mapper_reports_duplicate_nonzero_and_worker_loss() {
        let directory = TestDirectory::new("outcomes");
        let preview = preview(&directory);
        let SecurityOperation::PackageMap { executable, .. } = &preview.operation else {
            unreachable!();
        };
        write_executable(executable, "#!/bin/sh\nexit 9\n");
        let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
        let mut runner = SecurityMapperJobRunner::new();
        runner.start(command.clone()).await.unwrap();
        assert_eq!(
            runner.start(command.clone()).await,
            Err(SecurityMapperAdapterError::Busy)
        );
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::Started {
                id: SecuritySessionId(7)
            }
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::Failed {
                id: SecuritySessionId(7),
                exit_code: Some(9)
            }
        ));

        write_executable(executable, "#!/bin/sh\nsleep 2\n");
        let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
        runner.start(command).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        runner.lose_output_channel();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::Lost {
                id: SecuritySessionId(7),
                ..
            }
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn security_mapper_cancels_gracefully_forcibly_and_times_out() {
        let directory = TestDirectory::new("control");
        let preview = preview(&directory);
        let SecurityOperation::PackageMap { executable, .. } = &preview.operation else {
            unreachable!();
        };
        write_executable(
            executable,
            "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n",
        );
        let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
        let mut runner =
            SecurityMapperJobRunner::new().with_cancellation_timeout(Duration::from_secs(1));
        runner.start(command.clone()).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        assert!(!runner.cancel(SecuritySessionId(8)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::CancellationRejected {
                id: SecuritySessionId(8),
                ..
            }
        ));
        assert!(runner.cancel(SecuritySessionId(7)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::CancellationRequested {
                id: SecuritySessionId(7)
            }
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::Cancelled {
                id: SecuritySessionId(7),
                forced: false,
                ..
            }
        ));

        write_executable(
            executable,
            "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
        );
        let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
        let mut forced =
            SecurityMapperJobRunner::new().with_cancellation_timeout(Duration::from_millis(20));
        forced.start(command.clone()).await.unwrap();
        let _ = forced.next_event().await.unwrap();
        let _ = forced.next_event().await.unwrap();
        assert!(forced.cancel(SecuritySessionId(7)).await.unwrap());
        let _ = forced.next_event().await.unwrap();
        assert!(matches!(
            forced.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::Cancelled { forced: true, .. }
        ));

        let mut timed_out = SecurityMapperJobRunner::new()
            .with_cancellation_timeout(Duration::from_millis(20))
            .with_operation_timeout(Duration::from_millis(20));
        timed_out.start(command).await.unwrap();
        let _ = timed_out.next_event().await.unwrap();
        let _ = timed_out.next_event().await.unwrap();
        assert!(matches!(
            timed_out.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::TimedOut {
                id: SecuritySessionId(7),
                forced: true,
                ..
            }
        ));
    }
}
