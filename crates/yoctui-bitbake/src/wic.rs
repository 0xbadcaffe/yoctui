use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};

use crate::{WicRunnerEvent, WicRunnerOutputStream, output_text};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader},
    process::{Child, Command},
};
use yoctui_model::{
    MAX_WIC_KICKSTARTS, MAX_WIC_SOURCE_BYTES, WicCapability, WicCreatePreview, WicCreateRequest,
    WicKickstart, WicKickstartIdentity, WicOutput, WicOutputIdentity, WicOutputKind,
    WicPartitionSummary, normalize_wic_capability,
};

const MAX_WIC_LIST_BYTES: u64 = 256 * 1024;
const WIC_INSPECTION_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_WIC_LINE_BYTES: usize = 64 * 1024;
const MAX_WIC_OUTPUT_ENTRIES: usize = 4_096;
const WIC_EVENT_CHANNEL_CAPACITY: usize = 256;
type WicOutputSnapshot = BTreeMap<PathBuf, (u64, u128)>;
type WicOutputScan = (WicOutputSnapshot, Vec<String>);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WicAdapterError {
    #[error("unsafe Wic executable: {0}")]
    UnsafeExecutable(PathBuf),
    #[error("unsafe Wic kickstart: {0}")]
    UnsafeKickstart(PathBuf),
    #[error("unsafe Wic output directory: {0}")]
    UnsafeOutputDirectory(PathBuf),
    #[error("Wic capability command failed: {0}")]
    Capability(String),
    #[error("invalid Wic request: {0}")]
    InvalidRequest(String),
    #[error("Wic preview does not match the independently validated command")]
    PreviewMismatch,
    #[error("a Wic process or unconsumed terminal event is already active")]
    Busy,
    #[error("could not start Wic: {0}")]
    Spawn(String),
    #[error("Wic runner is not active")]
    NotRunning,
    #[error("Wic process control failed: {0}")]
    ProcessControl(String),
    #[error("Wic output scan failed: {0}")]
    OutputScan(String),
}

#[derive(Debug, Clone)]
pub struct WicCapabilityInspector {
    executable: PathBuf,
    configured_kickstarts: Vec<PathBuf>,
    canned_roots: Vec<PathBuf>,
}

impl Default for WicCapabilityInspector {
    fn default() -> Self {
        Self {
            executable: "wic".into(),
            configured_kickstarts: Vec::new(),
            canned_roots: Vec::new(),
        }
    }
}

impl WicCapabilityInspector {
    pub fn with_executable(executable: PathBuf) -> Self {
        Self {
            executable,
            ..Self::default()
        }
    }

    pub fn with_sources(
        mut self,
        configured_kickstarts: Vec<PathBuf>,
        canned_roots: Vec<PathBuf>,
    ) -> Self {
        self.configured_kickstarts = configured_kickstarts;
        self.canned_roots = canned_roots;
        self
    }

    pub async fn inspect(&self, image_targets: Vec<String>) -> WicCapability {
        let executable = match resolve_executable(&self.executable) {
            Ok(Some(executable)) => executable,
            Ok(None) => return WicCapability::MissingTool,
            Err(message) => return WicCapability::Failed { message },
        };
        let listed = match list_canned(&executable).await {
            Ok(listed) => listed,
            Err(error) => {
                return WicCapability::Failed {
                    message: error.to_string(),
                };
            }
        };
        let mut kickstarts = Vec::new();
        for path in &self.configured_kickstarts {
            match read_kickstart(path, None) {
                Ok(kickstart) => kickstarts.push(kickstart),
                Err(error) => {
                    return WicCapability::Failed {
                        message: error.to_string(),
                    };
                }
            }
        }
        for name in listed.into_iter().take(MAX_WIC_KICKSTARTS) {
            let path = self.canned_roots.iter().find_map(|root| {
                [
                    root.join(format!("{name}.wks")),
                    root.join(format!("{name}.wks.in")),
                ]
                .into_iter()
                .find(|path| path.exists())
            });
            match path {
                Some(path) => match read_kickstart(&path, Some(name)) {
                    Ok(kickstart) => kickstarts.push(kickstart),
                    Err(error) => {
                        return WicCapability::Failed {
                            message: error.to_string(),
                        };
                    }
                },
                None => kickstarts.push(WicKickstart {
                    identity: WicKickstartIdentity { name, path: None },
                    source: String::new(),
                    partitions: Vec::new(),
                    limitations: vec!["canned kickstart source is unavailable".into()],
                }),
            }
        }
        normalize_wic_capability(WicCapability::Available {
            executable,
            kickstarts,
            image_targets,
        })
    }
}

async fn list_canned(executable: &Path) -> Result<Vec<String>, WicAdapterError> {
    let mut command = Command::new(executable);
    command
        .args(["list", "images"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|error| WicAdapterError::Capability(error.to_string()))?;
    let stdout = child.stdout.take().ok_or_else(|| {
        WicAdapterError::Capability("wic list images stdout is unavailable".into())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        WicAdapterError::Capability("wic list images stderr is unavailable".into())
    })?;
    let read = async move {
        let mut stdout_bytes = Vec::new();
        let mut stderr_bytes = Vec::new();
        let mut bounded_stdout = stdout.take(MAX_WIC_LIST_BYTES + 1);
        let mut bounded_stderr = stderr.take(MAX_WIC_LIST_BYTES + 1);
        let stdout_read = bounded_stdout.read_to_end(&mut stdout_bytes);
        let stderr_read = bounded_stderr.read_to_end(&mut stderr_bytes);
        let (stdout_result, stderr_result, status) =
            tokio::join!(stdout_read, stderr_read, child.wait());
        stdout_result.map_err(|error| WicAdapterError::Capability(error.to_string()))?;
        stderr_result.map_err(|error| WicAdapterError::Capability(error.to_string()))?;
        let status = status.map_err(|error| WicAdapterError::Capability(error.to_string()))?;
        if stdout_bytes.len() as u64 > MAX_WIC_LIST_BYTES
            || stderr_bytes.len() as u64 > MAX_WIC_LIST_BYTES
        {
            return Err(WicAdapterError::Capability(
                "wic list images output exceeded its safety bound".into(),
            ));
        }
        if !status.success() {
            return Err(WicAdapterError::Capability(
                String::from_utf8_lossy(&stderr_bytes).trim().to_owned(),
            ));
        }
        let output = String::from_utf8(stdout_bytes)
            .map_err(|error| WicAdapterError::Capability(error.to_string()))?;
        let mut names = Vec::new();
        for line in output.lines().filter(|line| !line.trim().is_empty()) {
            let Some(name) = line.split_ascii_whitespace().next() else {
                continue;
            };
            if name.len() <= 256
                && name.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
                })
            {
                names.push(name.to_owned());
            } else {
                return Err(WicAdapterError::Capability(
                    "wic list images returned a malformed name".into(),
                ));
            }
        }
        names.sort();
        names.dedup();
        Ok(names)
    };
    tokio::time::timeout(WIC_INSPECTION_TIMEOUT, read)
        .await
        .map_err(|_| WicAdapterError::Capability("wic list images timed out".into()))?
}

fn read_kickstart(
    path: &Path,
    canned_name: Option<String>,
) -> Result<WicKickstart, WicAdapterError> {
    let canonical =
        regular_canonical(path).map_err(|_| WicAdapterError::UnsafeKickstart(path.into()))?;
    let bytes = fs::read(&canonical).map_err(|_| WicAdapterError::UnsafeKickstart(path.into()))?;
    if bytes.len() > MAX_WIC_SOURCE_BYTES {
        return Err(WicAdapterError::UnsafeKickstart(path.into()));
    }
    let source =
        String::from_utf8(bytes).map_err(|_| WicAdapterError::UnsafeKickstart(path.into()))?;
    let name = canned_name.unwrap_or_else(|| {
        canonical
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .trim_end_matches(".in")
            .trim_end_matches(".wks")
            .to_owned()
    });
    let (partitions, limitations) = parse_kickstart(&source);
    WicKickstart {
        identity: WicKickstartIdentity {
            name,
            path: Some(canonical),
        },
        source,
        partitions,
        limitations,
    }
    .normalize()
    .map_err(|_| WicAdapterError::UnsafeKickstart(path.into()))
}

fn parse_kickstart(source: &str) -> (Vec<WicPartitionSummary>, Vec<String>) {
    let mut partitions = Vec::new();
    let mut limitations = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut tokens = line.split_ascii_whitespace();
        let Some(command) = tokens.next() else {
            continue;
        };
        if !matches!(command, "part" | "partition") {
            if command != "bootloader" {
                limitations.push(format!("unsupported kickstart command: {command}"));
            }
            continue;
        }
        let mount_point = tokens
            .next()
            .filter(|value| !value.starts_with("--"))
            .map(str::to_owned);
        let mut partition = WicPartitionSummary {
            mount_point,
            filesystem: None,
            source_plugin: None,
            size_mib: None,
            alignment_kib: None,
        };
        for token in line.split_ascii_whitespace().skip(1) {
            if let Some(value) = token.strip_prefix("--fstype=") {
                partition.filesystem = Some(value.into());
            } else if let Some(value) = token.strip_prefix("--source=") {
                partition.source_plugin = Some(value.into());
            } else if let Some(value) = token.strip_prefix("--size=") {
                partition.size_mib = value.parse().ok();
                if partition.size_mib.is_none() {
                    limitations.push("dynamic or invalid partition size".into());
                }
            } else if let Some(value) = token.strip_prefix("--align=") {
                partition.alignment_kib = value.parse().ok();
                if partition.alignment_kib.is_none() {
                    limitations.push("dynamic or invalid partition alignment".into());
                }
            } else if token.contains("${") {
                limitations.push("variable-derived partition option".into());
            }
        }
        partitions.push(partition);
    }
    (partitions, limitations)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicCreateCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl WicCreateCommandSpec {
    pub fn from_preview(
        preview: &WicCreatePreview,
        capability: &WicCapability,
    ) -> Result<Self, WicAdapterError> {
        preview
            .request
            .validate()
            .map_err(|message| WicAdapterError::InvalidRequest(message.into()))?;
        let (inspected_executable, inspected_kickstart) = capability
            .resolve(&preview.request.kickstart, &preview.request.image)
            .map_err(|message| WicAdapterError::InvalidRequest(message.into()))?;
        if inspected_kickstart != &preview.kickstart
            || preview.argv.first().map(PathBuf::as_path) != Some(inspected_executable)
        {
            return Err(WicAdapterError::PreviewMismatch);
        }
        let executable = regular_executable(inspected_executable)?;
        if let Some(path) = &preview.request.kickstart.path {
            regular_canonical(path).map_err(|_| WicAdapterError::UnsafeKickstart(path.clone()))?;
        }
        canonical_directory(&preview.request.output_directory)?;
        let expected = create_arguments(&preview.request);
        if preview
            .argv
            .iter()
            .skip(1)
            .map(|argument| argument.as_os_str())
            .ne(expected.iter().map(OsString::as_os_str))
        {
            return Err(WicAdapterError::PreviewMismatch);
        }
        Ok(Self {
            executable,
            arguments: expected,
        })
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}

fn create_arguments(request: &WicCreateRequest) -> Vec<OsString> {
    let mut arguments = vec![
        "create".into(),
        request.kickstart.argument().into_os_string(),
        "-e".into(),
        request.image.clone().into(),
        "-o".into(),
        request.output_directory.as_os_str().to_owned(),
    ];
    if request.generate_bmap {
        arguments.push("--bmap".into());
    }
    if let Some(compression) = request.compression.argument() {
        arguments.extend(["--compress-with".into(), compression.into()]);
    }
    arguments
}

fn resolve_executable(program: &Path) -> Result<Option<PathBuf>, String> {
    if program.is_absolute() {
        return if program.exists() {
            regular_executable(program)
                .map(Some)
                .map_err(|error| error.to_string())
        } else {
            Ok(None)
        };
    }
    if program.components().count() != 1
        || !matches!(program.components().next(), Some(Component::Normal(_)))
    {
        return Err("relative Wic executable candidates are ambiguous".into());
    }
    let Some(path) = std::env::var_os("PATH") else {
        return Ok(None);
    };
    for directory in std::env::split_paths(&path).filter(|path| path.is_absolute()) {
        let candidate = directory.join(program);
        if candidate.exists() {
            return regular_executable(&candidate)
                .map(Some)
                .map_err(|error| error.to_string());
        }
    }
    Ok(None)
}

fn regular_executable(path: &Path) -> Result<PathBuf, WicAdapterError> {
    let canonical =
        regular_canonical(path).map_err(|_| WicAdapterError::UnsafeExecutable(path.into()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(&canonical)
            .map_err(|_| WicAdapterError::UnsafeExecutable(path.into()))?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err(WicAdapterError::UnsafeExecutable(path.into()));
        }
    }
    Ok(canonical)
}

fn regular_canonical(path: &Path) -> Result<PathBuf, ()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if !path.is_absolute() || metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(());
    }
    let canonical = fs::canonicalize(path).map_err(|_| ())?;
    (canonical == path).then_some(canonical).ok_or(())
}

fn canonical_directory(path: &Path) -> Result<PathBuf, WicAdapterError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| WicAdapterError::UnsafeOutputDirectory(path.into()))?;
    let canonical =
        fs::canonicalize(path).map_err(|_| WicAdapterError::UnsafeOutputDirectory(path.into()))?;
    if !path.is_absolute()
        || metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || canonical != path
    {
        return Err(WicAdapterError::UnsafeOutputDirectory(path.into()));
    }
    Ok(canonical)
}

#[derive(Debug)]
enum WicPipeEvent {
    Output {
        stream: WicRunnerOutputStream,
        line: String,
        truncated: bool,
    },
    Failed(String),
}

async fn read_wic_output<R>(
    stream: R,
    kind: WicRunnerOutputStream,
    sender: tokio::sync::mpsc::Sender<WicPipeEvent>,
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
                let _ = sender.send(WicPipeEvent::Failed(error.to_string())).await;
                return;
            }
        };
        if buffer.is_empty() {
            if !bytes.is_empty() || truncated {
                let _ = sender
                    .send(WicPipeEvent::Output {
                        stream: kind,
                        line: output_text(&bytes),
                        truncated,
                    })
                    .await;
            }
            return;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        if !truncated {
            let remaining = MAX_WIC_LINE_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(WicPipeEvent::Output {
                    stream: kind,
                    line: output_text(&bytes),
                    truncated,
                })
                .await
                .is_err()
            {
                return;
            }
            bytes.clear();
            truncated = false;
        }
    }
}

pub struct WicJobRunner {
    build_dir: PathBuf,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<WicPipeEvent>>,
    start_events_pending: u8,
    terminal_pending: VecDeque<WicRunnerEvent>,
    output_root: Option<PathBuf>,
    before: WicOutputSnapshot,
    cancellation_timeout: Duration,
    execution_timeout: Duration,
    started_at: Option<Instant>,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl WicJobRunner {
    pub fn new(build_dir: PathBuf) -> Self {
        Self {
            build_dir,
            child: None,
            output: None,
            start_events_pending: 0,
            terminal_pending: VecDeque::new(),
            output_root: None,
            before: BTreeMap::new(),
            cancellation_timeout: Duration::from_secs(5),
            execution_timeout: Duration::from_secs(60 * 60),
            started_at: None,
            cancellation_requested: false,
            #[cfg(unix)]
            process_group: None,
        }
    }

    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }

    pub fn with_execution_timeout(mut self, timeout: Duration) -> Self {
        self.execution_timeout = timeout;
        self
    }

    pub async fn start(
        &mut self,
        command: WicCreateCommandSpec,
        output_directory: PathBuf,
    ) -> Result<(), WicAdapterError> {
        if self.child.is_some()
            || self.output.is_some()
            || self.start_events_pending > 0
            || !self.terminal_pending.is_empty()
        {
            return Err(WicAdapterError::Busy);
        }
        let output_root = canonical_directory(&output_directory)?;
        let (before, _) = scan_outputs(&output_root)?;
        if !self.build_dir.is_dir() {
            return Err(WicAdapterError::Spawn(format!(
                "build directory does not exist: {}",
                self.build_dir.display()
            )));
        }
        let mut process = Command::new(&command.executable);
        process
            .args(&command.arguments)
            .current_dir(&self.build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        process.process_group(0);
        let mut child = process
            .spawn()
            .map_err(|error| WicAdapterError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| WicAdapterError::Spawn("stdout is unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| WicAdapterError::Spawn("stderr is unavailable".into()))?;
        let (sender, receiver) = tokio::sync::mpsc::channel(WIC_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_wic_output(
            stdout,
            WicRunnerOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_wic_output(
            stderr,
            WicRunnerOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.child = Some(child);
        self.output = Some(receiver);
        self.start_events_pending = 2;
        self.output_root = Some(output_root);
        self.before = before;
        self.cancellation_requested = false;
        self.started_at = Some(Instant::now());
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<WicRunnerEvent, WicAdapterError> {
        if self.start_events_pending == 2 {
            self.start_events_pending = 1;
            return Ok(WicRunnerEvent::Starting);
        }
        if self.start_events_pending == 1 {
            self.start_events_pending = 0;
            return Ok(WicRunnerEvent::Started);
        }
        let remaining = self.remaining();
        if let Some(receiver) = self.output.as_mut() {
            match tokio::time::timeout(remaining, receiver.recv()).await {
                Err(_) => {
                    self.kill_and_clear().await;
                    return Ok(WicRunnerEvent::Failed {
                        message: "wic create timed out".into(),
                        exit_code: None,
                    });
                }
                Ok(Some(WicPipeEvent::Output {
                    stream,
                    line,
                    truncated,
                })) => {
                    return Ok(WicRunnerEvent::Output {
                        stream,
                        line,
                        truncated,
                    });
                }
                Ok(Some(WicPipeEvent::Failed(message))) => {
                    self.kill_and_clear().await;
                    return Ok(WicRunnerEvent::Lost { message });
                }
                Ok(None) => {
                    self.output = None;
                }
            }
        }
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        let remaining = self.remaining();
        let child = self.child.as_mut().ok_or(WicAdapterError::NotRunning)?;
        let status = match tokio::time::timeout(remaining, child.wait()).await {
            Ok(Ok(status)) => status,
            Ok(Err(error)) => {
                self.kill_and_clear().await;
                return Ok(WicRunnerEvent::Lost {
                    message: format!("Wic process wait failed: {error}"),
                });
            }
            Err(_) => {
                self.kill_and_clear().await;
                return Ok(WicRunnerEvent::Failed {
                    message: "wic create timed out".into(),
                    exit_code: None,
                });
            }
        };
        self.child = None;
        self.clear_process_state();
        if !status.success() {
            return Ok(WicRunnerEvent::Failed {
                message: "wic create exited unsuccessfully".into(),
                exit_code: status.code(),
            });
        }
        let root = self
            .output_root
            .take()
            .ok_or_else(|| WicAdapterError::OutputScan("output root was lost".into()))?;
        let (after, limitations) = scan_outputs(&root)?;
        let outputs = after
            .into_iter()
            .filter(|(path, identity)| self.before.get(path) != Some(identity))
            .map(|(path, (size_bytes, modified_nanoseconds))| WicOutput {
                kind: classify_output(&path),
                identity: WicOutputIdentity {
                    path,
                    size_bytes,
                    modified_unix_seconds: (modified_nanoseconds / 1_000_000_000) as u64,
                },
            })
            .collect();
        self.before.clear();
        Ok(WicRunnerEvent::Completed {
            exit_code: status.code().unwrap_or(0),
            outputs,
            limitations,
        })
    }

    pub async fn cancel(&mut self) -> Result<bool, WicAdapterError> {
        if self.cancellation_requested || self.child.is_none() {
            self.terminal_pending
                .push_back(WicRunnerEvent::CancellationRejected {
                    message: "no cancellable Wic process is active".into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        let child = self.child.as_mut().expect("checked above");
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(group) = self.process_group {
            if unsafe { libc::kill(-group, libc::SIGTERM) } != 0 {
                child
                    .start_kill()
                    .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                Ok(result) => {
                    result.map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?
                }
                Err(_) => {
                    let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
                    forced = true;
                    child
                        .wait()
                        .await
                        .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?
                }
            }
        } else {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?
        };
        self.child = None;
        self.clear_process_state();
        self.terminal_pending.push_back(WicRunnerEvent::Cancelled {
            forced,
            exit_code: status.code(),
        });
        Ok(true)
    }

    async fn kill_and_clear(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.child = None;
        self.output = None;
        self.clear_process_state();
    }

    fn clear_process_state(&mut self) {
        self.cancellation_requested = false;
        self.started_at = None;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
    }

    fn remaining(&self) -> Duration {
        self.started_at
            .map(|started| self.execution_timeout.saturating_sub(started.elapsed()))
            .unwrap_or(self.execution_timeout)
    }
}

impl Drop for WicJobRunner {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(group) = self.process_group {
            let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }
}

fn scan_outputs(root: &Path) -> Result<WicOutputScan, WicAdapterError> {
    let root = canonical_directory(root)?;
    let mut files = BTreeMap::new();
    let mut limitations = Vec::new();
    for (index, entry) in fs::read_dir(&root)
        .map_err(|error| WicAdapterError::OutputScan(error.to_string()))?
        .enumerate()
    {
        if index >= MAX_WIC_OUTPUT_ENTRIES {
            limitations.push(format!(
                "Wic output scan was limited to {MAX_WIC_OUTPUT_ENTRIES} entries"
            ));
            break;
        }
        let Ok(entry) = entry else {
            limitations.push("one Wic output entry was unreadable".into());
            continue;
        };
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            limitations.push(format!("metadata unavailable for {}", path.display()));
            continue;
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }
        let Ok(canonical) = fs::canonicalize(&path) else {
            limitations.push(format!("could not canonicalize {}", path.display()));
            continue;
        };
        if canonical != path || !canonical.starts_with(&root) {
            limitations.push(format!("unsafe Wic output ignored: {}", path.display()));
            continue;
        }
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_nanos());
        files.insert(canonical, (metadata.len(), modified));
    }
    Ok((files, limitations))
}

fn classify_output(path: &Path) -> WicOutputKind {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if name.ends_with(".wic") {
        WicOutputKind::Wic
    } else if name.ends_with(".direct") {
        WicOutputKind::Direct
    } else if name.ends_with(".bmap") {
        WicOutputKind::Bmap
    } else if name.ends_with(".gz") || name.ends_with(".bz2") || name.ends_with(".xz") {
        WicOutputKind::Compressed
    } else {
        WicOutputKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use yoctui_model::{WicCompression, WicCreateDraft};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

    fn fixture(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "yoctui-wic-capability-{}-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        fs::canonicalize(path).unwrap()
    }

    #[cfg(unix)]
    fn executable(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_capability_discovers_parses_and_constructs_exact_command() {
        let directory = fixture("exact");
        let program = directory.join("wic");
        executable(
            &program,
            "test \"$1 $2\" = 'list images' && printf 'directdisk  Direct disk\\ncustom Custom\\n'",
        );
        let canned = directory.join("canned");
        fs::create_dir(&canned).unwrap();
        let canned = fs::canonicalize(canned).unwrap();
        fs::write(
            canned.join("directdisk.wks"),
            "part / --source=rootfs --fstype=ext4 --size=64 --align=4\nbootloader --ptable gpt\n",
        )
        .unwrap();
        fs::write(
            canned.join("custom.wks.in"),
            "part /boot --source=bootimg --size=${BOOT_SIZE}\nunsupported value\n",
        )
        .unwrap();
        let capability = WicCapabilityInspector::with_executable(program)
            .with_sources(Vec::new(), vec![canned])
            .inspect(vec!["core-image-minimal".into()])
            .await;
        let WicCapability::Available { kickstarts, .. } = &capability else {
            panic!("available capability: {capability:?}");
        };
        assert_eq!(kickstarts.len(), 2);
        assert_eq!(
            kickstarts[1].partitions[0].mount_point.as_deref(),
            Some("/")
        );
        assert_eq!(kickstarts[1].partitions[0].size_mib, Some(64));
        assert!(
            kickstarts[0]
                .limitations
                .iter()
                .any(|limitation| limitation.contains("dynamic"))
        );

        let output = directory.join("output");
        fs::create_dir(&output).unwrap();
        let output = fs::canonicalize(output).unwrap();
        let draft = WicCreateDraft {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            kickstart: kickstarts[1].identity.clone(),
            output_directory: output.display().to_string(),
            generate_bmap: true,
            compression: WicCompression::Gzip,
        };
        let preview = draft.preview(&capability).unwrap();
        let command = WicCreateCommandSpec::from_preview(&preview, &capability).unwrap();
        assert_eq!(
            command.arguments(),
            &[
                OsString::from("create"),
                kickstarts[1]
                    .identity
                    .path
                    .as_ref()
                    .unwrap()
                    .as_os_str()
                    .to_owned(),
                "-e".into(),
                "core-image-minimal".into(),
                "-o".into(),
                output.as_os_str().to_owned(),
                "--bmap".into(),
                "--compress-with".into(),
                "gzip".into(),
            ]
        );
        let alternate = directory.join("alternate-wic");
        executable(&alternate, "exit 0");
        let alternate = fs::canonicalize(alternate).unwrap();
        let mut changed_capability = capability.clone();
        if let WicCapability::Available { executable, .. } = &mut changed_capability {
            *executable = alternate;
        }
        assert_eq!(
            WicCreateCommandSpec::from_preview(&preview, &changed_capability).unwrap_err(),
            WicAdapterError::PreviewMismatch
        );
        let mut tampered = preview;
        tampered.argv.push("--debug".into());
        assert_eq!(
            WicCreateCommandSpec::from_preview(&tampered, &capability).unwrap_err(),
            WicAdapterError::PreviewMismatch
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_capability_reports_missing_malformed_and_unsafe_sources() {
        assert_eq!(
            WicCapabilityInspector::with_executable("/missing/wic".into())
                .inspect(vec!["core-image-minimal".into()])
                .await,
            WicCapability::MissingTool
        );
        let directory = fixture("unsafe");
        let program = directory.join("wic");
        executable(&program, "printf 'bad/name malformed\\n'");
        assert!(matches!(
            WicCapabilityInspector::with_executable(program.clone())
                .inspect(vec!["core-image-minimal".into()])
                .await,
            WicCapability::Failed { .. }
        ));
        let target = directory.join("target.wks");
        fs::write(&target, "part /\n").unwrap();
        let link = directory.join("linked.wks");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        executable(&program, "exit 0");
        assert!(matches!(
            WicCapabilityInspector::with_executable(program)
                .with_sources(vec![link], Vec::new())
                .inspect(vec!["core-image-minimal".into()])
                .await,
            WicCapability::Failed { .. }
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    async fn runner_fixture(name: &str, body: &str) -> (PathBuf, PathBuf, WicCreateCommandSpec) {
        let directory = fixture(name);
        let program = directory.join("wic");
        executable(&program, body);
        let kickstart_path = directory.join("directdisk.wks");
        fs::write(&kickstart_path, "part / --source=rootfs\n").unwrap();
        let kickstart_path = fs::canonicalize(kickstart_path).unwrap();
        let output = directory.join("output");
        fs::create_dir(&output).unwrap();
        let output = fs::canonicalize(output).unwrap();
        let capability = WicCapability::Available {
            executable: fs::canonicalize(program).unwrap(),
            kickstarts: vec![read_kickstart(&kickstart_path, Some("directdisk".into())).unwrap()],
            image_targets: vec!["core-image-minimal".into()],
        };
        let preview = WicCreateDraft {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            kickstart: WicKickstartIdentity {
                name: "directdisk".into(),
                path: Some(kickstart_path),
            },
            output_directory: output.display().to_string(),
            generate_bmap: false,
            compression: WicCompression::None,
        }
        .preview(&capability)
        .unwrap();
        let command = WicCreateCommandSpec::from_preview(&preview, &capability).unwrap();
        (directory, output, command)
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_runner_reports_only_new_outputs_and_nonzero_failure() {
        let (directory, output, command) = runner_fixture(
            "runner-success",
            "printf 'before\\n'; printf 'warning\\n' >&2; printf image > \"$6/new.wic\"; exit 0",
        )
        .await;
        fs::write(output.join("existing.wic"), "old").unwrap();
        let mut runner = WicJobRunner::new(directory.clone());
        runner.start(command, output.clone()).await.unwrap();
        assert_eq!(runner.next_event().await.unwrap(), WicRunnerEvent::Starting);
        assert_eq!(runner.next_event().await.unwrap(), WicRunnerEvent::Started);
        let terminal = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let event = runner.next_event().await.unwrap();
                if matches!(event, WicRunnerEvent::Completed { .. }) {
                    break event;
                }
            }
        })
        .await
        .unwrap();
        let WicRunnerEvent::Completed { outputs, .. } = terminal else {
            unreachable!()
        };
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].identity.path.ends_with("new.wic"));
        fs::remove_dir_all(directory).unwrap();

        let (directory, output, command) =
            runner_fixture("runner-failure", "printf failed >&2; exit 9").await;
        let mut runner = WicJobRunner::new(directory.clone());
        runner.start(command, output).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let terminal = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let event = runner.next_event().await.unwrap();
                if matches!(event, WicRunnerEvent::Failed { .. }) {
                    break event;
                }
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            terminal,
            WicRunnerEvent::Failed {
                exit_code: Some(9),
                ..
            }
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_runner_rejects_duplicate_and_forces_cancellation() {
        let (directory, output, command) = runner_fixture(
            "runner-cancel",
            "trap '' TERM; printf 'ready\\n'; while :; do :; done",
        )
        .await;
        let mut runner = WicJobRunner::new(directory.clone())
            .with_cancellation_timeout(Duration::from_millis(50));
        runner.start(command.clone(), output.clone()).await.unwrap();
        assert_eq!(
            runner.start(command, output).await.unwrap_err(),
            WicAdapterError::Busy
        );
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::Starting
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::Started
        ));
        loop {
            if matches!(
                runner.next_event().await.unwrap(),
                WicRunnerEvent::Output { ref line, .. } if line == "ready"
            ) {
                break;
            }
        }
        assert!(runner.cancel().await.unwrap());
        let cancelled = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let event = runner.next_event().await.unwrap();
                if matches!(event, WicRunnerEvent::Cancelled { .. }) {
                    break event;
                }
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            cancelled,
            WicRunnerEvent::Cancelled { forced: true, .. }
        ));
        assert!(!runner.cancel().await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::CancellationRejected { .. }
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_runner_times_out_without_blocking_forever() {
        let (directory, output, command) = runner_fixture("runner-timeout", "sleep 30").await;
        let mut runner =
            WicJobRunner::new(directory.clone()).with_execution_timeout(Duration::from_millis(20));
        runner.start(command, output).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::Failed {
                ref message,
                exit_code: None
            } if message.contains("timed out")
        ));
        fs::remove_dir_all(directory).unwrap();
    }
}
