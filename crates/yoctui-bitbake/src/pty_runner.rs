use std::{
    collections::BTreeMap,
    fs::File,
    io,
    os::fd::{FromRawFd, OwnedFd},
    process::Stdio,
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
    sync::mpsc,
};
use yoctui_model::{
    PtyClientId, PtyDimensions, PtyExitStatus, PtySession, PtySessionAction, PtySessionError,
    PtySessionLifecycle, PtySessionSpec,
};

const PTY_OUTPUT_CHUNK_BYTES: usize = 4096;
const PTY_OUTPUT_QUEUE_DEPTH: usize = 256;
const MAX_PTY_INPUT_BYTES: usize = 64 * 1024;
const DEFAULT_TERMINATION_GRACE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyRunnerEvent {
    Started,
    Output { sequence: u64, bytes: Vec<u8> },
    Exited(PtyExitStatus),
    Lost { message: String },
}

#[derive(Debug, Error)]
pub enum PtyRunnerError {
    #[error(transparent)]
    Session(#[from] PtySessionError),
    #[error("PTY runner is already active")]
    Busy,
    #[error("PTY runner is not active")]
    NotRunning,
    #[error("invalid PTY environment: {0}")]
    InvalidEnvironment(String),
    #[error("could not create PTY: {0}")]
    Open(String),
    #[error("could not start PTY child: {0}")]
    Spawn(String),
    #[error("PTY input exceeds {MAX_PTY_INPUT_BYTES} bytes")]
    InputTooLarge,
    #[error("PTY I/O failed: {0}")]
    Io(String),
    #[error("PTY process control failed: {0}")]
    ProcessControl(String),
}

enum ReaderEvent {
    Output(Vec<u8>),
    Failed(String),
}

pub struct PtyRunner {
    session: Option<PtySession>,
    child: Option<Child>,
    writer: Option<tokio::fs::File>,
    output: Option<mpsc::Receiver<ReaderEvent>>,
    start_pending: bool,
    next_sequence: u64,
    termination_grace: Duration,
}

impl Default for PtyRunner {
    fn default() -> Self {
        Self {
            session: None,
            child: None,
            writer: None,
            output: None,
            start_pending: false,
            next_sequence: 0,
            termination_grace: DEFAULT_TERMINATION_GRACE,
        }
    }
}

impl PtyRunner {
    pub fn with_termination_grace(mut self, grace: Duration) -> Self {
        self.termination_grace = grace;
        self
    }

    pub fn session(&self) -> Option<&PtySession> {
        self.session.as_ref()
    }

    pub fn is_active(&self) -> bool {
        self.child.is_some()
    }

    pub async fn start(
        &mut self,
        spec: PtySessionSpec,
        environment: BTreeMap<String, String>,
    ) -> Result<(), PtyRunnerError> {
        if self.child.is_some() || self.output.is_some() || self.start_pending {
            return Err(PtyRunnerError::Busy);
        }
        spec.validate()?;
        validate_environment(&environment)?;
        let (master, slave) = open_pty(spec.dimensions)?;
        let master_file = File::from(master);
        let reader_file = master_file
            .try_clone()
            .map_err(|error| PtyRunnerError::Open(error.to_string()))?;
        let stdin = File::from(slave);
        let stdout = stdin
            .try_clone()
            .map_err(|error| PtyRunnerError::Open(error.to_string()))?;
        let stderr = stdin
            .try_clone()
            .map_err(|error| PtyRunnerError::Open(error.to_string()))?;
        let mut command = Command::new(&spec.command.executable);
        command
            .args(&spec.command.arguments)
            .current_dir(&spec.cwd)
            .env_clear()
            .envs(environment)
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .kill_on_drop(true);
        // SAFETY: this closure runs in the child after fork and uses only
        // async-signal-safe libc calls. Stdio has already installed the PTY
        // slave as fd 0; setsid creates the daemon-owned child session.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(io::Error::last_os_error());
                }
                if libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command
            .spawn()
            .map_err(|error| PtyRunnerError::Spawn(error.to_string()))?;
        let process_group = child
            .id()
            .ok_or_else(|| PtyRunnerError::Spawn("child PID is unavailable".into()))?
            as i32;
        let mut session = PtySession::new(spec, process_group)?;
        session.apply(PtySessionAction::MarkRunning)?;
        let (sender, receiver) = mpsc::channel(PTY_OUTPUT_QUEUE_DEPTH);
        tokio::spawn(read_master(tokio::fs::File::from_std(reader_file), sender));
        self.writer = Some(tokio::fs::File::from_std(master_file));
        self.output = Some(receiver);
        self.child = Some(child);
        self.session = Some(session);
        self.start_pending = true;
        self.next_sequence = 0;
        Ok(())
    }

    pub fn apply_session_action(&mut self, action: PtySessionAction) -> Result<(), PtyRunnerError> {
        self.session
            .as_mut()
            .ok_or(PtyRunnerError::NotRunning)?
            .apply(action)?;
        Ok(())
    }

    pub async fn input(
        &mut self,
        client: PtyClientId,
        writer_epoch: u64,
        bytes: &[u8],
    ) -> Result<(), PtyRunnerError> {
        if bytes.len() > MAX_PTY_INPUT_BYTES {
            return Err(PtyRunnerError::InputTooLarge);
        }
        let session = self.session.as_ref().ok_or(PtyRunnerError::NotRunning)?;
        if session.lifecycle != PtySessionLifecycle::Running
            || session.writer.map(|writer| (writer.client, writer.epoch))
                != Some((client, writer_epoch))
        {
            return Err(PtySessionError::NotWriter.into());
        }
        self.writer
            .as_mut()
            .ok_or(PtyRunnerError::NotRunning)?
            .write_all(bytes)
            .await
            .map_err(|error| PtyRunnerError::Io(error.to_string()))
    }

    pub fn resize(
        &mut self,
        client: PtyClientId,
        writer_epoch: u64,
        dimensions: PtyDimensions,
    ) -> Result<(), PtyRunnerError> {
        let mut next = self
            .session
            .as_ref()
            .ok_or(PtyRunnerError::NotRunning)?
            .clone();
        next.apply(PtySessionAction::Resize {
            client,
            writer_epoch,
            dimensions,
        })?;
        let fd = self
            .writer
            .as_ref()
            .ok_or(PtyRunnerError::NotRunning)?
            .as_raw_fd();
        set_window_size(fd, dimensions)?;
        self.session = Some(next);
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<PtyRunnerEvent, PtyRunnerError> {
        if self.start_pending {
            self.start_pending = false;
            return Ok(PtyRunnerEvent::Started);
        }
        if let Some(receiver) = self.output.as_mut() {
            match receiver.recv().await {
                Some(ReaderEvent::Output(bytes)) => {
                    let sequence = self.next_sequence;
                    self.next_sequence = self.next_sequence.checked_add(1).ok_or_else(|| {
                        PtyRunnerError::Io("PTY output sequence exhausted".into())
                    })?;
                    return Ok(PtyRunnerEvent::Output { sequence, bytes });
                }
                Some(ReaderEvent::Failed(message)) => {
                    self.force_kill().await;
                    self.mark_lost();
                    return Ok(PtyRunnerEvent::Lost { message });
                }
                None => self.output = None,
            }
        }
        let child = self.child.as_mut().ok_or(PtyRunnerError::NotRunning)?;
        let status = child
            .wait()
            .await
            .map_err(|error| PtyRunnerError::ProcessControl(error.to_string()))?;
        let exit = exit_status(status);
        self.child = None;
        self.writer = None;
        if let Some(session) = self.session.as_mut() {
            session.apply(PtySessionAction::Exit(exit))?;
        }
        Ok(PtyRunnerEvent::Exited(exit))
    }

    pub async fn terminate(&mut self) -> Result<bool, PtyRunnerError> {
        let group = self
            .session
            .as_ref()
            .and_then(|session| session.process_group)
            .ok_or(PtyRunnerError::NotRunning)?;
        if self
            .session
            .as_ref()
            .is_some_and(|session| session.lifecycle == PtySessionLifecycle::Running)
        {
            self.session
                .as_mut()
                .expect("checked above")
                .apply(PtySessionAction::BeginTermination)?;
        }
        let child_pid = self
            .child
            .as_ref()
            .and_then(Child::id)
            .ok_or(PtyRunnerError::NotRunning)? as libc::pid_t;
        // SAFETY: child_pid is the owned child and group is its positive session
        // leader PID. Signaling both makes the owner's graceful trap reliable
        // while retaining process-group cleanup for descendants.
        if unsafe { libc::kill(child_pid, libc::SIGTERM) } != 0
            || unsafe { libc::kill(-group, libc::SIGTERM) } != 0
        {
            return Err(PtyRunnerError::ProcessControl(
                io::Error::last_os_error().to_string(),
            ));
        }
        let child = self.child.as_mut().ok_or(PtyRunnerError::NotRunning)?;
        let (status, forced) =
            match tokio::time::timeout(self.termination_grace, child.wait()).await {
                Ok(result) => {
                    let status = result
                        .map_err(|error| PtyRunnerError::ProcessControl(error.to_string()))?;
                    (status, false)
                }
                Err(_) => {
                    // SAFETY: this is the same child-owned process group.
                    let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
                    let status = child
                        .wait()
                        .await
                        .map_err(|error| PtyRunnerError::ProcessControl(error.to_string()))?;
                    (status, true)
                }
            };
        self.child = None;
        self.writer = None;
        self.output = None;
        let exit = exit_status(status);
        if let Some(session) = self.session.as_mut() {
            session.apply(PtySessionAction::Exit(exit))?;
        }
        Ok(forced)
    }

    async fn force_kill(&mut self) {
        if let Some(group) = self
            .session
            .as_ref()
            .and_then(|session| session.process_group)
        {
            // SAFETY: this is the child-owned process group.
            let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.wait().await;
        }
        self.child = None;
        self.writer = None;
        self.output = None;
    }

    fn mark_lost(&mut self) {
        if let Some(session) = self.session.as_mut()
            && !session.lifecycle.is_terminal()
        {
            let _ = session.apply(PtySessionAction::MarkLost);
        }
    }
}

impl Drop for PtyRunner {
    fn drop(&mut self) {
        if let Some(group) = self
            .session
            .as_ref()
            .and_then(|session| session.process_group)
        {
            // SAFETY: this is the child-owned process group.
            let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }
}

use std::os::fd::AsRawFd;

fn open_pty(dimensions: PtyDimensions) -> Result<(OwnedFd, OwnedFd), PtyRunnerError> {
    dimensions.validate()?;
    let mut master = -1;
    let mut slave = -1;
    let window = libc::winsize {
        ws_row: dimensions.rows,
        ws_col: dimensions.columns,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: valid output pointers and winsize are provided; termios defaults are requested.
    if unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            &window,
        )
    } != 0
    {
        return Err(PtyRunnerError::Open(io::Error::last_os_error().to_string()));
    }
    // SAFETY: successful openpty returned uniquely owned file descriptors.
    Ok(unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) })
}

fn set_window_size(fd: i32, dimensions: PtyDimensions) -> Result<(), PtyRunnerError> {
    dimensions.validate()?;
    let window = libc::winsize {
        ws_row: dimensions.rows,
        ws_col: dimensions.columns,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: fd is the runner-owned PTY master and window points to initialized data.
    if unsafe { libc::ioctl(fd, libc::TIOCSWINSZ as _, &window) } != 0 {
        return Err(PtyRunnerError::Io(io::Error::last_os_error().to_string()));
    }
    Ok(())
}

async fn read_master(mut master: tokio::fs::File, sender: mpsc::Sender<ReaderEvent>) {
    let mut buffer = vec![0; PTY_OUTPUT_CHUNK_BYTES];
    loop {
        match master.read(&mut buffer).await {
            Ok(0) => break,
            Ok(count) => {
                if sender
                    .send(ReaderEvent::Output(buffer[..count].to_vec()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(error) if error.raw_os_error() == Some(libc::EIO) => break,
            Err(error) => {
                let _ = sender.send(ReaderEvent::Failed(error.to_string())).await;
                break;
            }
        }
    }
}

fn validate_environment(environment: &BTreeMap<String, String>) -> Result<(), PtyRunnerError> {
    for (name, value) in environment {
        if name.is_empty() || name.contains('=') || name.contains('\0') || value.contains('\0') {
            return Err(PtyRunnerError::InvalidEnvironment(name.clone()));
        }
    }
    Ok(())
}

fn exit_status(status: std::process::ExitStatus) -> PtyExitStatus {
    use std::os::unix::process::ExitStatusExt;
    status
        .code()
        .map(PtyExitStatus::Code)
        .or_else(|| status.signal().map(PtyExitStatus::Signal))
        .unwrap_or(PtyExitStatus::Signal(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    use yoctui_model::{PtyCommandIdentity, PtySessionId, PtySessionKind, PtyWorkspaceContext};

    static FIXTURE: AtomicU64 = AtomicU64::new(1);

    fn fixture(script: &str) -> (PathBuf, PtySessionSpec) {
        let id = FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("yoctui-pty-runner-{}-{id}", std::process::id()));
        fs::create_dir_all(root.join("build")).unwrap();
        let spec = PtySessionSpec {
            id: PtySessionId(id),
            name: "test shell".into(),
            kind: PtySessionKind::BuildShell,
            cwd: root.join("build"),
            command: PtyCommandIdentity {
                executable: "/bin/sh".into(),
                arguments: vec!["-c".into(), script.into()],
            },
            dimensions: PtyDimensions {
                columns: 80,
                rows: 24,
            },
            restartable: true,
            workspace: PtyWorkspaceContext {
                source_dir: root.clone(),
                build_dir: root.join("build"),
                authorized_context_roots: Vec::new(),
                owner_identity: format!("fixture-{id}"),
            },
        };
        (root, spec)
    }

    async fn collect_until_exit(runner: &mut PtyRunner) -> Vec<u8> {
        let mut output = Vec::new();
        loop {
            match runner.next_event().await.unwrap() {
                PtyRunnerEvent::Started => {}
                PtyRunnerEvent::Output { bytes, .. } => output.extend(bytes),
                PtyRunnerEvent::Exited(_) => return output,
                PtyRunnerEvent::Lost { message } => panic!("PTY lost: {message}"),
            }
        }
    }

    #[tokio::test]
    async fn pty_runner_uses_real_pty_raw_io_and_resize() {
        let (root, spec) = fixture("stty size; IFS= read -r line; printf 'got:%s\\n' \"$line\"");
        let mut runner = PtyRunner::default();
        runner
            .start(
                spec,
                BTreeMap::from([
                    ("PATH".into(), "/usr/bin:/bin".into()),
                    ("TERM".into(), "xterm-256color".into()),
                ]),
            )
            .await
            .unwrap();
        let client = PtyClientId([1; 16]);
        runner
            .apply_session_action(PtySessionAction::Attach(client))
            .unwrap();
        runner
            .apply_session_action(PtySessionAction::TakeControl {
                client,
                expected_epoch: 0,
            })
            .unwrap();
        runner
            .resize(
                client,
                1,
                PtyDimensions {
                    columns: 100,
                    rows: 30,
                },
            )
            .unwrap();
        runner.input(client, 1, b"hello\n").await.unwrap();
        let output = collect_until_exit(&mut runner).await;
        let text = String::from_utf8_lossy(&output).replace('\r', "");
        assert!(text.contains("30 100"), "{text:?}");
        assert!(text.contains("got:hello"), "{text:?}");
        assert_eq!(
            runner.session().unwrap().lifecycle,
            PtySessionLifecycle::Exited
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn pty_runner_bounds_input_and_forces_ignored_termination() {
        let (root, spec) = fixture("trap '' TERM; echo ready; while :; do sleep 1; done");
        let mut runner = PtyRunner::default().with_termination_grace(Duration::from_millis(20));
        runner
            .start(
                spec,
                BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
            )
            .await
            .unwrap();
        loop {
            if let PtyRunnerEvent::Output { bytes, .. } = runner.next_event().await.unwrap()
                && String::from_utf8_lossy(&bytes).contains("ready")
            {
                break;
            }
        }
        assert!(matches!(
            runner
                .input(PtyClientId([2; 16]), 0, &vec![0; MAX_PTY_INPUT_BYTES + 1])
                .await,
            Err(PtyRunnerError::InputTooLarge)
        ));
        assert!(runner.terminate().await.unwrap());
        assert_eq!(
            runner.session().unwrap().lifecycle,
            PtySessionLifecycle::Exited
        );
        assert_eq!(runner.session().unwrap().process_group, None);
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn pty_runner_preserves_raw_bytes_and_terminates_gracefully() {
        let (root, spec) = fixture("stty raw -echo; printf R; dd bs=1 count=3 2>/dev/null");
        let mut runner = PtyRunner::default();
        runner
            .start(
                spec,
                BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
            )
            .await
            .unwrap();
        let client = PtyClientId([3; 16]);
        runner
            .apply_session_action(PtySessionAction::Attach(client))
            .unwrap();
        runner
            .apply_session_action(PtySessionAction::TakeControl {
                client,
                expected_epoch: 0,
            })
            .unwrap();
        loop {
            if let PtyRunnerEvent::Output { bytes, .. } = runner.next_event().await.unwrap()
                && bytes.contains(&b'R')
            {
                break;
            }
        }
        let raw = [0xff, 0xfe, 0x80];
        runner.input(client, 1, &raw).await.unwrap();
        let output = collect_until_exit(&mut runner).await;
        assert!(output.windows(raw.len()).any(|window| window == raw));
        fs::remove_dir_all(root).unwrap();

        let (root, spec) = fixture("trap 'exit 0' TERM; echo ready; while :; do sleep 1; done");
        let mut runner = PtyRunner::default().with_termination_grace(Duration::from_secs(1));
        runner
            .start(
                spec,
                BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
            )
            .await
            .unwrap();
        loop {
            if let PtyRunnerEvent::Output { bytes, .. } = runner.next_event().await.unwrap()
                && String::from_utf8_lossy(&bytes).contains("ready")
            {
                break;
            }
        }
        assert!(!runner.terminate().await.unwrap());
        assert_eq!(
            runner.session().unwrap().exit_status,
            Some(PtyExitStatus::Code(0))
        );
        fs::remove_dir_all(root).unwrap();
    }
}
