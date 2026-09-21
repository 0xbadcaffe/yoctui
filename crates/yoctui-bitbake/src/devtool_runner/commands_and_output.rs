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
