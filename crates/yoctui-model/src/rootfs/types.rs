pub const MAX_ROOTFS_PACKAGES: usize = 8_192;
pub const MAX_ROOTFS_ENTRIES: usize = 65_536;
pub const MAX_ROOTFS_DEPTH: usize = 64;
pub const MAX_ROOTFS_LIMITATIONS: usize = 64;
pub const MAX_ROOTFS_TEXT_BYTES: usize = 512;
pub const MAX_ROOTFS_PATH_BYTES: usize = 4_096;
pub const MAX_ROOTFS_SYSTEM_PREVIEW_BYTES: usize = 8 * 1024;
pub const MAX_ROOTFS_UDEV_RULES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImagesView {
    #[default]
    Artifacts,
    RootfsPackages,
    RootfsFilesystem,
    SystemdServices,
    SystemDbus,
    UdevRules,
}

impl ImagesView {
    pub const ALL: [Self; 6] = [
        Self::Artifacts,
        Self::RootfsPackages,
        Self::RootfsFilesystem,
        Self::SystemdServices,
        Self::SystemDbus,
        Self::UdevRules,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Artifacts => "Artifacts",
            Self::RootfsPackages => "Rootfs packages",
            Self::RootfsFilesystem => "Rootfs filesystem",
            Self::SystemdServices => "systemd services",
            Self::SystemDbus => "System D-Bus",
            Self::UdevRules => "udev rules",
        }
    }

    pub fn shifted(self, delta: isize) -> Self {
        let current = Self::ALL.iter().position(|view| *view == self).unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(Self::ALL.len() as isize) as usize;
        Self::ALL[next]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootfsCompositionRequest {
    pub generation: u64,
    pub image: ImageArtifactIdentity,
}

impl RootfsCompositionRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.generation == 0 {
            return Err("rootfs composition generations must be non-zero");
        }
        self.image.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootfsInstalledPackage {
    pub identity: PackageIdentity,
    pub recipe: Option<String>,
    pub category: String,
    pub installed_size_bytes: u64,
    pub file_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RootfsEntryKind {
    Directory,
    RegularFile,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootfsPathIdentity(pub PathBuf);

impl RootfsPathIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        if rootfs_path_is_valid(&self.0) {
            Ok(())
        } else {
            Err("rootfs entry paths must be normalized absolute logical paths")
        }
    }

    pub fn depth(&self) -> usize {
        self.0
            .components()
            .filter(|part| !matches!(part, Component::RootDir))
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootfsEntry {
    pub identity: RootfsPathIdentity,
    pub kind: RootfsEntryKind,
    pub size_bytes: u64,
    pub package: Option<PackageIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RootfsPackageInventory {
    pub packages: Vec<RootfsInstalledPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RootfsFilesystemTree {
    pub entries: Vec<RootfsEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootfsSystemdService {
    pub name: String,
    pub logical_path: RootfsPathIdentity,
    pub host_path: PathBuf,
    pub description: Option<String>,
    pub bus_name: Option<String>,
    pub enabled_by: Vec<String>,
    pub preview: String,
    pub preview_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootfsDbusService {
    pub name: String,
    pub logical_path: RootfsPathIdentity,
    pub host_path: PathBuf,
    pub exec: Option<String>,
    pub user: Option<String>,
    pub systemd_service: Option<String>,
    pub policy_files: Vec<RootfsPathIdentity>,
    pub preview: String,
    pub preview_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RootfsSystemInventory {
    pub systemd_services: Vec<RootfsSystemdService>,
    pub dbus_services: Vec<RootfsDbusService>,
    pub udev_rules: Vec<RootfsUdevRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootfsUdevRule {
    pub name: String,
    pub logical_path: RootfsPathIdentity,
    pub masked: bool,
    pub overridden_by: Option<RootfsPathIdentity>,
    pub limitation: Option<String>,
    pub preview: String,
    pub preview_truncated: bool,
}

impl RootfsUdevRule {
    pub fn status(&self) -> &'static str {
        if self.overridden_by.is_some() {
            "Overridden"
        } else if self.limitation.is_some() {
            "Unresolved"
        } else if self.masked {
            "Masked"
        } else {
            "Selected file"
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootfsAuthority<T> {
    Available(T),
    Partial { value: T, limitations: Vec<String> },
    Unavailable { reason: String },
}

impl<T> RootfsAuthority<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Available(value) | Self::Partial { value, .. } => Some(value),
            Self::Unavailable { .. } => None,
        }
    }

    pub fn is_partial(&self) -> bool {
        matches!(self, Self::Partial { .. })
    }

    pub fn is_unavailable(&self) -> bool {
        matches!(self, Self::Unavailable { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootfsComposition {
    pub image: ImageArtifactIdentity,
    pub installed_packages: RootfsAuthority<RootfsPackageInventory>,
    pub filesystem_tree: RootfsAuthority<RootfsFilesystemTree>,
    pub system_inventory: RootfsAuthority<RootfsSystemInventory>,
    pub root_directory: Option<PathBuf>,
}

impl RootfsComposition {
    pub fn package_inventory(&self) -> Option<&RootfsPackageInventory> {
        self.installed_packages.value()
    }

    pub fn filesystem_tree(&self) -> Option<&RootfsFilesystemTree> {
        self.filesystem_tree.value()
    }

    pub fn system_inventory(&self) -> Option<&RootfsSystemInventory> {
        self.system_inventory.value()
    }

    pub fn is_empty(&self) -> bool {
        self.package_inventory()
            .is_none_or(|inventory| inventory.packages.is_empty())
            && self
                .filesystem_tree()
                .is_none_or(|tree| tree.entries.is_empty())
            && self.system_inventory().is_none_or(|inventory| {
                inventory.systemd_services.is_empty()
                    && inventory.dbus_services.is_empty()
                    && inventory.udev_rules.is_empty()
            })
    }

    pub fn is_unavailable(&self) -> bool {
        self.installed_packages.is_unavailable()
            && self.filesystem_tree.is_unavailable()
            && self.system_inventory.is_unavailable()
    }

    pub fn is_partial(&self) -> bool {
        self.installed_packages.is_partial()
            || self.filesystem_tree.is_partial()
            || self.system_inventory.is_partial()
            || [
                self.installed_packages.is_unavailable(),
                self.filesystem_tree.is_unavailable(),
                self.system_inventory.is_unavailable(),
            ]
            .windows(2)
            .any(|pair| pair[0] != pair[1])
    }
}

