use std::path::{Component, Path, PathBuf};

use crate::{
    ImageArtifactIdentity, ImageArtifactKind, QemuCapability, QemuDisplayMode, QemuLaunchDraft,
    QemuNetworkingMode, QemuSerialMode, TerminalCreationKind,
};

pub const MAX_IMAGE_CONSOLE_HOST_BYTES: usize = 253;
pub const MAX_IMAGE_CONSOLE_USER_BYTES: usize = 64;
pub const MAX_IMAGE_CONSOLE_IDENTITY_BYTES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SshClientCapability {
    #[default]
    NotInspected,
    Available {
        executable: PathBuf,
    },
    Missing,
    Failed {
        message: String,
    },
}

impl SshClientCapability {
    pub fn executable(&self) -> Result<&Path, &'static str> {
        match self {
            Self::Available { executable } if absolute_normal_path(executable) => Ok(executable),
            Self::Available { .. } => Err("the inspected SSH executable identity is invalid"),
            Self::NotInspected => Err("SSH client capability has not been inspected"),
            Self::Missing => Err("OpenSSH client is not available"),
            Self::Failed { .. } => Err("SSH client capability inspection failed"),
        }
    }

    pub fn status_text(&self) -> String {
        match self {
            Self::NotInspected => "not inspected".into(),
            Self::Available { executable } => format!("available: {}", executable.display()),
            Self::Missing => "unavailable: ssh was not found".into(),
            Self::Failed { message } => format!("failed: {message}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageConsoleMode {
    #[default]
    Qemu,
    Ssh,
}

impl ImageConsoleMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Qemu => "Boot with QEMU",
            Self::Ssh => "Connect over SSH",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageConsoleField {
    Mode,
    Image,
    Networking,
    Memory,
    Host,
    User,
    Port,
    IdentityFile,
}

impl ImageConsoleField {
    pub fn is_read_only(self) -> bool {
        self == Self::Image
    }

    pub fn is_text(self) -> bool {
        matches!(
            self,
            Self::Memory | Self::Host | Self::User | Self::Port | Self::IdentityFile
        )
    }
}

const QEMU_FIELDS: [ImageConsoleField; 4] = [
    ImageConsoleField::Mode,
    ImageConsoleField::Image,
    ImageConsoleField::Networking,
    ImageConsoleField::Memory,
];
const SSH_FIELDS: [ImageConsoleField; 6] = [
    ImageConsoleField::Mode,
    ImageConsoleField::Image,
    ImageConsoleField::Host,
    ImageConsoleField::User,
    ImageConsoleField::Port,
    ImageConsoleField::IdentityFile,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageConsoleDraft {
    pub mode: ImageConsoleMode,
    pub image: ImageArtifactIdentity,
    pub artifact_kind: ImageArtifactKind,
    pub networking: QemuNetworkingMode,
    pub memory_mib: String,
    pub host: String,
    pub user: String,
    pub port: String,
    pub identity_file: String,
}

impl ImageConsoleDraft {
    pub fn for_artifact(image: ImageArtifactIdentity, artifact_kind: ImageArtifactKind) -> Self {
        Self {
            mode: ImageConsoleMode::Qemu,
            image,
            artifact_kind,
            networking: QemuNetworkingMode::Slirp,
            memory_mib: "1024".into(),
            host: String::new(),
            user: "root".into(),
            port: "22".into(),
            identity_file: String::new(),
        }
    }

    pub fn preview(
        &self,
        qemu: &QemuCapability,
        ssh: &SshClientCapability,
    ) -> Result<ImageConsolePreview, String> {
        match self.mode {
            ImageConsoleMode::Qemu => self.qemu_preview(qemu),
            ImageConsoleMode::Ssh => self.ssh_preview(ssh),
        }
    }

    fn qemu_preview(&self, capability: &QemuCapability) -> Result<ImageConsolePreview, String> {
        if !matches!(
            self.artifact_kind,
            ImageArtifactKind::RootFilesystem | ImageArtifactKind::Wic
        ) {
            return Err("QEMU console requires a selected root filesystem or Wic artifact".into());
        }
        let mut draft = QemuLaunchDraft::for_artifact(self.image.clone(), self.artifact_kind);
        draft.networking = self.networking;
        draft.memory_mib.clone_from(&self.memory_mib);
        draft.display = QemuDisplayMode::Nographic;
        draft.serial = QemuSerialMode::Stdio;
        let preview = draft.preview(capability).map_err(str::to_owned)?;
        let program = preview
            .argv
            .first()
            .cloned()
            .ok_or_else(|| "runqemu preview has no executable".to_owned())?;
        let arguments = preview
            .argv
            .iter()
            .skip(1)
            .map(|argument| {
                argument
                    .to_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "runqemu arguments must be valid UTF-8".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ImageConsolePreview {
            image: self.image.clone(),
            mode: self.mode,
            name: bounded_terminal_name("QEMU", &self.image.image),
            kind: TerminalCreationKind::QemuConsole,
            program,
            arguments,
            destination: format!(
                "{} · {} MiB · {:?}",
                self.image.machine, preview.request.memory_mib, preview.request.networking
            ),
        })
    }

    fn ssh_preview(&self, capability: &SshClientCapability) -> Result<ImageConsolePreview, String> {
        let executable = capability
            .executable()
            .map_err(|message| match capability {
                SshClientCapability::Failed { message } => {
                    format!("SSH client capability inspection failed: {message}")
                }
                _ => message.to_owned(),
            })?;
        validate_host(&self.host)?;
        validate_user(&self.user)?;
        let port = self
            .port
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or_else(|| "SSH port must be a whole number from 1 through 65535".to_owned())?;
        let identity = optional_identity_path(&self.identity_file)?;
        let mut arguments = vec!["-t".into(), "-p".into(), port.to_string()];
        if let Some(identity) = identity {
            arguments.push("-i".into());
            arguments.push(
                identity
                    .to_str()
                    .ok_or_else(|| "SSH identity path must be valid UTF-8".to_owned())?
                    .into(),
            );
        }
        let target = format!("{}@{}", self.user, self.host);
        arguments.push(target.clone());
        Ok(ImageConsolePreview {
            image: self.image.clone(),
            mode: self.mode,
            name: bounded_terminal_name("SSH", &target),
            kind: TerminalCreationKind::SshConsole,
            program: executable.to_path_buf(),
            arguments,
            destination: format!("{target}:{port}"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageConsoleDialog {
    pub draft: ImageConsoleDraft,
    pub selected_field: ImageConsoleField,
    pub editing: bool,
    pub validation_error: Option<String>,
}

impl ImageConsoleDialog {
    pub fn new(draft: ImageConsoleDraft) -> Self {
        Self {
            draft,
            selected_field: ImageConsoleField::Mode,
            editing: false,
            validation_error: None,
        }
    }

    pub fn fields(&self) -> &'static [ImageConsoleField] {
        match self.draft.mode {
            ImageConsoleMode::Qemu => &QEMU_FIELDS,
            ImageConsoleMode::Ssh => &SSH_FIELDS,
        }
    }

    pub fn shift_field(&mut self, delta: isize) {
        let fields = self.fields();
        let current = fields
            .iter()
            .position(|field| *field == self.selected_field)
            .unwrap_or_default();
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current.saturating_add(delta as usize).min(fields.len() - 1)
        };
        self.selected_field = fields[next];
    }

    pub fn cycle_choice(&mut self, backwards: bool) -> bool {
        match self.selected_field {
            ImageConsoleField::Mode => {
                self.draft.mode = match self.draft.mode {
                    ImageConsoleMode::Qemu => ImageConsoleMode::Ssh,
                    ImageConsoleMode::Ssh => ImageConsoleMode::Qemu,
                };
                self.selected_field = ImageConsoleField::Mode;
                true
            }
            ImageConsoleField::Networking => {
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
            _ => false,
        }
    }

    pub fn selected_text_mut(&mut self) -> Option<(&mut String, usize)> {
        match self.selected_field {
            ImageConsoleField::Memory => Some((&mut self.draft.memory_mib, 16)),
            ImageConsoleField::Host => Some((&mut self.draft.host, MAX_IMAGE_CONSOLE_HOST_BYTES)),
            ImageConsoleField::User => Some((&mut self.draft.user, MAX_IMAGE_CONSOLE_USER_BYTES)),
            ImageConsoleField::Port => Some((&mut self.draft.port, 5)),
            ImageConsoleField::IdentityFile => Some((
                &mut self.draft.identity_file,
                MAX_IMAGE_CONSOLE_IDENTITY_BYTES,
            )),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageConsolePreview {
    pub image: ImageArtifactIdentity,
    pub mode: ImageConsoleMode,
    pub name: String,
    pub kind: TerminalCreationKind,
    pub program: PathBuf,
    pub arguments: Vec<String>,
    pub destination: String,
}

fn validate_host(host: &str) -> Result<(), String> {
    if host.is_empty()
        || host.len() > MAX_IMAGE_CONSOLE_HOST_BYTES
        || host.starts_with('-')
        || host.chars().any(|character| {
            !character.is_ascii_alphanumeric()
                && !matches!(character, '.' | '-' | '_' | ':' | '%' | '[' | ']')
        })
    {
        return Err("SSH host must be an explicit bounded IPv4, IPv6, or DNS name".into());
    }
    Ok(())
}

fn validate_user(user: &str) -> Result<(), String> {
    if user.is_empty()
        || user.len() > MAX_IMAGE_CONSOLE_USER_BYTES
        || user.starts_with('-')
        || user.chars().any(|character| {
            !character.is_ascii_alphanumeric() && !matches!(character, '_' | '-' | '.')
        })
    {
        return Err("SSH user must be an explicit bounded account name".into());
    }
    Ok(())
}

fn optional_identity_path(input: &str) -> Result<Option<PathBuf>, String> {
    if input.is_empty() {
        return Ok(None);
    }
    if input.trim() != input || input.len() > MAX_IMAGE_CONSOLE_IDENTITY_BYTES {
        return Err("SSH identity path is oversized or contains surrounding space".into());
    }
    let path = PathBuf::from(input);
    if !absolute_normal_path(&path) {
        return Err("SSH identity path must be a normalized absolute path".into());
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

fn bounded_terminal_name(prefix: &str, identity: &str) -> String {
    let mut output = format!("{prefix} ");
    for character in identity.chars() {
        if output.len().saturating_add(character.len_utf8()) > 128 {
            break;
        }
        if !character.is_control() {
            output.push(character);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> ImageArtifactIdentity {
        ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: "/deploy/core-image-minimal-qemux86-64.rootfs.ext4".into(),
        }
    }

    #[test]
    fn image_console_qemu_enforces_serial_stdio_and_nographic() {
        let image = image();
        let draft =
            ImageConsoleDraft::for_artifact(image.clone(), ImageArtifactKind::RootFilesystem);
        let preview = draft
            .preview(
                &QemuCapability::Available {
                    executable: "/opt/poky/runqemu".into(),
                    compatible_images: vec![image],
                },
                &SshClientCapability::Missing,
            )
            .unwrap();
        assert_eq!(preview.kind, TerminalCreationKind::QemuConsole);
        assert!(
            preview
                .arguments
                .iter()
                .any(|argument| argument == "nographic")
        );
        assert!(
            preview
                .arguments
                .iter()
                .any(|argument| argument == "serialstdio")
        );
    }

    #[test]
    fn image_console_ssh_is_argv_only_and_keeps_host_key_defaults() {
        let mut draft = ImageConsoleDraft::for_artifact(image(), ImageArtifactKind::RootFilesystem);
        draft.mode = ImageConsoleMode::Ssh;
        draft.host = "192.0.2.44".into();
        draft.user = "root".into();
        draft.port = "2222".into();
        draft.identity_file = "/home/user/.ssh/id_ed25519".into();
        let preview = draft
            .preview(
                &QemuCapability::MissingTool,
                &SshClientCapability::Available {
                    executable: "/usr/bin/ssh".into(),
                },
            )
            .unwrap();
        assert_eq!(preview.kind, TerminalCreationKind::SshConsole);
        assert_eq!(preview.program, PathBuf::from("/usr/bin/ssh"));
        assert_eq!(
            preview.arguments,
            [
                "-t",
                "-p",
                "2222",
                "-i",
                "/home/user/.ssh/id_ed25519",
                "root@192.0.2.44"
            ]
        );
        assert!(!preview.arguments.iter().any(|argument| {
            argument.contains("StrictHostKeyChecking") || argument.contains("UserKnownHostsFile")
        }));
    }

    #[test]
    fn image_console_rejects_ambiguous_ssh_inputs_and_bounds_fields() {
        let mut dialog = ImageConsoleDialog::new(ImageConsoleDraft::for_artifact(
            image(),
            ImageArtifactKind::RootFilesystem,
        ));
        assert!(dialog.cycle_choice(false));
        assert_eq!(dialog.draft.mode, ImageConsoleMode::Ssh);
        dialog.draft.host = "-oProxyCommand=bad".into();
        assert!(
            dialog
                .draft
                .preview(
                    &QemuCapability::MissingTool,
                    &SshClientCapability::Available {
                        executable: "/usr/bin/ssh".into()
                    }
                )
                .unwrap_err()
                .contains("SSH host")
        );
        dialog.draft.host = "target.example".into();
        dialog.draft.port = "0".into();
        assert!(
            dialog
                .draft
                .preview(
                    &QemuCapability::MissingTool,
                    &SshClientCapability::Available {
                        executable: "/usr/bin/ssh".into()
                    }
                )
                .unwrap_err()
                .contains("SSH port")
        );
    }
}
