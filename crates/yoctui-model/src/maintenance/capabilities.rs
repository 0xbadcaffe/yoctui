pub const MAX_MAINTENANCE_TOOLS: usize = 32;
pub const MAX_MAINTENANCE_TARGETS: usize = 128;
pub const MAX_MAINTENANCE_PATHS: usize = 4_096;
pub const MAX_MAINTENANCE_ARGUMENTS: usize = 256;
pub const MAX_MAINTENANCE_OUTPUT: usize = 512;
pub const MAX_MAINTENANCE_EVIDENCE: usize = 256;
pub const MAX_MAINTENANCE_SESSIONS: usize = 32;
pub const MAX_MAINTENANCE_LIMITATIONS: usize = 128;
pub const MAX_MAINTENANCE_TEXT_BYTES: usize = 4_096;

fn bounded_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_MAINTENANCE_TEXT_BYTES
        && !value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
}

fn bounded_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !matches!(value, "." | "..")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+' | ':')
        })
}

fn absolute_normal_path(path: &Path) -> bool {
    is_absolute_normal_path_within(path, MAX_MAINTENANCE_TEXT_BYTES)
}

fn normalize_text(mut values: Vec<String>, maximum: usize) -> Vec<String> {
    values.retain(|value| bounded_text(value));
    values.sort();
    values.dedup();
    values.truncate(maximum);
    values
}

fn normalize_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.retain(|path| absolute_normal_path(path));
    paths.sort();
    paths.dedup();
    paths.truncate(MAX_MAINTENANCE_PATHS);
    paths
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MaintenanceView {
    #[default]
    Sstate,
    Services,
    Release,
    Integrations,
}

impl MaintenanceView {
    pub fn cycle(self, backwards: bool) -> Self {
        match (self, backwards) {
            (Self::Sstate, false) | (Self::Release, true) => Self::Services,
            (Self::Services, false) | (Self::Integrations, true) => Self::Release,
            (Self::Release, false) | (Self::Sstate, true) => Self::Integrations,
            (Self::Integrations, false) | (Self::Services, true) => Self::Sstate,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Sstate => 0,
            Self::Services => 1,
            Self::Release => 2,
            Self::Integrations => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaintenanceTool {
    OeCheckSstate,
    SstateCacheManagement,
    PrServiceTool,
    LockedSignatureCache,
    BuildHistoryDiff,
    BuildCompare,
    GitArchive,
    CreatePullRequest,
    SendPullRequest,
    SendErrorReport,
    Toaster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceToolInterface {
    Native,
    SstatePython,
    SstateLegacyShell,
    DetectionOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceFileIdentity {
    pub path: PathBuf,
    pub byte_size: u64,
    pub modified_at: SystemTime,
}

impl MaintenanceFileIdentity {
    pub fn new(
        path: PathBuf,
        byte_size: u64,
        modified_at: SystemTime,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&path) {
            return Err("Maintenance file identity must be a canonical absolute non-root path");
        }
        Ok(Self {
            path,
            byte_size,
            modified_at,
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.path.clone(), self.byte_size, self.modified_at).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceToolCapability {
    Available {
        tool: MaintenanceTool,
        executable: MaintenanceFileIdentity,
        interface: MaintenanceToolInterface,
    },
    Unavailable {
        tool: MaintenanceTool,
        reason: String,
    },
}

impl MaintenanceToolCapability {
    pub fn tool(&self) -> MaintenanceTool {
        match self {
            Self::Available { tool, .. } | Self::Unavailable { tool, .. } => *tool,
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            Self::Available { executable, .. } => executable.is_valid(),
            Self::Unavailable { reason, .. } => bounded_text(reason),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MaintenanceMetadata {
    pub build_dir: Option<PathBuf>,
    pub sstate_dir: Option<PathBuf>,
    pub tmp_dir: Option<PathBuf>,
    pub stamps_dirs: Vec<PathBuf>,
    pub buildhistory_dir: Option<PathBuf>,
    pub prserv_host: Option<String>,
    pub hashserve: Option<String>,
    pub hashserve_upstream: Option<String>,
    pub signature_handler: Option<String>,
    pub native_lsb: Option<String>,
    pub machine: Option<String>,
    pub distro: Option<String>,
}

impl MaintenanceMetadata {
    pub fn new(mut value: Self) -> Result<Self, &'static str> {
        for path in [
            value.build_dir.as_ref(),
            value.sstate_dir.as_ref(),
            value.tmp_dir.as_ref(),
            value.buildhistory_dir.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if !absolute_normal_path(path) {
                return Err("Maintenance metadata path is invalid");
            }
        }
        value.stamps_dirs = normalize_paths(value.stamps_dirs);
        for text in [
            value.prserv_host.as_deref(),
            value.hashserve.as_deref(),
            value.hashserve_upstream.as_deref(),
            value.signature_handler.as_deref(),
            value.native_lsb.as_deref(),
            value.machine.as_deref(),
            value.distro.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if !bounded_text(text) {
                return Err("Maintenance metadata text is invalid");
            }
        }
        Ok(value)
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.clone()).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceCapabilitySnapshot {
    pub metadata: MaintenanceMetadata,
    pub tools: Vec<MaintenanceToolCapability>,
    pub limitations: Vec<String>,
}

impl MaintenanceCapabilitySnapshot {
    pub fn new(
        metadata: MaintenanceMetadata,
        mut tools: Vec<MaintenanceToolCapability>,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if !metadata.is_valid() || tools.iter().any(|tool| !tool.is_valid()) {
            return Err("Maintenance capability snapshot is invalid");
        }
        tools.sort_by_key(MaintenanceToolCapability::tool);
        tools.dedup_by_key(|tool| tool.tool());
        tools.truncate(MAX_MAINTENANCE_TOOLS);
        Ok(Self {
            metadata,
            tools,
            limitations: normalize_text(limitations, MAX_MAINTENANCE_LIMITATIONS),
        })
    }

    pub fn capability(&self, tool: MaintenanceTool) -> Option<&MaintenanceToolCapability> {
        self.tools
            .iter()
            .find(|capability| capability.tool() == tool)
    }

    pub fn supports(&self, tool: MaintenanceTool) -> bool {
        matches!(
            self.capability(tool),
            Some(MaintenanceToolCapability::Available { .. })
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum MaintenanceCapability {
    #[default]
    NotInspected,
    Loading(u64),
    Available {
        request: u64,
        snapshot: MaintenanceCapabilitySnapshot,
    },
    Partial {
        request: u64,
        snapshot: MaintenanceCapabilitySnapshot,
        limitations: Vec<String>,
    },
    Failed {
        request: u64,
        message: String,
    },
}

impl MaintenanceCapability {
    pub fn request(&self) -> Option<u64> {
        match self {
            Self::Loading(request)
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Failed { request, .. } => Some(*request),
            Self::NotInspected => None,
        }
    }

    pub fn snapshot(&self) -> Option<&MaintenanceCapabilitySnapshot> {
        match self {
            Self::Available { snapshot, .. } | Self::Partial { snapshot, .. } => Some(snapshot),
            _ => None,
        }
    }
}
