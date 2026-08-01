use std::{
    collections::VecDeque,
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
};
use yoctui_model::{
    ImageArtifact, ImageArtifactIdentity, ImageArtifactKind, QemuCapability, QemuDisplayMode,
    QemuLaunchPreview, QemuLaunchRequest, QemuNetworkingMode, QemuSerialMode,
};

use crate::{QemuRunnerEvent, QemuRunnerOutputStream, output_text};

const MAX_QEMU_LINE_BYTES: usize = 64 * 1024;
const QEMU_EVENT_CHANNEL_CAPACITY: usize = 256;

#[derive(Debug, Clone)]
pub struct QemuCapabilityInspector {
    executable: PathBuf,
}

impl Default for QemuCapabilityInspector {
    fn default() -> Self {
        Self {
            executable: "runqemu".into(),
        }
    }
}

impl QemuCapabilityInspector {
    pub fn with_executable(executable: PathBuf) -> Self {
        Self { executable }
    }

    pub fn inspect(&self, artifacts: &[ImageArtifact]) -> QemuCapability {
        let executable = match resolve_executable(&self.executable) {
            Ok(Some(executable)) => executable,
            Ok(None) => return QemuCapability::MissingTool,
            Err(message) => return QemuCapability::Failed { message },
        };
        let mut compatible_images = Vec::new();
        for artifact in artifacts.iter().filter(|artifact| {
            matches!(
                artifact.kind,
                ImageArtifactKind::RootFilesystem | ImageArtifactKind::Wic
            )
        }) {
            if let Err(message) = validate_artifact_file(&artifact.identity) {
                return QemuCapability::Failed {
                    message: message.into(),
                };
            }
            compatible_images.push(artifact.identity.clone());
        }
        compatible_images.sort();
        compatible_images.dedup();
        if compatible_images.is_empty() {
            QemuCapability::MissingCompatibleImage
        } else {
            QemuCapability::Available {
                executable,
                compatible_images,
            }
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum QemuAdapterError {
    #[error("invalid runqemu request: {0}")]
    InvalidRequest(String),
    #[error("runqemu preview does not match the validated request")]
    PreviewMismatch,
    #[error("unsafe runqemu executable: {0}")]
    UnsafeExecutable(PathBuf),
    #[error("unsafe runqemu artifact path: {0}")]
    UnsafeArtifact(PathBuf),
    #[error("a runqemu process or unconsumed terminal event is already active")]
    Busy,
    #[error("runqemu executable is missing: {0}")]
    MissingExecutable(PathBuf),
    #[error("could not start runqemu: {0}")]
    Spawn(String),
    #[error("runqemu process stream is unavailable: {0:?}")]
    StreamUnavailable(QemuRunnerOutputStream),
    #[error("runqemu runner is not active")]
    NotRunning,
    #[error("runqemu process control failed: {0}")]
    ProcessControl(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QemuCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl QemuCommandSpec {
    pub fn from_preview(preview: &QemuLaunchPreview) -> Result<Self, QemuAdapterError> {
        preview
            .request
            .validate()
            .map_err(|message| QemuAdapterError::InvalidRequest(message.into()))?;
        let Some(executable) = preview.argv.first() else {
            return Err(QemuAdapterError::PreviewMismatch);
        };
        validate_executable_file(executable)?;
        validate_artifact_file(&preview.request.image)
            .map_err(|_| QemuAdapterError::UnsafeArtifact(preview.request.image.path.clone()))?;
        for path in preview
            .request
            .kernel
            .iter()
            .chain(preview.request.rootfs.iter())
        {
            validate_regular_file(path)
                .map_err(|_| QemuAdapterError::UnsafeArtifact(path.clone()))?;
        }
        let expected = command_arguments(&preview.request);
        if preview
            .argv
            .iter()
            .skip(1)
            .map(|argument| argument.as_os_str())
            .ne(expected.iter().map(OsString::as_os_str))
        {
            return Err(QemuAdapterError::PreviewMismatch);
        }
        Ok(Self {
            executable: executable.clone(),
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

fn command_arguments(request: &QemuLaunchRequest) -> Vec<OsString> {
    let mut arguments = vec![
        OsString::from(&request.machine),
        request.image.path.as_os_str().to_owned(),
        OsString::from(format!("qemumemory={}", request.memory_mib)),
        OsString::from(match request.networking {
            QemuNetworkingMode::Slirp => "slirp",
            QemuNetworkingMode::Tap => "tap",
            QemuNetworkingMode::None => "nonetwork",
        }),
        OsString::from(match request.display {
            QemuDisplayMode::Graphical => "sdl",
            QemuDisplayMode::Nographic => "nographic",
        }),
    ];
    if let Some(kernel) = &request.kernel {
        arguments.push(kernel.as_os_str().to_owned());
    }
    if let Some(rootfs) = &request.rootfs {
        arguments.push(rootfs.as_os_str().to_owned());
    }
    match request.serial {
        QemuSerialMode::Stdio => arguments.push("serialstdio".into()),
        QemuSerialMode::Telnet => arguments.push("serialtelnet".into()),
        QemuSerialMode::None => {}
    }
    arguments.extend(request.extra_arguments.iter().map(OsString::from));
    arguments
}

fn resolve_executable(program: &Path) -> Result<Option<PathBuf>, String> {
    if program.is_absolute() {
        if !program.exists() {
            return Ok(None);
        }
        return validate_executable_file(program)
            .map(Some)
            .map_err(|error| error.to_string());
    }
    if program.components().count() != 1
        || !matches!(program.components().next(), Some(Component::Normal(_)))
    {
        return Err(format!(
            "relative runqemu executable candidates are ambiguous: {}",
            program.display()
        ));
    }
    let Some(path) = std::env::var_os("PATH") else {
        return Ok(None);
    };
    for directory in std::env::split_paths(&path).filter(|directory| directory.is_absolute()) {
        let candidate = directory.join(program);
        if !candidate.exists() {
            continue;
        }
        return validate_executable_file(&candidate)
            .map(Some)
            .map_err(|error| error.to_string());
    }
    Ok(None)
}

fn validate_executable_file(path: &Path) -> Result<PathBuf, QemuAdapterError> {
    let canonical = validate_regular_file(path)
        .map_err(|_| QemuAdapterError::UnsafeExecutable(path.to_path_buf()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&canonical)
            .map_err(|_| QemuAdapterError::UnsafeExecutable(path.to_path_buf()))?
            .permissions()
            .mode();
        if mode & 0o111 == 0 {
            return Err(QemuAdapterError::UnsafeExecutable(path.to_path_buf()));
        }
    }
    Ok(canonical)
}

fn validate_artifact_file(identity: &ImageArtifactIdentity) -> Result<PathBuf, &'static str> {
    identity.validate()?;
    if identity
        .path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        != Some(identity.machine.as_str())
    {
        return Err("artifact path does not match its machine identity");
    }
    validate_regular_file(&identity.path)
}

fn validate_regular_file(path: &Path) -> Result<PathBuf, &'static str> {
    if !path.is_absolute() {
        return Err("path is not absolute");
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| "path does not exist")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("path is not a regular non-symlink file");
    }
    let canonical = fs::canonicalize(path).map_err(|_| "path could not be canonicalized")?;
    if canonical != path {
        return Err("path is not canonical");
    }
    Ok(canonical)
}

#[derive(Debug)]
enum QemuPipeEvent {
    Output {
        stream: QemuRunnerOutputStream,
        line: String,
        truncated: bool,
    },
    Failed {
        stream: QemuRunnerOutputStream,
        message: String,
    },
}

async fn read_qemu_output<R>(
    stream: R,
    kind: QemuRunnerOutputStream,
    sender: tokio::sync::mpsc::Sender<QemuPipeEvent>,
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
                    .send(QemuPipeEvent::Failed {
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
                    .send(QemuPipeEvent::Output {
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
            let remaining = MAX_QEMU_LINE_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(QemuPipeEvent::Output {
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

pub struct QemuJobRunner {
    build_dir: PathBuf,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<QemuPipeEvent>>,
    streams_drained: bool,
    start_events_pending: u8,
    terminal_pending: VecDeque<QemuRunnerEvent>,
    cancellation_timeout: Duration,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl QemuJobRunner {
    pub fn new(build_dir: PathBuf) -> Self {
        Self {
            build_dir,
            child: None,
            output: None,
            streams_drained: true,
            start_events_pending: 0,
            terminal_pending: VecDeque::new(),
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

    pub async fn start(&mut self, command: QemuCommandSpec) -> Result<(), QemuAdapterError> {
        if self.child.is_some()
            || self.start_events_pending > 0
            || !self.terminal_pending.is_empty()
            || self.output.is_some()
        {
            return Err(QemuAdapterError::Busy);
        }
        if !self.build_dir.is_dir() {
            return Err(QemuAdapterError::Spawn(format!(
                "build directory does not exist: {}",
                self.build_dir.display()
            )));
        }
        let mut process = Command::new(&command.executable);
        process
            .args(&command.arguments)
            .current_dir(&self.build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        process.process_group(0);
        let mut child = process.spawn().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                QemuAdapterError::MissingExecutable(command.executable.clone())
            } else {
                QemuAdapterError::Spawn(error.to_string())
            }
        })?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            return Err(QemuAdapterError::StreamUnavailable(
                QemuRunnerOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            return Err(QemuAdapterError::StreamUnavailable(
                QemuRunnerOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(QEMU_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_qemu_output(
            stdout,
            QemuRunnerOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_qemu_output(
            stderr,
            QemuRunnerOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.start_events_pending = 2;
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<QemuRunnerEvent, QemuAdapterError> {
        if self.start_events_pending == 2 {
            self.start_events_pending = 1;
            return Ok(QemuRunnerEvent::Starting);
        }
        if self.start_events_pending == 1 {
            self.start_events_pending = 0;
            return Ok(QemuRunnerEvent::Started);
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            self.kill_and_clear().await;
            return Ok(QemuRunnerEvent::Lost {
                message: "runqemu output event channel was lost".into(),
            });
        }
        if let Some(receiver) = self.output.as_mut() {
            match receiver.recv().await {
                Some(QemuPipeEvent::Output {
                    stream,
                    line,
                    truncated,
                }) => {
                    return Ok(QemuRunnerEvent::Output {
                        stream,
                        line,
                        truncated,
                    });
                }
                Some(QemuPipeEvent::Failed { stream, message }) => {
                    self.kill_and_clear().await;
                    return Ok(QemuRunnerEvent::Lost {
                        message: format!("{stream:?} stream failed: {message}"),
                    });
                }
                None => {
                    self.output = None;
                    self.streams_drained = true;
                }
            }
        }
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        let Some(child) = self.child.as_mut() else {
            return Err(QemuAdapterError::NotRunning);
        };
        let status = match child.wait().await {
            Ok(status) => status,
            Err(error) => {
                self.child = None;
                self.clear_process_state();
                return Ok(QemuRunnerEvent::Lost {
                    message: format!("runqemu process wait failed: {error}"),
                });
            }
        };
        self.child = None;
        self.clear_process_state();
        let exit_code = status.code();
        if status.success() {
            Ok(QemuRunnerEvent::Completed {
                exit_code: exit_code.unwrap_or(0),
            })
        } else {
            Ok(QemuRunnerEvent::Failed {
                message: "runqemu exited unsuccessfully".into(),
                exit_code,
            })
        }
    }

    pub async fn cancel(&mut self) -> Result<bool, QemuAdapterError> {
        if self.cancellation_requested || self.child.is_none() {
            self.terminal_pending
                .push_back(QemuRunnerEvent::CancellationRejected {
                    message: "no cancellable runqemu process is active".into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        let child = self.child.as_mut().expect("checked above");
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(process_group) = self.process_group {
            // SAFETY: this negative PID targets only the group created for this child.
            if unsafe { libc::kill(-process_group, libc::SIGTERM) } != 0 {
                child
                    .start_kill()
                    .map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                Ok(result) => {
                    result.map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?
                }
                Err(_) => {
                    // SAFETY: same child-owned process group as the graceful signal.
                    let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                    forced = true;
                    child
                        .wait()
                        .await
                        .map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?
                }
            }
        } else {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?
        };
        self.child = None;
        self.clear_process_state();
        self.terminal_pending.push_back(QemuRunnerEvent::Cancelled {
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
        self.streams_drained = true;
        self.clear_process_state();
    }

    fn clear_process_state(&mut self) {
        self.cancellation_requested = false;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
    }
}

impl Drop for QemuJobRunner {
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
    use yoctui_model::{ImageArtifactField, QemuLaunchDraft};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

    fn fixture_dir(name: &str) -> PathBuf {
        let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "yoctui-qemu-adapter-{}-{name}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::canonicalize(directory).unwrap()
    }

    #[cfg(unix)]
    fn executable(path: &Path, body: &str) {
        crate::test_support::write_executable(path, &format!("#!/bin/sh\n{body}\n"));
    }

    fn artifact(path: PathBuf, kind: ImageArtifactKind) -> ImageArtifact {
        ImageArtifact {
            identity: ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: "core-image-minimal".into(),
                path,
            },
            kind,
            size_bytes: ImageArtifactField::Unavailable,
            modified_unix_seconds: ImageArtifactField::Unavailable,
            checksums: ImageArtifactField::Unavailable,
            manifests: ImageArtifactField::Unavailable,
            licenses: ImageArtifactField::Unavailable,
            spdx: ImageArtifactField::Unavailable,
            wic_files: ImageArtifactField::Unavailable,
        }
    }

    #[cfg(unix)]
    fn fixture_preview(name: &str, body: &str) -> (PathBuf, QemuLaunchPreview, QemuCommandSpec) {
        let directory = fixture_dir(name);
        let program = directory.join("runqemu");
        executable(&program, body);
        let deploy = directory.join("qemux86-64");
        fs::create_dir(&deploy).unwrap();
        let image_path = deploy.join("core-image-minimal.wic");
        fs::write(&image_path, b"wic").unwrap();
        let image = artifact(image_path, ImageArtifactKind::Wic);
        let capability =
            QemuCapabilityInspector::with_executable(program).inspect(std::slice::from_ref(&image));
        let draft = QemuLaunchDraft::for_artifact(image.identity, image.kind);
        let preview = draft.preview(&capability).unwrap();
        let command = QemuCommandSpec::from_preview(&preview).unwrap();
        (directory, preview, command)
    }

    #[cfg(unix)]
    #[test]
    fn qemu_adapter_capability_distinguishes_available_missing_image_and_failure() {
        let directory = fixture_dir("capability");
        let program = directory.join("runqemu");
        executable(&program, "exit 0");
        let deploy = directory.join("qemux86-64");
        fs::create_dir(&deploy).unwrap();
        let image_path = deploy.join("core-image-minimal.wic");
        fs::write(&image_path, b"wic").unwrap();
        let image = artifact(image_path, ImageArtifactKind::Wic);
        assert!(matches!(
            QemuCapabilityInspector::with_executable(program.clone())
                .inspect(std::slice::from_ref(&image)),
            QemuCapability::Available {
                compatible_images,
                ..
            } if compatible_images == vec![image.identity.clone()]
        ));
        assert_eq!(
            QemuCapabilityInspector::with_executable(directory.join("missing")).inspect(&[]),
            QemuCapability::MissingTool
        );
        assert_eq!(
            QemuCapabilityInspector::with_executable("definitely-missing-runqemu".into())
                .inspect(&[]),
            QemuCapability::MissingTool
        );
        assert_eq!(
            QemuCapabilityInspector::with_executable(program.clone()).inspect(&[]),
            QemuCapability::MissingCompatibleImage
        );
        let stale = artifact(deploy.join("missing.wic"), ImageArtifactKind::Wic);
        assert!(matches!(
            QemuCapabilityInspector::with_executable(program).inspect(&[stale]),
            QemuCapability::Failed { message } if message.contains("does not exist")
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn qemu_adapter_builds_exact_shell_free_arguments_and_rejects_tampering() {
        let (directory, mut preview, command) = fixture_preview("command", "printf '%s\\n' \"$@\"");
        assert_eq!(command.executable(), directory.join("runqemu"));
        assert_eq!(
            command.arguments(),
            [
                OsString::from("qemux86-64"),
                directory
                    .join("qemux86-64/core-image-minimal.wic")
                    .as_os_str()
                    .to_owned(),
                OsString::from("qemumemory=1024"),
                OsString::from("slirp"),
                OsString::from("sdl"),
                OsString::from("serialstdio"),
            ]
        );
        preview.argv.push("--help".into());
        assert_eq!(
            QemuCommandSpec::from_preview(&preview),
            Err(QemuAdapterError::PreviewMismatch)
        );
        preview.argv.pop();
        preview.request.extra_arguments = vec!["--help".into()];
        assert!(matches!(
            QemuCommandSpec::from_preview(&preview),
            Err(QemuAdapterError::InvalidRequest(_))
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn qemu_adapter_rejects_symlinked_artifacts() {
        use std::os::unix::fs::symlink;
        let directory = fixture_dir("symlink");
        let program = directory.join("runqemu");
        executable(&program, "exit 0");
        let deploy = directory.join("qemux86-64");
        fs::create_dir(&deploy).unwrap();
        let target = deploy.join("target.wic");
        let link = deploy.join("core-image-minimal.wic");
        fs::write(&target, b"wic").unwrap();
        symlink(&target, &link).unwrap();
        let image = artifact(link, ImageArtifactKind::Wic);
        assert!(matches!(
            QemuCapabilityInspector::with_executable(program).inspect(&[image]),
            QemuCapability::Failed { message } if message.contains("non-symlink")
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qemu_adapter_streams_bounded_output_and_reports_nonzero_exit() {
        let (directory, _, command) = fixture_preview(
            "output",
            "printf 'stdout\\n'; printf 'stderr\\n' >&2; printf '\\377bad\\n'; head -c 70000 /dev/zero | tr '\\000' x; printf '\\n'; exit 7",
        );
        let mut runner = QemuJobRunner::new(directory.clone());
        runner.start(command.clone()).await.unwrap();
        assert_eq!(runner.start(command).await, Err(QemuAdapterError::Busy));
        assert_eq!(
            runner.next_event().await.unwrap(),
            QemuRunnerEvent::Starting
        );
        assert_eq!(runner.next_event().await.unwrap(), QemuRunnerEvent::Started);
        let mut output = Vec::new();
        loop {
            match runner.next_event().await.unwrap() {
                QemuRunnerEvent::Output {
                    stream,
                    line,
                    truncated,
                } => output.push((stream, line, truncated)),
                QemuRunnerEvent::Failed { exit_code, .. } => {
                    assert_eq!(exit_code, Some(7));
                    break;
                }
                event => panic!("unexpected event: {event:?}"),
            }
        }
        assert!(output.iter().any(|(stream, line, _)| {
            *stream == QemuRunnerOutputStream::Stdout && line == "stdout"
        }));
        assert!(output.iter().any(|(stream, line, _)| {
            *stream == QemuRunnerOutputStream::Stderr && line == "stderr"
        }));
        assert!(output.iter().any(|(_, line, _)| line.contains('\u{fffd}')));
        assert!(
            output
                .iter()
                .any(|(_, line, truncated)| { *truncated && line.len() <= MAX_QEMU_LINE_BYTES })
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qemu_adapter_reports_successful_completion() {
        let (directory, _, command) = fixture_preview("success", "exit 0");
        let mut runner = QemuJobRunner::new(directory.clone());
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            QemuRunnerEvent::Starting
        );
        assert_eq!(runner.next_event().await.unwrap(), QemuRunnerEvent::Started);
        assert_eq!(
            runner.next_event().await.unwrap(),
            QemuRunnerEvent::Completed { exit_code: 0 }
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qemu_adapter_cancels_gracefully_and_escalates_process_groups() {
        for (name, trap, expected_forced) in [
            ("cancel-graceful", "trap 'exit 0' TERM", false),
            ("cancel-forced", "trap '' TERM", true),
        ] {
            let (directory, _, command) = fixture_preview(
                name,
                &format!("{trap}; printf 'ready\\n'; while :; do :; done"),
            );
            let mut runner = QemuJobRunner::new(directory.clone())
                .with_cancellation_timeout(Duration::from_millis(250));
            runner.start(command).await.unwrap();
            let _ = runner.next_event().await.unwrap();
            let _ = runner.next_event().await.unwrap();
            loop {
                if matches!(
                    runner.next_event().await.unwrap(),
                    QemuRunnerEvent::Output { ref line, .. } if line == "ready"
                ) {
                    break;
                }
            }
            assert!(runner.cancel().await.unwrap());
            assert!(!runner.cancel().await.unwrap());
            loop {
                if let QemuRunnerEvent::Cancelled { forced, .. } =
                    runner.next_event().await.unwrap()
                {
                    assert_eq!(forced, expected_forced);
                    break;
                }
            }
            assert!(matches!(
                runner.next_event().await.unwrap(),
                QemuRunnerEvent::CancellationRejected { message }
                    if message.contains("no cancellable")
            ));
            fs::remove_dir_all(directory).unwrap();
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qemu_adapter_reports_unexpected_output_channel_loss() {
        let (directory, _, command) =
            fixture_preview("channel-loss", "printf 'ready\\n'; sleep 30");
        let mut runner = QemuJobRunner::new(directory.clone());
        runner.start(command).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        runner.output = None;
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QemuRunnerEvent::Lost { message } if message.contains("channel")
        ));
        assert!(!runner.is_active());
        fs::remove_dir_all(directory).unwrap();
    }
}
