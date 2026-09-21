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
