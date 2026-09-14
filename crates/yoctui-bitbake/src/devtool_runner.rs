//! Devtool runner.
use super::*;

#[derive(Debug, Clone)]
pub struct DevtoolInspector {
    pub(crate) devtool_program: Option<PathBuf>,
    pub(crate) git_program: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolCommandSpec {
    pub(crate) executable: PathBuf,
    pub(crate) arguments: Vec<OsString>,
    pub(crate) capability_generation: u64,
    pub(crate) capability: yoctui_model::CapabilityId,
    pub(crate) build_directory: PathBuf,
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

pub(crate) const MAX_DEVTOOL_LINE_BYTES: usize = 64 * 1024;

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
pub(crate) enum DevtoolPipeEvent {
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

pub(crate) async fn read_devtool_output<R>(
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
    pub(crate) build_dir: PathBuf,
    pub(crate) child: Option<Child>,
    pub(crate) output: Option<tokio::sync::mpsc::Receiver<DevtoolPipeEvent>>,
    pub(crate) streams_drained: bool,
    pub(crate) started_pending: bool,
    pub(crate) terminal_pending: Option<DevtoolRunnerEvent>,
    pub(crate) cancellation_timeout: Duration,
    pub(crate) cancellation_requested: bool,
    #[cfg(unix)]
    pub(crate) process_group: Option<i32>,
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

pub(crate) fn parse_devtool_status(
    output: &str,
) -> Result<Vec<(String, PathBuf, Option<PathBuf>)>, String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !["NOTE: ", "WARNING: ", "DEBUG: "]
                    .iter()
                    .any(|prefix| line.starts_with(prefix))
        })
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

pub(crate) async fn inspect_git(program: &Path, source_path: &Path) -> DevtoolGitState {
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

pub(crate) fn parse_git_status(output: &str) -> Result<DevtoolGitState, String> {
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
