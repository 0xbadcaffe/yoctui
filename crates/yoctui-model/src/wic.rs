use std::path::{Component, Path, PathBuf};

pub const MAX_WIC_KICKSTARTS: usize = 256;
pub const MAX_WIC_PARTITIONS: usize = 128;
pub const MAX_WIC_OUTPUTS: usize = 256;
pub const MAX_WIC_DEVICES: usize = 128;
pub const MAX_WIC_DEVICE_MOUNTS: usize = 64;
pub const MAX_WIC_LIMITATIONS: usize = 64;
pub const MAX_WIC_SOURCE_BYTES: usize = 64 * 1024;
pub const MAX_WIC_OUTPUT_DIRECTORY_INPUT_BYTES: usize = 4_096;
pub const MAX_WIC_WRITE_PHRASE_INPUT_BYTES: usize = 4_102;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WicKickstartIdentity {
    pub name: String,
    pub path: Option<PathBuf>,
}

impl WicKickstartIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !safe_name(&self.name) {
            return Err("Wic kickstart names must be bounded plain tokens");
        }
        if self.path.as_ref().is_some_and(|path| {
            let name = path.file_name().and_then(|name| name.to_str());
            !absolute_normal_path(path)
                || !name.is_some_and(|name| name.ends_with(".wks") || name.ends_with(".wks.in"))
        }) {
            return Err("Wic kickstart paths must be normalized absolute .wks or .wks.in paths");
        }
        Ok(())
    }

    pub fn argument(&self) -> PathBuf {
        self.path
            .clone()
            .unwrap_or_else(|| PathBuf::from(&self.name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicPartitionSummary {
    pub mount_point: Option<String>,
    pub filesystem: Option<String>,
    pub source_plugin: Option<String>,
    pub size_mib: Option<u64>,
    pub alignment_kib: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicKickstart {
    pub identity: WicKickstartIdentity,
    pub source: String,
    pub partitions: Vec<WicPartitionSummary>,
    pub limitations: Vec<String>,
}

impl WicKickstart {
    pub fn normalize(mut self) -> Result<Self, &'static str> {
        self.identity.validate()?;
        if self.source.len() > MAX_WIC_SOURCE_BYTES || self.source.chars().any(|ch| ch == '\0') {
            return Err("Wic kickstart source exceeds its safety bound or contains NUL");
        }
        self.partitions.truncate(MAX_WIC_PARTITIONS);
        for partition in &mut self.partitions {
            for value in [
                &mut partition.mount_point,
                &mut partition.filesystem,
                &mut partition.source_plugin,
            ] {
                *value = value
                    .take()
                    .filter(|value| !value.chars().any(char::is_control))
                    .map(|value| value.chars().take(1_024).collect());
            }
        }
        self.limitations = normalize_messages(self.limitations);
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WicCapability {
    #[default]
    NotInspected,
    Available {
        executable: PathBuf,
        kickstarts: Vec<WicKickstart>,
        image_targets: Vec<String>,
    },
    MissingTool,
    MissingKickstarts {
        executable: PathBuf,
    },
    Failed {
        message: String,
    },
}

pub fn normalize_wic_capability(capability: WicCapability) -> WicCapability {
    let WicCapability::Available {
        executable,
        kickstarts,
        image_targets,
    } = capability
    else {
        return capability;
    };
    if !absolute_normal_path(&executable) {
        return WicCapability::Failed {
            message: "Wic inspection returned an invalid executable path".into(),
        };
    }
    let mut kickstarts: Vec<_> = kickstarts
        .into_iter()
        .filter_map(|kickstart| kickstart.normalize().ok())
        .collect();
    kickstarts.sort_by(|left, right| left.identity.name.cmp(&right.identity.name));
    kickstarts.dedup_by(|left, right| left.identity == right.identity);
    kickstarts.truncate(MAX_WIC_KICKSTARTS);
    if kickstarts.is_empty() {
        return WicCapability::MissingKickstarts { executable };
    }
    let mut image_targets: Vec<_> = image_targets
        .into_iter()
        .filter(|target| safe_name(target))
        .collect();
    image_targets.sort();
    image_targets.dedup();
    image_targets.truncate(MAX_WIC_KICKSTARTS);
    WicCapability::Available {
        executable,
        kickstarts,
        image_targets,
    }
}

impl WicCapability {
    pub fn resolve(
        &self,
        kickstart: &WicKickstartIdentity,
        image: &str,
    ) -> Result<(&Path, &WicKickstart), &'static str> {
        let Self::Available {
            executable,
            kickstarts,
            image_targets,
        } = self
        else {
            return Err(match self {
                Self::NotInspected => "Wic capability has not been inspected",
                Self::MissingTool => "wic is not available",
                Self::MissingKickstarts { .. } => "no Wic kickstarts are available",
                Self::Failed { .. } => "Wic capability inspection failed",
                Self::Available { .. } => unreachable!(),
            });
        };
        if !absolute_normal_path(executable) {
            return Err("Wic executable paths must be normalized and absolute");
        }
        if !safe_name(image) || !image_targets.iter().any(|candidate| candidate == image) {
            return Err("the image is not in the inspected Wic capability");
        }
        let candidate = kickstarts
            .iter()
            .find(|candidate| candidate.identity == *kickstart)
            .ok_or("the kickstart is not in the inspected Wic capability")?;
        Ok((executable, candidate))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WicCompression {
    None,
    Gzip,
    Bzip2,
    Xz,
}

impl WicCompression {
    pub fn argument(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Gzip => Some("gzip"),
            Self::Bzip2 => Some("bzip2"),
            Self::Xz => Some("xz"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicCreateRequest {
    pub machine: String,
    pub image: String,
    pub kickstart: WicKickstartIdentity,
    pub output_directory: PathBuf,
    pub generate_bmap: bool,
    pub compression: WicCompression,
}

impl WicCreateRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !safe_name(&self.machine) || !safe_name(&self.image) {
            return Err("Wic machine and image identities must be bounded plain tokens");
        }
        self.kickstart.validate()?;
        if !absolute_normal_path(&self.output_directory) {
            return Err("Wic output directories must be normalized absolute paths");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicCreateDraft {
    pub machine: String,
    pub image: String,
    pub kickstart: WicKickstartIdentity,
    pub output_directory: String,
    pub generate_bmap: bool,
    pub compression: WicCompression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WicCreateField {
    Machine,
    Image,
    Kickstart,
    OutputDirectory,
    GenerateBmap,
    Compression,
}

impl WicCreateField {
    const ALL: [Self; 6] = [
        Self::Machine,
        Self::Image,
        Self::Kickstart,
        Self::OutputDirectory,
        Self::GenerateBmap,
        Self::Compression,
    ];

    pub fn shifted(self, delta: isize) -> Self {
        let current = Self::ALL
            .iter()
            .position(|value| *value == self)
            .unwrap_or(0);
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta as usize)
                .min(Self::ALL.len() - 1)
        };
        Self::ALL[next]
    }

    pub fn is_read_only(self) -> bool {
        self == Self::Machine
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicCreateDialog {
    pub draft: WicCreateDraft,
    pub selected_field: WicCreateField,
    pub editing: bool,
    pub validation_error: Option<String>,
}

impl WicCreateDialog {
    pub fn new(draft: WicCreateDraft) -> Self {
        Self {
            draft,
            selected_field: WicCreateField::Machine,
            editing: false,
            validation_error: None,
        }
    }

    pub fn selected_text_mut(&mut self) -> Option<(&mut String, usize)> {
        (self.selected_field == WicCreateField::OutputDirectory).then_some((
            &mut self.draft.output_directory,
            MAX_WIC_OUTPUT_DIRECTORY_INPUT_BYTES,
        ))
    }

    pub fn cycle_choice(&mut self, capability: &WicCapability, backwards: bool) -> bool {
        let WicCapability::Available {
            kickstarts,
            image_targets,
            ..
        } = capability
        else {
            return false;
        };
        match self.selected_field {
            WicCreateField::Image if !image_targets.is_empty() => {
                cycle_value(&mut self.draft.image, image_targets, backwards);
                true
            }
            WicCreateField::Kickstart if !kickstarts.is_empty() => {
                let values: Vec<_> = kickstarts
                    .iter()
                    .map(|kickstart| kickstart.identity.clone())
                    .collect();
                cycle_value(&mut self.draft.kickstart, &values, backwards);
                true
            }
            WicCreateField::GenerateBmap => {
                self.draft.generate_bmap = !self.draft.generate_bmap;
                true
            }
            WicCreateField::Compression => {
                self.draft.compression = match (self.draft.compression, backwards) {
                    (WicCompression::None, false) => WicCompression::Gzip,
                    (WicCompression::Gzip, false) => WicCompression::Bzip2,
                    (WicCompression::Bzip2, false) => WicCompression::Xz,
                    (WicCompression::Xz, false) => WicCompression::None,
                    (WicCompression::None, true) => WicCompression::Xz,
                    (WicCompression::Gzip, true) => WicCompression::None,
                    (WicCompression::Bzip2, true) => WicCompression::Gzip,
                    (WicCompression::Xz, true) => WicCompression::Bzip2,
                };
                true
            }
            _ => false,
        }
    }
}

fn cycle_value<T: Clone + PartialEq>(current: &mut T, values: &[T], backwards: bool) {
    let index = values
        .iter()
        .position(|value| value == current)
        .unwrap_or(0);
    let next = if backwards {
        index.checked_sub(1).unwrap_or(values.len() - 1)
    } else {
        (index + 1) % values.len()
    };
    *current = values[next].clone();
}

impl WicCreateDraft {
    pub fn preview(&self, capability: &WicCapability) -> Result<WicCreatePreview, &'static str> {
        if self.output_directory.trim() != self.output_directory
            || self.output_directory.chars().any(char::is_control)
        {
            return Err("Wic output directories must not contain surrounding space or controls");
        }
        let request = WicCreateRequest {
            machine: self.machine.clone(),
            image: self.image.clone(),
            kickstart: self.kickstart.clone(),
            output_directory: PathBuf::from(&self.output_directory),
            generate_bmap: self.generate_bmap,
            compression: self.compression,
        };
        request.validate()?;
        let (executable, kickstart) = capability.resolve(&request.kickstart, &request.image)?;
        let mut argv = vec![
            executable.to_path_buf(),
            "create".into(),
            request.kickstart.argument(),
            "-e".into(),
            request.image.clone().into(),
            "-o".into(),
            request.output_directory.clone(),
        ];
        if request.generate_bmap {
            argv.push("--bmap".into());
        }
        if let Some(compression) = request.compression.argument() {
            argv.extend(["--compress-with".into(), compression.into()]);
        }
        Ok(WicCreatePreview {
            request,
            kickstart: kickstart.clone(),
            argv,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicCreatePreview {
    pub request: WicCreateRequest,
    pub kickstart: WicKickstart,
    pub argv: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WicOutputKind {
    Wic,
    Direct,
    Bmap,
    Compressed,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WicOutputIdentity {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_unix_seconds: u64,
}

impl WicOutputIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !absolute_normal_path(&self.path) {
            return Err("Wic output paths must be normalized and absolute");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicOutput {
    pub identity: WicOutputIdentity,
    pub kind: WicOutputKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicOutputInventoryRequest {
    pub generation: u64,
    pub output_directory: PathBuf,
}

impl WicOutputInventoryRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.generation == 0 || !absolute_normal_path(&self.output_directory) {
            return Err("Wic output requests require a generation and normalized absolute root");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WicOutputInventoryState {
    #[default]
    NotLoaded,
    Loading {
        request: WicOutputInventoryRequest,
    },
    Available {
        request: WicOutputInventoryRequest,
        outputs: Vec<WicOutput>,
    },
    Partial {
        request: WicOutputInventoryRequest,
        outputs: Vec<WicOutput>,
        limitations: Vec<String>,
    },
    Failed {
        request: WicOutputInventoryRequest,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WicDeviceIdentity {
    pub path: PathBuf,
    pub major_minor: String,
    pub size_bytes: u64,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub transport: Option<String>,
}

impl WicDeviceIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !absolute_normal_path(&self.path)
            || !self.path.starts_with("/dev")
            || self.path.as_os_str().len() > 4_096
            || self.major_minor.is_empty()
            || self.major_minor.len() > 32
            || !self
                .major_minor
                .chars()
                .all(|character| character.is_ascii_digit() || character == ':')
        {
            return Err(
                "Wic device identities must use an exact normalized /dev path and major:minor",
            );
        }
        let valid_major_minor = self
            .major_minor
            .split_once(':')
            .is_some_and(|(major, minor)| {
                !major.is_empty()
                    && !minor.is_empty()
                    && major.chars().all(|character| character.is_ascii_digit())
                    && minor.chars().all(|character| character.is_ascii_digit())
            });
        if !valid_major_minor
            || self
                .model
                .iter()
                .chain(self.serial.iter())
                .chain(self.transport.iter())
                .any(|value| value.len() > 256 || value.chars().any(char::is_control))
        {
            return Err("Wic device metadata is malformed or exceeds its safety bound");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicDevice {
    pub identity: WicDeviceIdentity,
    pub removable: bool,
    pub writable: bool,
    pub read_only: bool,
    pub descendant_mounts: Vec<PathBuf>,
    pub unavailable_reason: Option<String>,
}

impl WicDevice {
    pub fn eligible_for(&self, image: &WicOutputIdentity) -> Result<(), &'static str> {
        self.identity.validate()?;
        image.validate()?;
        if !self.removable {
            return Err("Wic writes require a removable whole device");
        }
        if !self.writable || self.read_only {
            return Err("the Wic write device is not writable");
        }
        if !self.descendant_mounts.is_empty() {
            return Err("the Wic write device has mounted descendants");
        }
        if self.identity.size_bytes < image.size_bytes {
            return Err("the Wic write device is smaller than the image");
        }
        if self.unavailable_reason.is_some() {
            return Err("the Wic write device is excluded by safety inspection");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicDeviceInventoryRequest {
    pub generation: u64,
    pub image: WicOutputIdentity,
}

impl WicDeviceInventoryRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.generation == 0 {
            return Err("Wic device requests require a non-zero generation");
        }
        self.image.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WicDeviceInventoryState {
    #[default]
    NotLoaded,
    Loading {
        request: WicDeviceInventoryRequest,
    },
    Available {
        request: WicDeviceInventoryRequest,
        devices: Vec<WicDevice>,
    },
    Partial {
        request: WicDeviceInventoryRequest,
        devices: Vec<WicDevice>,
        limitations: Vec<String>,
    },
    Failed {
        request: WicDeviceInventoryRequest,
        message: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicWriteRequest {
    pub executable: PathBuf,
    pub image: WicOutputIdentity,
    pub device: WicDeviceIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicWritePreview {
    pub request: WicWriteRequest,
    pub argv: Vec<PathBuf>,
}

impl WicWritePreview {
    pub fn new(
        capability: &WicCapability,
        image: WicOutputIdentity,
        device: &WicDevice,
        phrase: &str,
    ) -> Result<Self, &'static str> {
        device.eligible_for(&image)?;
        let executable = match capability {
            WicCapability::Available { executable, .. } if absolute_normal_path(executable) => {
                executable.clone()
            }
            WicCapability::MissingKickstarts { executable } if absolute_normal_path(executable) => {
                executable.clone()
            }
            _ => return Err("Wic capability is unavailable for device writing"),
        };
        let expected = format!("WRITE {}", device.identity.path.display());
        if phrase != expected {
            return Err("Wic device confirmation phrase does not exactly match");
        }
        let request = WicWriteRequest {
            executable: executable.clone(),
            image,
            device: device.identity.clone(),
        };
        let argv = vec![
            executable,
            "write".into(),
            request.image.path.clone(),
            request.device.path.clone(),
        ];
        Ok(Self { request, argv })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicDevicePickerDialog {
    pub request: WicDeviceInventoryRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicWritePhraseDialog {
    pub request: WicDeviceInventoryRequest,
    pub device: WicDeviceIdentity,
    pub input: String,
    pub validation_error: Option<String>,
}

impl WicWritePhraseDialog {
    pub fn append(&mut self, character: char) {
        if !character.is_control()
            && self.input.len().saturating_add(character.len_utf8())
                <= MAX_WIC_WRITE_PHRASE_INPUT_BYTES
        {
            self.input.push(character);
            self.validation_error = None;
        }
    }

    pub fn backspace(&mut self) {
        self.input.pop();
        self.validation_error = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WicSessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WicOperation {
    Create(WicCreateRequest),
    Write(WicWriteRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicSession {
    pub id: WicSessionId,
    pub background_job_id: crate::BackgroundJobId,
    pub operation: WicOperation,
    pub exit_code: Option<i32>,
    pub error_detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WicOutputStream {
    Stdout,
    Stderr,
}

pub fn normalize_wic_outputs(
    output_directory: &Path,
    mut outputs: Vec<WicOutput>,
) -> Result<Vec<WicOutput>, &'static str> {
    if !absolute_normal_path(output_directory) {
        return Err("Wic output inventory roots must be normalized and absolute");
    }
    outputs.retain(|output| {
        output.identity.validate().is_ok() && output.identity.path.starts_with(output_directory)
    });
    outputs.sort_by(|left, right| left.identity.path.cmp(&right.identity.path));
    outputs.dedup_by(|left, right| left.identity == right.identity);
    outputs.truncate(MAX_WIC_OUTPUTS);
    Ok(outputs)
}

pub fn normalize_wic_devices(mut devices: Vec<WicDevice>) -> Vec<WicDevice> {
    devices.retain(|device| device.identity.validate().is_ok());
    for device in &mut devices {
        device
            .descendant_mounts
            .retain(|mount| absolute_normal_path(mount));
        device.descendant_mounts.sort();
        device.descendant_mounts.dedup();
        device.descendant_mounts.truncate(MAX_WIC_DEVICE_MOUNTS);
        device.unavailable_reason = device
            .unavailable_reason
            .take()
            .filter(|reason| !reason.is_empty() && !reason.chars().any(char::is_control))
            .map(|reason| reason.chars().take(2_048).collect());
    }
    devices.sort_by(|left, right| left.identity.path.cmp(&right.identity.path));
    devices.dedup_by(|left, right| left.identity == right.identity);
    devices.truncate(MAX_WIC_DEVICES);
    devices
}

pub fn normalize_wic_limitations(limitations: Vec<String>) -> Vec<String> {
    normalize_messages(limitations)
}

fn normalize_messages(messages: Vec<String>) -> Vec<String> {
    let mut messages: Vec<_> = messages
        .into_iter()
        .filter(|message| !message.is_empty() && !message.chars().any(char::is_control))
        .map(|message| message.chars().take(2_048).collect())
        .collect();
    messages.sort();
    messages.dedup();
    messages.truncate(MAX_WIC_LIMITATIONS);
    messages
}

fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
        })
}

pub(crate) fn absolute_normal_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.components().all(|component| {
            !matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kickstart() -> WicKickstart {
        WicKickstart {
            identity: WicKickstartIdentity {
                name: "directdisk".into(),
                path: Some("/layers/meta/wic/directdisk.wks".into()),
            },
            source: "part / --source rootfs --fstype=ext4 --size=64".into(),
            partitions: vec![WicPartitionSummary {
                mount_point: Some("/".into()),
                filesystem: Some("ext4".into()),
                source_plugin: Some("rootfs".into()),
                size_mib: Some(64),
                alignment_kib: None,
            }],
            limitations: Vec::new(),
        }
    }

    fn capability() -> WicCapability {
        WicCapability::Available {
            executable: "/opt/poky/scripts/wic".into(),
            kickstarts: vec![kickstart()],
            image_targets: vec!["core-image-minimal".into()],
        }
    }

    #[test]
    fn wic_model_creation_preview_is_exact_and_rejects_stale_or_unsafe_identity() {
        let draft = WicCreateDraft {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            kickstart: kickstart().identity,
            output_directory: "/build/wic-output".into(),
            generate_bmap: true,
            compression: WicCompression::Xz,
        };
        let preview = draft.preview(&capability()).unwrap();
        assert_eq!(
            preview.argv,
            vec![
                PathBuf::from("/opt/poky/scripts/wic"),
                "create".into(),
                "/layers/meta/wic/directdisk.wks".into(),
                "-e".into(),
                "core-image-minimal".into(),
                "-o".into(),
                "/build/wic-output".into(),
                "--bmap".into(),
                "--compress-with".into(),
                "xz".into(),
            ]
        );
        let mut unsafe_draft = draft.clone();
        unsafe_draft.output_directory = "/build/../escape".into();
        assert!(unsafe_draft.preview(&capability()).is_err());
        let mut stale = capability();
        if let WicCapability::Available { image_targets, .. } = &mut stale {
            image_targets.clear();
        }
        assert!(draft.preview(&stale).is_err());
    }

    #[test]
    fn wic_device_write_phrase_and_inventory_bounds_are_enforced() {
        let image = WicOutputIdentity {
            path: "/build/out/image.wic".into(),
            size_bytes: 1024,
            modified_unix_seconds: 1,
        };
        let device = WicDevice {
            identity: WicDeviceIdentity {
                path: "/dev/sdz".into(),
                major_minor: "8:240".into(),
                size_bytes: 2048,
                model: Some("test".into()),
                serial: None,
                transport: Some("usb".into()),
            },
            removable: true,
            writable: true,
            read_only: false,
            descendant_mounts: Vec::new(),
            unavailable_reason: None,
        };
        assert!(
            WicWritePreview::new(&capability(), image.clone(), &device, "WRITE /dev/sdy").is_err()
        );
        let preview =
            WicWritePreview::new(&capability(), image, &device, "WRITE /dev/sdz").unwrap();
        assert_eq!(
            preview.argv,
            vec![
                PathBuf::from("/opt/poky/scripts/wic"),
                "write".into(),
                "/build/out/image.wic".into(),
                "/dev/sdz".into(),
            ]
        );
        let mut mounted = device;
        mounted.descendant_mounts.push("/media/card".into());
        assert!(
            mounted
                .eligible_for(&preview.request.image)
                .unwrap_err()
                .contains("mounted")
        );
        let mut malformed = mounted.identity;
        malformed.major_minor = "8:".into();
        assert!(malformed.validate().is_err());
        let mut phrase = WicWritePhraseDialog {
            request: WicDeviceInventoryRequest {
                generation: 1,
                image: preview.request.image.clone(),
            },
            device: preview.request.device.clone(),
            input: String::new(),
            validation_error: Some("old".into()),
        };
        phrase.append('\n');
        for _ in 0..(MAX_WIC_WRITE_PHRASE_INPUT_BYTES + 10) {
            phrase.append('x');
        }
        assert_eq!(phrase.input.len(), MAX_WIC_WRITE_PHRASE_INPUT_BYTES);
        assert!(phrase.validation_error.is_none());
        phrase.backspace();
        assert_eq!(phrase.input.len(), MAX_WIC_WRITE_PHRASE_INPUT_BYTES - 1);

        let outputs = (0..(MAX_WIC_OUTPUTS + 10))
            .map(|index| WicOutput {
                identity: WicOutputIdentity {
                    path: PathBuf::from(format!("/build/out/{index}.wic")),
                    size_bytes: index as u64,
                    modified_unix_seconds: 1,
                },
                kind: WicOutputKind::Wic,
            })
            .collect();
        assert_eq!(
            normalize_wic_outputs(Path::new("/build/out"), outputs)
                .unwrap()
                .len(),
            MAX_WIC_OUTPUTS
        );
    }
}
