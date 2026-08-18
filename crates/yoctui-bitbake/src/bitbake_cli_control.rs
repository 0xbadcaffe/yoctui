use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::{Child, Command},
    task::JoinHandle,
};
use yoctui_model::DaemonCompatibilitySnapshot;

use crate::{BitBakeCommandPlanner, BitBakeServerCommandOperation};

const DEFAULT_OUTPUT_LIMIT: usize = 64 * 1024;
const MAX_OUTPUT_LIMIT: usize = 1024 * 1024;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_CANCEL_GRACE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitBakeCliOperation {
    Status,
    StartServer,
    StopServer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeCliPreview {
    pub operation: BitBakeCliOperation,
    pub argv: Vec<OsString>,
    pub cwd: PathBuf,
    pub timeout: Duration,
    pub output_limit_per_stream: usize,
}

#[derive(Debug, Clone)]
pub struct BitBakeCliCommand {
    preview: BitBakeCliPreview,
    environment: BTreeMap<String, String>,
}

impl BitBakeCliCommand {
    pub fn new(
        executable: PathBuf,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
        compatibility: &DaemonCompatibilitySnapshot,
        expected_generation: u64,
        operation: BitBakeCliOperation,
    ) -> Result<Self, BitBakeCliControlError> {
        Self::with_limits(
            executable,
            build_dir,
            environment,
            compatibility,
            expected_generation,
            operation,
            DEFAULT_TIMEOUT,
            DEFAULT_OUTPUT_LIMIT,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_limits(
        executable: PathBuf,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
        compatibility: &DaemonCompatibilitySnapshot,
        expected_generation: u64,
        operation: BitBakeCliOperation,
        timeout: Duration,
        output_limit_per_stream: usize,
    ) -> Result<Self, BitBakeCliControlError> {
        validate_executable(&executable)?;
        if !build_dir.is_absolute() || !build_dir.is_dir() {
            return Err(BitBakeCliControlError::InvalidBuildDirectory(build_dir));
        }
        let authorized =
            BitBakeCommandPlanner::new(compatibility, expected_generation, &build_dir)?
                .server_control(match operation {
                    BitBakeCliOperation::Status => BitBakeServerCommandOperation::Status,
                    BitBakeCliOperation::StartServer => BitBakeServerCommandOperation::Start,
                    BitBakeCliOperation::StopServer => BitBakeServerCommandOperation::Stop,
                })?;
        if timeout.is_zero() {
            return Err(BitBakeCliControlError::InvalidLimit(
                "timeout must be greater than zero".into(),
            ));
        }
        if output_limit_per_stream == 0 || output_limit_per_stream > MAX_OUTPUT_LIMIT {
            return Err(BitBakeCliControlError::InvalidLimit(format!(
                "output limit must be between 1 and {MAX_OUTPUT_LIMIT} bytes per stream"
            )));
        }
        validate_environment(&environment)?;
        Ok(Self {
            preview: BitBakeCliPreview {
                operation,
                argv: std::iter::once(executable.into_os_string())
                    .chain(authorized.arguments)
                    .collect(),
                cwd: build_dir,
                timeout,
                output_limit_per_stream,
            },
            environment,
        })
    }

    pub fn preview(&self) -> &BitBakeCliPreview {
        &self.preview
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitBakeCliOutcome {
    Succeeded {
        exit_code: i32,
        stdout: String,
        stderr: String,
        truncated: bool,
    },
    NonZero {
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
        truncated: bool,
    },
    TimedOut {
        forced: bool,
        stdout: String,
        stderr: String,
        truncated: bool,
    },
    Cancelled {
        forced: bool,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
        truncated: bool,
    },
}

#[derive(Debug, Error)]
pub enum BitBakeCliControlError {
    #[error("invalid BitBake executable: {0}")]
    InvalidExecutable(PathBuf),
    #[error("invalid BitBake build directory: {0}")]
    InvalidBuildDirectory(PathBuf),
    #[error(transparent)]
    Authorization(#[from] crate::BitBakeCommandAuthorizationError),
    #[error("invalid BitBake CLI limit: {0}")]
    InvalidLimit(String),
    #[error("invalid captured environment: {0}")]
    InvalidEnvironment(String),
    #[error("a BitBake CLI control process is already active")]
    Busy,
    #[error("no BitBake CLI control process is active")]
    NotRunning,
    #[error("could not start BitBake CLI control: {0}")]
    Spawn(String),
    #[error("BitBake CLI process control failed: {0}")]
    ProcessControl(String),
    #[error("BitBake CLI output collection failed: {0}")]
    Output(String),
}

pub struct BitBakeCliRunner {
    child: Option<Child>,
    stdout: Option<JoinHandle<std::io::Result<BoundedOutput>>>,
    stderr: Option<JoinHandle<std::io::Result<BoundedOutput>>>,
    timeout: Duration,
    cancellation_grace: Duration,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl Default for BitBakeCliRunner {
    fn default() -> Self {
        Self {
            child: None,
            stdout: None,
            stderr: None,
            timeout: DEFAULT_TIMEOUT,
            cancellation_grace: DEFAULT_CANCEL_GRACE,
            #[cfg(unix)]
            process_group: None,
        }
    }
}

impl BitBakeCliRunner {
    pub fn with_cancellation_grace(mut self, grace: Duration) -> Self {
        self.cancellation_grace = grace;
        self
    }

    pub fn is_active(&self) -> bool {
        self.child.is_some()
    }

    pub async fn start(
        &mut self,
        command: BitBakeCliCommand,
    ) -> Result<(), BitBakeCliControlError> {
        if self.is_active() || self.stdout.is_some() || self.stderr.is_some() {
            return Err(BitBakeCliControlError::Busy);
        }
        let preview = &command.preview;
        let mut process = Command::new(&preview.argv[0]);
        process
            .args(&preview.argv[1..])
            .current_dir(&preview.cwd)
            .env_clear()
            .envs(command.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        process.process_group(0);
        let mut child = process
            .spawn()
            .map_err(|error| BitBakeCliControlError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BitBakeCliControlError::Spawn("stdout pipe was not created".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| BitBakeCliControlError::Spawn("stderr pipe was not created".into()))?;
        let limit = preview.output_limit_per_stream;
        self.stdout = Some(tokio::spawn(read_bounded(stdout, limit)));
        self.stderr = Some(tokio::spawn(read_bounded(stderr, limit)));
        self.timeout = preview.timeout;
        self.child = Some(child);
        Ok(())
    }

    pub async fn complete(&mut self) -> Result<BitBakeCliOutcome, BitBakeCliControlError> {
        let child = self
            .child
            .as_mut()
            .ok_or(BitBakeCliControlError::NotRunning)?;
        match tokio::time::timeout(self.timeout, child.wait()).await {
            Ok(status) => {
                let status = status
                    .map_err(|error| BitBakeCliControlError::ProcessControl(error.to_string()))?;
                self.child = None;
                self.clear_process_group();
                let output = self.collect_output().await?;
                Ok(outcome_for_status(status, output))
            }
            Err(_) => {
                let (status, forced) = self.terminate().await?;
                let output = self.collect_output().await?;
                let _ = status;
                Ok(BitBakeCliOutcome::TimedOut {
                    forced,
                    stdout: output.stdout,
                    stderr: output.stderr,
                    truncated: output.truncated,
                })
            }
        }
    }

    pub async fn cancel(&mut self) -> Result<BitBakeCliOutcome, BitBakeCliControlError> {
        if self.child.is_none() {
            return Err(BitBakeCliControlError::NotRunning);
        }
        let (status, forced) = self.terminate().await?;
        let output = self.collect_output().await?;
        Ok(BitBakeCliOutcome::Cancelled {
            forced,
            exit_code: status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
            truncated: output.truncated,
        })
    }

    async fn terminate(&mut self) -> Result<(ExitStatus, bool), BitBakeCliControlError> {
        let child = self
            .child
            .as_mut()
            .ok_or(BitBakeCliControlError::NotRunning)?;
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(group) = self.process_group {
            // SAFETY: the negative PID targets only the process group created for this child.
            if unsafe { libc::kill(-group, libc::SIGTERM) } != 0 {
                child
                    .start_kill()
                    .map_err(|error| BitBakeCliControlError::ProcessControl(error.to_string()))?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_grace, child.wait()).await {
                Ok(result) => result
                    .map_err(|error| BitBakeCliControlError::ProcessControl(error.to_string()))?,
                Err(_) => {
                    // SAFETY: this is the same child-owned process group.
                    let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
                    forced = true;
                    child.wait().await.map_err(|error| {
                        BitBakeCliControlError::ProcessControl(error.to_string())
                    })?
                }
            }
        } else {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| BitBakeCliControlError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| BitBakeCliControlError::ProcessControl(error.to_string()))?
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| BitBakeCliControlError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| BitBakeCliControlError::ProcessControl(error.to_string()))?
        };
        self.child = None;
        self.clear_process_group();
        Ok((status, forced))
    }

    async fn collect_output(&mut self) -> Result<CombinedOutput, BitBakeCliControlError> {
        let stdout = join_output(self.stdout.take()).await?;
        let stderr = join_output(self.stderr.take()).await?;
        Ok(CombinedOutput {
            stdout: String::from_utf8_lossy(&stdout.bytes).into_owned(),
            stderr: String::from_utf8_lossy(&stderr.bytes).into_owned(),
            truncated: stdout.truncated || stderr.truncated,
        })
    }

    fn clear_process_group(&mut self) {
        #[cfg(unix)]
        {
            self.process_group = None;
        }
    }
}

impl Drop for BitBakeCliRunner {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(group) = self.process_group {
            // SAFETY: the negative PID targets only the process group created for this child.
            let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }
}

#[derive(Debug)]
struct BoundedOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

#[derive(Debug)]
struct CombinedOutput {
    stdout: String,
    stderr: String,
    truncated: bool,
}

async fn read_bounded(
    mut reader: impl AsyncRead + Unpin,
    limit: usize,
) -> std::io::Result<BoundedOutput> {
    let mut bytes = Vec::with_capacity(limit.min(8192));
    let mut chunk = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let count = reader.read(&mut chunk).await?;
        if count == 0 {
            break;
        }
        let retained = limit.saturating_sub(bytes.len()).min(count);
        bytes.extend_from_slice(&chunk[..retained]);
        truncated |= retained < count;
    }
    Ok(BoundedOutput { bytes, truncated })
}

async fn join_output(
    task: Option<JoinHandle<std::io::Result<BoundedOutput>>>,
) -> Result<BoundedOutput, BitBakeCliControlError> {
    task.ok_or_else(|| BitBakeCliControlError::Output("output task is unavailable".into()))?
        .await
        .map_err(|error| BitBakeCliControlError::Output(error.to_string()))?
        .map_err(|error| BitBakeCliControlError::Output(error.to_string()))
}

fn outcome_for_status(status: ExitStatus, output: CombinedOutput) -> BitBakeCliOutcome {
    if status.success() {
        BitBakeCliOutcome::Succeeded {
            exit_code: status.code().unwrap_or(0),
            stdout: output.stdout,
            stderr: output.stderr,
            truncated: output.truncated,
        }
    } else {
        BitBakeCliOutcome::NonZero {
            exit_code: status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
            truncated: output.truncated,
        }
    }
}

fn validate_executable(executable: &Path) -> Result<(), BitBakeCliControlError> {
    if executable.as_os_str().is_empty() || has_nul(executable.as_os_str()) {
        return Err(BitBakeCliControlError::InvalidExecutable(
            executable.to_path_buf(),
        ));
    }
    Ok(())
}

fn validate_environment(
    environment: &BTreeMap<String, String>,
) -> Result<(), BitBakeCliControlError> {
    for (name, value) in environment {
        if name.is_empty() || name.contains('=') || name.contains('\0') || value.contains('\0') {
            return Err(BitBakeCliControlError::InvalidEnvironment(name.clone()));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn has_nul(value: &OsStr) -> bool {
    use std::os::unix::ffi::OsStrExt;
    value.as_bytes().contains(&0)
}

#[cfg(not(unix))]
fn has_nul(value: &OsStr) -> bool {
    value.to_string_lossy().contains('\0')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityId, CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, YoctoEnvironmentIdentity,
    };

    fn operation_parts(operation: BitBakeCliOperation) -> (&'static str, &'static str) {
        match operation {
            BitBakeCliOperation::Status => (
                crate::compatibility_command::BITBAKE_SERVER_STATUS_ARGV_IMPLEMENTATION,
                "--status-only",
            ),
            BitBakeCliOperation::StartServer => (
                crate::compatibility_command::BITBAKE_SERVER_START_ARGV_IMPLEMENTATION,
                "--server-only",
            ),
            BitBakeCliOperation::StopServer => (
                crate::compatibility_command::BITBAKE_SERVER_STOP_ARGV_IMPLEMENTATION,
                "--kill-server",
            ),
        }
    }

    fn compatibility(
        build_dir: &Path,
        operation: BitBakeCliOperation,
    ) -> DaemonCompatibilitySnapshot {
        let (implementation, _) = operation_parts(operation);
        let capability = match operation {
            BitBakeCliOperation::Status => CapabilityId::BitBakeServerStatus,
            BitBakeCliOperation::StartServer => CapabilityId::BitBakeServerStart,
            BitBakeCliOperation::StopServer => CapabilityId::BitBakeServerStop,
        };
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation: 1,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        build_dir.to_owned(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities: vec![CapabilityRecord {
                    id: capability,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: "BitBake server option".into(),
                        detail: "The exact server option was observed in help output.".into(),
                        argv: vec!["bitbake".into(), "--help".into()],
                    }],
                }],
            },
            implementations: BTreeMap::from([(
                capability,
                CapabilityImplementation {
                    id: implementation.into(),
                    kind: CapabilityImplementationKind::Command,
                },
            )]),
        }
        .normalize()
        .unwrap()
    }

    fn fixture(body: &str) -> (PathBuf, PathBuf) {
        let nonce = std::time::SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "yoctui-cli-control-{}-{nonce};no-shell",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("fake bitbake;not-shell");
        fs::write(&executable, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&executable, permissions).unwrap();
        (root, executable)
    }

    fn command(
        executable: PathBuf,
        build_dir: PathBuf,
        operation: BitBakeCliOperation,
        timeout: Duration,
        output_limit: usize,
    ) -> BitBakeCliCommand {
        let compatibility = compatibility(&build_dir, operation);
        BitBakeCliCommand::with_limits(
            executable,
            build_dir,
            BTreeMap::from([("MARKER".into(), "captured".into())]),
            &compatibility,
            compatibility.snapshot.generation,
            operation,
            timeout,
            output_limit,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn cli_control_previews_and_runs_exact_shell_free_server_operations() {
        let (root, executable) = fixture("printf '%s:%s' \"$1\" \"$MARKER\"");
        for (operation, expected) in [
            (BitBakeCliOperation::Status, "--status-only:captured"),
            (BitBakeCliOperation::StartServer, "--server-only:captured"),
            (BitBakeCliOperation::StopServer, "--kill-server:captured"),
        ] {
            let command = command(
                executable.clone(),
                root.clone(),
                operation,
                Duration::from_secs(2),
                1024,
            );
            assert_eq!(
                command.preview().argv,
                vec![
                    executable.clone().into_os_string(),
                    OsString::from(operation_parts(operation).1)
                ]
            );
            assert_eq!(command.preview().cwd, root);
            let mut runner = BitBakeCliRunner::default();
            runner.start(command).await.unwrap();
            assert_eq!(
                runner.complete().await.unwrap(),
                BitBakeCliOutcome::Succeeded {
                    exit_code: 0,
                    stdout: expected.into(),
                    stderr: String::new(),
                    truncated: false,
                }
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn cli_control_is_capability_aware_and_bounds_output_and_runtime() {
        let (root, executable) = fixture("printf '123456789'; printf 'abcdefghi' >&2; exit 7");
        let status_only = compatibility(&root, BitBakeCliOperation::Status);
        let unsupported = BitBakeCliCommand::new(
            executable.clone(),
            root.clone(),
            BTreeMap::new(),
            &status_only,
            status_only.snapshot.generation,
            BitBakeCliOperation::StartServer,
        );
        assert!(matches!(
            unsupported,
            Err(BitBakeCliControlError::Authorization(
                crate::BitBakeCommandAuthorizationError::CapabilityMissing { .. }
            ))
        ));
        let mut runner = BitBakeCliRunner::default();
        runner
            .start(command(
                executable.clone(),
                root.clone(),
                BitBakeCliOperation::Status,
                Duration::from_secs(2),
                5,
            ))
            .await
            .unwrap();
        assert_eq!(
            runner.complete().await.unwrap(),
            BitBakeCliOutcome::NonZero {
                exit_code: Some(7),
                stdout: "12345".into(),
                stderr: "abcde".into(),
                truncated: true,
            }
        );

        fs::write(
            &executable,
            "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
        )
        .unwrap();
        let mut runner =
            BitBakeCliRunner::default().with_cancellation_grace(Duration::from_millis(20));
        runner
            .start(command(
                executable,
                root.clone(),
                BitBakeCliOperation::Status,
                Duration::from_millis(20),
                64,
            ))
            .await
            .unwrap();
        assert!(matches!(
            runner.complete().await.unwrap(),
            BitBakeCliOutcome::TimedOut { forced: true, .. }
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn cli_control_cancels_the_owned_process_group() {
        let (root, executable) = fixture("trap 'exit 0' TERM; : > \"$READY\"; while :; do :; done");
        let ready = root.join("ready");
        let mut command = command(
            executable,
            root.clone(),
            BitBakeCliOperation::StartServer,
            Duration::from_secs(5),
            64,
        );
        command
            .environment
            .insert("READY".into(), ready.to_string_lossy().into_owned());
        let mut runner =
            BitBakeCliRunner::default().with_cancellation_grace(Duration::from_secs(1));
        runner.start(command).await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while !ready.is_file() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("fixture did not install its TERM trap");
        let outcome = runner.cancel().await.unwrap();
        assert!(matches!(
            outcome,
            BitBakeCliOutcome::Cancelled { forced: false, .. }
        ));
        assert!(!runner.is_active());
        fs::remove_dir_all(root).unwrap();
    }
}
