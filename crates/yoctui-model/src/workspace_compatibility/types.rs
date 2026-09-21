#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceDestination {
    Dashboard,
    Recipes,
    Layers,
    Configuration,
    Tasks,
    BuildHistory,
    Logs,
    Errors,
    Dependencies,
    Signatures,
    Packages,
    Images,
    Kernel,
    Firmware,
    Sdk,
    Testing,
    Security,
    Qa,
    RawMode,
    Devtool,
    QemuWic,
    Maintenance,
    ProjectProfiles,
    TerminalSessions,
    BuildEnvironment,
    Compatibility,
    Settings,
    Help,
}

impl WorkspaceDestination {
    pub const ALL: [Self; 28] = [
        Self::Dashboard,
        Self::Recipes,
        Self::Layers,
        Self::Configuration,
        Self::Tasks,
        Self::BuildHistory,
        Self::Logs,
        Self::Errors,
        Self::Dependencies,
        Self::Signatures,
        Self::Packages,
        Self::Images,
        Self::Kernel,
        Self::Firmware,
        Self::Sdk,
        Self::Testing,
        Self::Security,
        Self::Qa,
        Self::RawMode,
        Self::Devtool,
        Self::QemuWic,
        Self::Maintenance,
        Self::ProjectProfiles,
        Self::TerminalSessions,
        Self::BuildEnvironment,
        Self::Compatibility,
        Self::Settings,
        Self::Help,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceEffectRequirement {
    ClientLocal,
    /// Safe environment inspection belongs to the daemon probe coordinator;
    /// a client must not execute it to infer support independently.
    DaemonProbe {
        capabilities: Vec<CapabilityId>,
    },
    Capabilities {
        /// Every ID in this set is required.
        all: Vec<CapabilityId>,
        /// At least one ID in this set is required when non-empty.
        any: Vec<CapabilityId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceAvailabilityState {
    Available,
    AvailableWithLimitations,
    Unavailable,
    Unsupported,
    Unknown,
}

impl WorkspaceAvailabilityState {
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Available | Self::AvailableWithLimitations)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCapabilityIssue {
    pub capability: Option<CapabilityId>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceAvailability {
    pub state: WorkspaceAvailabilityState,
    pub issues: Vec<WorkspaceCapabilityIssue>,
    pub implementations: Vec<(CapabilityId, String)>,
}

impl WorkspaceAvailability {
    pub const fn is_enabled(&self) -> bool {
        self.state.is_enabled()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceCompatibilityState {
    authority: Option<DaemonCompatibilitySnapshot>,
}

impl WorkspaceCompatibilityState {
    pub const fn authority(&self) -> Option<&DaemonCompatibilitySnapshot> {
        self.authority.as_ref()
    }

    pub fn availability(&self, requirement: &WorkspaceEffectRequirement) -> WorkspaceAvailability {
        workspace_requirement_availability(self.authority(), requirement)
    }

    pub fn install(
        &mut self,
        authority: DaemonCompatibilitySnapshot,
    ) -> Result<WorkspaceSnapshotInstall, WorkspaceCompatibilityError> {
        let authority = authority
            .normalize()
            .map_err(|error| WorkspaceCompatibilityError::InvalidSnapshot(error.to_string()))?;
        if let Some(current) = self.authority.as_ref() {
            if authority.snapshot.generation < current.snapshot.generation {
                return Err(WorkspaceCompatibilityError::StaleGeneration {
                    current: current.snapshot.generation,
                    received: authority.snapshot.generation,
                });
            }
            if authority.snapshot.generation == current.snapshot.generation {
                if &authority == current {
                    return Ok(WorkspaceSnapshotInstall::Unchanged);
                }
                return Err(WorkspaceCompatibilityError::ConflictingGeneration(
                    authority.snapshot.generation,
                ));
            }
        }
        self.authority = Some(authority);
        Ok(WorkspaceSnapshotInstall::Replaced)
    }

    pub fn invalidate(&mut self) {
        self.authority = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSnapshotInstall {
    Replaced,
    Unchanged,
    Invalidated,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkspaceCompatibilityError {
    #[error("invalid workspace capability snapshot: {0}")]
    InvalidSnapshot(String),
    #[error("stale workspace capability snapshot: current {current}, received {received}")]
    StaleGeneration { current: u64, received: u64 },
    #[error("workspace capability generation {0} conflicts with installed authority")]
    ConflictingGeneration(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRevalidation {
    pub install: WorkspaceSnapshotInstall,
    pub closed_dialog: bool,
    pub reason: Option<String>,
}

impl WorkspaceEffectRequirement {
    pub(crate) fn one(id: CapabilityId) -> Self {
        Self::Capabilities {
            all: vec![id],
            any: Vec::new(),
        }
    }

    pub(crate) fn all(ids: &[CapabilityId]) -> Self {
        Self::Capabilities {
            all: ids.to_vec(),
            any: Vec::new(),
        }
    }

    pub(crate) fn all_and_any(all: &[CapabilityId], any: &[CapabilityId]) -> Self {
        Self::Capabilities {
            all: all.to_vec(),
            any: any.to_vec(),
        }
    }

    fn probe(ids: &[CapabilityId]) -> Self {
        Self::DaemonProbe {
            capabilities: ids.to_vec(),
        }
    }
}

