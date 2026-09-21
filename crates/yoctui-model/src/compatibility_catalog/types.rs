pub const CAPABILITY_CATALOG_VERSION: u32 = 2;
pub const MAX_CATALOG_REQUIREMENTS: usize = 32;
pub const MAX_CATALOG_PROBES: usize = 32;
pub const MAX_CATALOG_BOUNDARIES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityToolId {
    BitBake,
    BitBakePrserv,
    BitBakeSelftest,
    BitBakeDiffSigs,
    BitBakeDumpSig,
    BitBakeGetVar,
    BitBakeLayers,
    Devtool,
    BuildCompare,
    BuildHistoryDiff,
    OeCheckSstate,
    OeFindNativeSysroot,
    OeGitArchive,
    OePkgdataUtil,
    OePublishSdk,
    OeSelftest,
    Recipetool,
    Resulttool,
    Runqemu,
    SstateCacheManagement,
    Wic,
    YoctoCheckLayer,
}

impl CapabilityToolId {
    pub const fn executable_name(self) -> &'static str {
        match self {
            Self::BitBake => "bitbake",
            Self::BitBakePrserv => "bitbake-prserv",
            Self::BitBakeSelftest => "bitbake-selftest",
            Self::BitBakeDiffSigs => "bitbake-diffsigs",
            Self::BitBakeDumpSig => "bitbake-dumpsig",
            Self::BitBakeGetVar => "bitbake-getvar",
            Self::BitBakeLayers => "bitbake-layers",
            Self::Devtool => "devtool",
            Self::BuildCompare => "build-compare",
            Self::BuildHistoryDiff => "buildhistory-diff",
            Self::OeCheckSstate => "oe-check-sstate",
            Self::OeFindNativeSysroot => "oe-find-native-sysroot",
            Self::OeGitArchive => "oe-git-archive",
            Self::OePkgdataUtil => "oe-pkgdata-util",
            Self::OePublishSdk => "oe-publish-sdk",
            Self::OeSelftest => "oe-selftest",
            Self::Recipetool => "recipetool",
            Self::Resulttool => "resulttool",
            Self::Runqemu => "runqemu",
            Self::SstateCacheManagement => "sstate-cache-management.sh",
            Self::Wic => "wic",
            Self::YoctoCheckLayer => "yocto-check-layer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRequirement {
    pub tool: CapabilityToolId,
    pub subcommand: Option<String>,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetadataRequirement {
    AnyTask { names: Vec<String> },
    Variable { name: String },
    Api { name: String },
    Artifact { kind: String },
    Configuration { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CapabilityProbeSpec {
    Executable {
        tool: CapabilityToolId,
    },
    CommandVersion {
        tool: CapabilityToolId,
    },
    CommandHelp {
        tool: CapabilityToolId,
        subcommand: Option<String>,
    },
    CommandOption {
        tool: CapabilityToolId,
        subcommand: Option<String>,
        option: String,
    },
    CommandHelpText {
        tool: CapabilityToolId,
        needle: String,
    },
    MetadataAnyTask {
        names: Vec<String>,
    },
    MetadataVariable {
        name: String,
    },
    BackendCapability {
        name: String,
    },
    ProtocolCapability {
        name: String,
    },
    Artifact {
        kind: String,
    },
    Configuration {
        name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityImplementationKind {
    BackendApi,
    Command,
    MetadataTask,
    ProcessAdapter,
    Protocol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityImplementation {
    pub id: String,
    pub kind: CapabilityImplementationKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FallbackSelector {
    PositiveProbe { index: usize },
    AvailableCapability { id: CapabilityId },
    VersionInferenceWhenUnprobeable { map_key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FallbackImplementation {
    pub implementation: CapabilityImplementation,
    pub selector: FallbackSelector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvisoryReleaseBoundary {
    pub component: String,
    pub introduced: Option<String>,
    pub removed: Option<String>,
    pub source: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityCatalogEntry {
    pub id: CapabilityId,
    pub label: String,
    pub required_tools: Vec<CapabilityToolId>,
    pub required_commands: Vec<CommandRequirement>,
    pub required_metadata: Vec<MetadataRequirement>,
    pub probes: Vec<CapabilityProbeSpec>,
    pub preferred: CapabilityImplementation,
    pub fallback: Option<FallbackImplementation>,
    pub known_release_boundaries: Vec<AdvisoryReleaseBoundary>,
    pub unavailable_reason: CapabilityReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityCatalog {
    pub version: u32,
    pub entries: Vec<CapabilityCatalogEntry>,
}

impl CapabilityCatalog {
    pub fn builtin() -> Self {
        Self {
            version: CAPABILITY_CATALOG_VERSION,
            entries: CapabilityId::ALL.into_iter().map(builtin_entry).collect(),
        }
    }

    pub fn validate(&self) -> Result<(), CapabilityCatalogError> {
        if self.version == 0 {
            return Err(CapabilityCatalogError::InvalidVersion);
        }
        let expected = CapabilityId::ALL.into_iter().collect::<BTreeSet<_>>();
        let mut actual = BTreeSet::new();
        for entry in &self.entries {
            if !actual.insert(entry.id) {
                return Err(CapabilityCatalogError::Duplicate(entry.id));
            }
            validate_entry(entry)?;
        }
        let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(CapabilityCatalogError::Missing(missing));
        }
        let unknown = actual.difference(&expected).copied().collect::<Vec<_>>();
        if !unknown.is_empty() {
            return Err(CapabilityCatalogError::Unexpected(unknown));
        }
        Ok(())
    }

    pub fn entry(&self, id: CapabilityId) -> Option<&CapabilityCatalogEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityCatalogError {
    #[error("capability catalog version must be non-zero")]
    InvalidVersion,
    #[error("duplicate capability catalog entry: {0}")]
    Duplicate(CapabilityId),
    #[error("missing capability catalog entries: {0:?}")]
    Missing(Vec<CapabilityId>),
    #[error("unexpected capability catalog entries: {0:?}")]
    Unexpected(Vec<CapabilityId>),
    #[error("invalid capability catalog entry: {0}")]
    InvalidEntry(CapabilityId),
    #[error("fallback selector for {0} is invalid")]
    InvalidFallback(CapabilityId),
}

fn validate_entry(entry: &CapabilityCatalogEntry) -> Result<(), CapabilityCatalogError> {
    if !valid_text(&entry.label)
        || entry.required_tools.len() > MAX_CATALOG_REQUIREMENTS
        || entry.required_commands.len() > MAX_CATALOG_REQUIREMENTS
        || entry.required_metadata.len() > MAX_CATALOG_REQUIREMENTS
        || entry.probes.is_empty()
        || entry.probes.len() > MAX_CATALOG_PROBES
        || entry.known_release_boundaries.len() > MAX_CATALOG_BOUNDARIES
        || !valid_id(&entry.preferred.id)
        || entry
            .required_commands
            .iter()
            .any(|command| !valid_command(command, &entry.required_tools))
        || entry
            .required_metadata
            .iter()
            .any(|requirement| !valid_metadata(requirement))
        || entry
            .probes
            .iter()
            .any(|probe| !valid_probe(probe, &entry.required_tools))
        || entry
            .probes
            .iter()
            .enumerate()
            .any(|(index, probe)| entry.probes[..index].contains(probe))
        || entry
            .known_release_boundaries
            .iter()
            .any(|boundary| !valid_boundary(boundary))
    {
        return Err(CapabilityCatalogError::InvalidEntry(entry.id));
    }
    if let Some(fallback) = &entry.fallback
        && (!valid_id(&fallback.implementation.id)
            || fallback.implementation == entry.preferred
            || match &fallback.selector {
                FallbackSelector::PositiveProbe { index } => *index >= entry.probes.len(),
                FallbackSelector::AvailableCapability { id } => *id == entry.id,
                FallbackSelector::VersionInferenceWhenUnprobeable { map_key } => !valid_id(map_key),
            })
    {
        return Err(CapabilityCatalogError::InvalidFallback(entry.id));
    }
    Ok(())
}

