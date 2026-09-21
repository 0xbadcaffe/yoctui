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
