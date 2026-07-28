use crate::{ImageArtifactIdentity, ImageArtifactKind};
use std::path::{Component, Path, PathBuf};

pub const MIN_QEMU_MEMORY_MIB: u32 = 128;
pub const MAX_QEMU_MEMORY_MIB: u32 = 262_144;
pub const MAX_QEMU_EXTRA_ARGUMENTS: usize = 32;
pub const MAX_QEMU_EXTRA_ARGUMENT_BYTES: usize = 256;
pub const MAX_QEMU_PATH_INPUT_BYTES: usize = 4_096;
pub const MAX_QEMU_EXTRA_ARGUMENT_INPUT_BYTES: usize =
    MAX_QEMU_EXTRA_ARGUMENTS * (MAX_QEMU_EXTRA_ARGUMENT_BYTES + 1);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum QemuCapability {
    #[default]
    NotInspected,
    Available {
        executable: PathBuf,
        compatible_images: Vec<ImageArtifactIdentity>,
    },
    MissingTool,
    MissingCompatibleImage,
    Failed {
        message: String,
    },
}

impl QemuCapability {
    pub fn executable_for(&self, image: &ImageArtifactIdentity) -> Result<&Path, &'static str> {
        let Self::Available {
            executable,
            compatible_images,
        } = self
        else {
            return Err(match self {
                Self::NotInspected => "runqemu capability has not been inspected",
                Self::MissingTool => "runqemu is not available",
                Self::MissingCompatibleImage => "no compatible runqemu image is available",
                Self::Failed { .. } => "runqemu capability inspection failed",
                Self::Available { .. } => unreachable!(),
            });
        };
        if !absolute_normal_path(executable) {
            return Err("runqemu executable paths must be normalized absolute paths");
        }
        if !compatible_images.iter().any(|candidate| candidate == image) {
            return Err("the selected image is not in the inspected runqemu capability");
        }
        Ok(executable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QemuNetworkingMode {
    Slirp,
    Tap,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QemuDisplayMode {
    Graphical,
    Nographic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QemuSerialMode {
    Stdio,
    Telnet,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QemuLaunchRequest {
    pub machine: String,
    pub image: ImageArtifactIdentity,
    pub artifact_kind: ImageArtifactKind,
    pub kernel: Option<PathBuf>,
    pub rootfs: Option<PathBuf>,
    pub networking: QemuNetworkingMode,
    pub display: QemuDisplayMode,
    pub serial: QemuSerialMode,
    pub memory_mib: u32,
    pub extra_arguments: Vec<String>,
}

impl QemuLaunchRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.image.validate()?;
        if self.machine != self.image.machine {
            return Err("runqemu machine and image identities must match");
        }
        if !matches!(
            self.artifact_kind,
            ImageArtifactKind::RootFilesystem | ImageArtifactKind::Wic
        ) {
            return Err("runqemu requires a root filesystem or Wic image artifact");
        }
        if self
            .kernel
            .iter()
            .chain(self.rootfs.iter())
            .any(|path| !absolute_normal_path(path))
        {
            return Err("runqemu kernel and rootfs paths must be normalized absolute paths");
        }
        if !(MIN_QEMU_MEMORY_MIB..=MAX_QEMU_MEMORY_MIB).contains(&self.memory_mib) {
            return Err("runqemu memory is outside the supported safety bounds");
        }
        if self.display == QemuDisplayMode::Nographic && self.serial == QemuSerialMode::None {
            return Err("nographic runqemu sessions require a serial connection");
        }
        if self.extra_arguments.len() > MAX_QEMU_EXTRA_ARGUMENTS
            || self
                .extra_arguments
                .iter()
                .any(|argument| !extra_argument_is_valid(argument))
        {
            return Err("runqemu extra arguments must be bounded unambiguous tokens");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QemuLaunchDraft {
    pub machine: String,
    pub image: ImageArtifactIdentity,
    pub artifact_kind: ImageArtifactKind,
    pub kernel: String,
    pub rootfs: String,
    pub networking: QemuNetworkingMode,
    pub display: QemuDisplayMode,
    pub serial: QemuSerialMode,
    pub memory_mib: String,
    pub extra_arguments: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QemuLaunchField {
    Machine,
    Image,
    Kernel,
    Rootfs,
    Networking,
    Memory,
    Display,
    Serial,
    ExtraArguments,
}

impl QemuLaunchField {
    const ALL: [Self; 9] = [
        Self::Machine,
        Self::Image,
        Self::Kernel,
        Self::Rootfs,
        Self::Networking,
        Self::Memory,
        Self::Display,
        Self::Serial,
        Self::ExtraArguments,
    ];

    pub fn is_read_only(self) -> bool {
        matches!(self, Self::Machine | Self::Image)
    }

    pub fn is_text(self) -> bool {
        matches!(
            self,
            Self::Kernel | Self::Rootfs | Self::Memory | Self::ExtraArguments
        )
    }

    pub fn shifted(self, delta: isize) -> Self {
        let current = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or_default();
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta as usize)
                .min(Self::ALL.len() - 1)
        };
        Self::ALL[next]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QemuLaunchDialog {
    pub draft: QemuLaunchDraft,
    pub selected_field: QemuLaunchField,
    pub editing: bool,
    pub validation_error: Option<String>,
}

impl QemuLaunchDialog {
    pub fn new(draft: QemuLaunchDraft) -> Self {
        Self {
            draft,
            selected_field: QemuLaunchField::Machine,
            editing: false,
            validation_error: None,
        }
    }

    pub fn selected_text_mut(&mut self) -> Option<(&mut String, usize)> {
        match self.selected_field {
            QemuLaunchField::Kernel => Some((&mut self.draft.kernel, MAX_QEMU_PATH_INPUT_BYTES)),
            QemuLaunchField::Rootfs => Some((&mut self.draft.rootfs, MAX_QEMU_PATH_INPUT_BYTES)),
            QemuLaunchField::Memory => Some((&mut self.draft.memory_mib, 16)),
            QemuLaunchField::ExtraArguments => Some((
                &mut self.draft.extra_arguments,
                MAX_QEMU_EXTRA_ARGUMENT_INPUT_BYTES,
            )),
            QemuLaunchField::Machine
            | QemuLaunchField::Image
            | QemuLaunchField::Networking
            | QemuLaunchField::Display
            | QemuLaunchField::Serial => None,
        }
    }

    pub fn cycle_choice(&mut self, backwards: bool) -> bool {
        match self.selected_field {
            QemuLaunchField::Networking => {
                self.draft.networking = match (self.draft.networking, backwards) {
                    (QemuNetworkingMode::Slirp, false) => QemuNetworkingMode::Tap,
                    (QemuNetworkingMode::Tap, false) => QemuNetworkingMode::None,
                    (QemuNetworkingMode::None, false) => QemuNetworkingMode::Slirp,
                    (QemuNetworkingMode::Slirp, true) => QemuNetworkingMode::None,
                    (QemuNetworkingMode::Tap, true) => QemuNetworkingMode::Slirp,
                    (QemuNetworkingMode::None, true) => QemuNetworkingMode::Tap,
                };
                true
            }
            QemuLaunchField::Display => {
                self.draft.display = match self.draft.display {
                    QemuDisplayMode::Graphical => QemuDisplayMode::Nographic,
                    QemuDisplayMode::Nographic => QemuDisplayMode::Graphical,
                };
                true
            }
            QemuLaunchField::Serial => {
                self.draft.serial = match (self.draft.serial, backwards) {
                    (QemuSerialMode::Stdio, false) => QemuSerialMode::Telnet,
                    (QemuSerialMode::Telnet, false) => QemuSerialMode::None,
                    (QemuSerialMode::None, false) => QemuSerialMode::Stdio,
                    (QemuSerialMode::Stdio, true) => QemuSerialMode::None,
                    (QemuSerialMode::Telnet, true) => QemuSerialMode::Stdio,
                    (QemuSerialMode::None, true) => QemuSerialMode::Telnet,
                };
                true
            }
            QemuLaunchField::Machine
            | QemuLaunchField::Image
            | QemuLaunchField::Kernel
            | QemuLaunchField::Rootfs
            | QemuLaunchField::Memory
            | QemuLaunchField::ExtraArguments => false,
        }
    }
}

impl QemuLaunchDraft {
    pub fn for_artifact(image: ImageArtifactIdentity, artifact_kind: ImageArtifactKind) -> Self {
        Self {
            machine: image.machine.clone(),
            image,
            artifact_kind,
            kernel: String::new(),
            rootfs: String::new(),
            networking: QemuNetworkingMode::Slirp,
            display: QemuDisplayMode::Graphical,
            serial: QemuSerialMode::Stdio,
            memory_mib: "1024".into(),
            extra_arguments: String::new(),
        }
    }

    pub fn preview(&self, capability: &QemuCapability) -> Result<QemuLaunchPreview, &'static str> {
        let executable = capability.executable_for(&self.image)?.to_path_buf();
        if self.extra_arguments.contains(['\'', '"', '\\'])
            || self.extra_arguments.chars().any(char::is_control)
        {
            return Err("runqemu extra arguments do not accept quoting or escaping");
        }
        let request = QemuLaunchRequest {
            machine: self.machine.clone(),
            image: self.image.clone(),
            artifact_kind: self.artifact_kind,
            kernel: optional_path(&self.kernel)?,
            rootfs: optional_path(&self.rootfs)?,
            networking: self.networking,
            display: self.display,
            serial: self.serial,
            memory_mib: self
                .memory_mib
                .parse()
                .map_err(|_| "runqemu memory must be a whole number of MiB")?,
            extra_arguments: self
                .extra_arguments
                .split_ascii_whitespace()
                .map(str::to_owned)
                .collect(),
        };
        request.validate()?;
        let mut argv = vec![
            executable,
            PathBuf::from(&request.machine),
            request.image.path.clone(),
            PathBuf::from(format!("qemumemory={}", request.memory_mib)),
            PathBuf::from(match request.networking {
                QemuNetworkingMode::Slirp => "slirp",
                QemuNetworkingMode::Tap => "tap",
                QemuNetworkingMode::None => "nonetwork",
            }),
            PathBuf::from(match request.display {
                QemuDisplayMode::Graphical => "sdl",
                QemuDisplayMode::Nographic => "nographic",
            }),
        ];
        if let Some(kernel) = &request.kernel {
            argv.push(kernel.clone());
        }
        if let Some(rootfs) = &request.rootfs {
            argv.push(rootfs.clone());
        }
        match request.serial {
            QemuSerialMode::Stdio => argv.push("serialstdio".into()),
            QemuSerialMode::Telnet => argv.push("serialtelnet".into()),
            QemuSerialMode::None => {}
        }
        argv.extend(request.extra_arguments.iter().map(PathBuf::from));
        Ok(QemuLaunchPreview { request, argv })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QemuLaunchPreview {
    pub request: QemuLaunchRequest,
    pub argv: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QemuSessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QemuSession {
    pub id: QemuSessionId,
    pub background_job_id: crate::BackgroundJobId,
    pub request: QemuLaunchRequest,
    pub exit_code: Option<i32>,
    pub error_detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QemuOutputStream {
    Stdout,
    Stderr,
}

fn optional_path(input: &str) -> Result<Option<PathBuf>, &'static str> {
    if input.is_empty() {
        return Ok(None);
    }
    if input.trim() != input || input.chars().any(char::is_control) {
        return Err("runqemu paths must not contain surrounding space or control characters");
    }
    let path = PathBuf::from(input);
    if !absolute_normal_path(&path) {
        return Err("runqemu paths must be normalized absolute paths");
    }
    Ok(Some(path))
}

fn absolute_normal_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.components().all(|component| {
            !matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
}

fn extra_argument_is_valid(argument: &str) -> bool {
    !argument.is_empty()
        && argument.len() <= MAX_QEMU_EXTRA_ARGUMENT_BYTES
        && !argument.starts_with('-')
        && !argument.chars().any(|character| {
            character.is_control()
                || character.is_whitespace()
                || matches!(
                    character,
                    ';' | '&' | '|' | '<' | '>' | '`' | '$' | '(' | ')' | '{' | '}'
                )
        })
}
