pub const MAX_ENVIRONMENT_IDENTITY_TEXT_BYTES: usize = 1_024;
pub const MAX_ENVIRONMENT_IDENTITY_PATH_BYTES: usize = 4_096;
pub const MAX_ENVIRONMENT_SOURCE_ROOTS: usize = 256;
pub const MAX_ENVIRONMENT_LAYER_SERIES: usize = 256;
pub const MAX_ENVIRONMENT_TOOLS: usize = 256;
pub const MAX_LAYER_COMPATIBLE_SERIES: usize = 64;
pub const MAX_CAPABILITY_RECORDS: usize = 512;
pub const MAX_CAPABILITY_EVIDENCE: usize = 32;
pub const MAX_CAPABILITY_LIMITATIONS: usize = 32;
pub const MAX_CAPABILITY_EVIDENCE_ARGUMENTS: usize = 64;

/// Authoritative origins accepted by the environment identity model.
///
/// There is deliberately no branch-name, directory-name, nearest-tag, or
/// inferred-version variant. Those values may be diagnostics, but cannot
/// become authoritative identity through this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityAuthority {
    BackendHandshake,
    BitBakeDatastore,
    BitBakeVersionProbe,
    ConfiguredLayerMetadata,
    ExecutableProbe,
    InitializedEnvironment,
    ProtocolNegotiation,
    ReleaseMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AuthoritativeValue<T> {
    #[default]
    Unknown,
    Detected {
        value: T,
        authority: IdentityAuthority,
    },
}

impl<T> AuthoritativeValue<T> {
    pub const fn unknown() -> Self {
        Self::Unknown
    }

    pub const fn detected(value: T, authority: IdentityAuthority) -> Self {
        Self::Detected { value, authority }
    }

    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Unknown => None,
            Self::Detected { value, .. } => Some(value),
        }
    }

    pub const fn authority(&self) -> Option<IdentityAuthority> {
        match self {
            Self::Unknown => None,
            Self::Detected { authority, .. } => Some(*authority),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReleaseIdentity {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DistroIdentity {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRootKind {
    CoreBase,
    OpenEmbeddedCore,
    Poky,
    Layer,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceRootIdentity {
    pub kind: SourceRootKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LayerSeriesIdentity {
    pub layer: String,
    pub root: PathBuf,
    pub compatible_series: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ToolIdentity {
    pub id: String,
    pub executable: PathBuf,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BackendIdentity {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProtocolIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct YoctoEnvironmentIdentity {
    pub build_directory: AuthoritativeValue<PathBuf>,
    pub source_roots: AuthoritativeValue<Vec<SourceRootIdentity>>,
    pub bitbake_version: AuthoritativeValue<String>,
    pub oe_core: AuthoritativeValue<ReleaseIdentity>,
    pub poky: AuthoritativeValue<ReleaseIdentity>,
    pub distro: AuthoritativeValue<DistroIdentity>,
    pub machine: AuthoritativeValue<String>,
    pub layer_series: AuthoritativeValue<Vec<LayerSeriesIdentity>>,
    pub available_tools: AuthoritativeValue<Vec<ToolIdentity>>,
    pub backend: AuthoritativeValue<BackendIdentity>,
    pub protocol: AuthoritativeValue<ProtocolIdentity>,
}

impl YoctoEnvironmentIdentity {
    pub fn normalize(mut self) -> Result<Self, EnvironmentIdentityError> {
        validate_detected(
            "build_directory",
            &self.build_directory,
            &[
                IdentityAuthority::BackendHandshake,
                IdentityAuthority::BitBakeDatastore,
                IdentityAuthority::InitializedEnvironment,
            ],
            |path| valid_absolute_path(path),
        )?;
        validate_detected(
            "bitbake_version",
            &self.bitbake_version,
            &[
                IdentityAuthority::BackendHandshake,
                IdentityAuthority::BitBakeVersionProbe,
            ],
            |value| valid_text(value),
        )?;
        validate_detected(
            "oe_core",
            &self.oe_core,
            &[
                IdentityAuthority::BitBakeDatastore,
                IdentityAuthority::ConfiguredLayerMetadata,
                IdentityAuthority::ReleaseMetadata,
            ],
            valid_release,
        )?;
        validate_detected(
            "poky",
            &self.poky,
            &[
                IdentityAuthority::BitBakeDatastore,
                IdentityAuthority::ConfiguredLayerMetadata,
                IdentityAuthority::ReleaseMetadata,
            ],
            valid_release,
        )?;
        validate_detected(
            "distro",
            &self.distro,
            &[IdentityAuthority::BitBakeDatastore],
            |value| valid_token(&value.name) && value.version.as_deref().is_none_or(valid_text),
        )?;
        validate_detected(
            "machine",
            &self.machine,
            &[IdentityAuthority::BitBakeDatastore],
            |value| valid_token(value),
        )?;
        validate_detected(
            "backend",
            &self.backend,
            &[IdentityAuthority::BackendHandshake],
            |value| valid_token(&value.name) && value.version.as_deref().is_none_or(valid_text),
        )?;
        validate_detected(
            "protocol",
            &self.protocol,
            &[IdentityAuthority::ProtocolNegotiation],
            |value| valid_token(&value.name) && valid_text(&value.version),
        )?;

        normalize_source_roots(&mut self.source_roots)?;
        normalize_layer_series(&mut self.layer_series)?;
        normalize_tools(&mut self.available_tools)?;
        Ok(self)
    }
}

