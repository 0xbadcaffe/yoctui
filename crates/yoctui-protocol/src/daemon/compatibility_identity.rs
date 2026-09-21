#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    StateSnapshots,
    IncrementalEvents,
    EventReplay,
    BackgroundJobs,
    BitBakeLifecycle,
    PtySessions,
    PtyWriterLease,
    PaneAttachments,
    TerminalMouse,
    EnvironmentCompatibility,
    RawExecution,
    RootfsSources,
    GracefulShutdown,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientHello {
    pub minimum_version: ProtocolVersion,
    pub maximum_version: ProtocolVersion,
    pub client_id: ClientId,
    pub client_name: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonHello {
    pub selected_version: ProtocolVersion,
    pub daemon_instance_id: DaemonInstanceId,
    pub boot_id: String,
    pub capabilities: Vec<Capability>,
    pub limits: ProtocolLimits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolLimits {
    pub maximum_frame_bytes: u32,
    pub maximum_snapshot_bytes: u32,
    pub maximum_pending_requests: u16,
    pub maximum_queue_depth: u16,
    pub maximum_terminal_rows: u16,
    pub maximum_terminal_columns: u16,
    pub maximum_clients: u16,
    pub maximum_pty_sessions: u16,
    pub maximum_scrollback_lines: u32,
    pub maximum_utility_output_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceIdentity {
    pub canonical_source: String,
    pub canonical_build: String,
    pub identity_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityIdentityAuthority {
    BackendHandshake,
    BitBakeDatastore,
    BitBakeVersionProbe,
    ConfiguredLayerMetadata,
    ExecutableProbe,
    InitializedEnvironment,
    ProtocolNegotiation,
    ReleaseMetadata,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CompatibilityDetected<T> {
    Unknown,
    Detected {
        value: T,
        authority: CompatibilityIdentityAuthority,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityReleaseIdentity {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityDistroIdentity {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilitySourceRootIdentity {
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityLayerSeriesIdentity {
    pub layer: String,
    pub root: String,
    pub compatible_series: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityToolIdentity {
    pub id: String,
    pub executable: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityBackendIdentity {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityProtocolIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityEnvironmentIdentity {
    pub build_directory: CompatibilityDetected<String>,
    pub source_roots: CompatibilityDetected<Vec<CompatibilitySourceRootIdentity>>,
    pub bitbake_version: CompatibilityDetected<String>,
    pub oe_core: CompatibilityDetected<CompatibilityReleaseIdentity>,
    pub poky: CompatibilityDetected<CompatibilityReleaseIdentity>,
    pub distro: CompatibilityDetected<CompatibilityDistroIdentity>,
    pub machine: CompatibilityDetected<String>,
    pub layer_series: CompatibilityDetected<Vec<CompatibilityLayerSeriesIdentity>>,
    pub available_tools: CompatibilityDetected<Vec<CompatibilityToolIdentity>>,
    pub backend: CompatibilityDetected<CompatibilityBackendIdentity>,
    pub protocol: CompatibilityDetected<CompatibilityProtocolIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityReasonData {
    pub code: String,
    pub message: String,
    pub requirement: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityEvidenceKind {
    DirectProbe,
    BackendNegotiation,
    ProtocolNegotiation,
    Metadata,
    ExecutableIdentity,
    ReleaseVersionFallback,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityEvidenceOutcome {
    Positive,
    Negative,
    Inconclusive,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityEvidenceData {
    pub kind: CompatibilityEvidenceKind,
    pub outcome: CompatibilityEvidenceOutcome,
    pub subject: String,
    pub detail: String,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CompatibilityStateData {
    Available,
    AvailableWithLimitations {
        reason: CompatibilityReasonData,
        limitations: Vec<String>,
    },
    Unavailable {
        reason: CompatibilityReasonData,
    },
    Unknown {
        reason: CompatibilityReasonData,
    },
    Unsupported {
        reason: CompatibilityReasonData,
    },
    #[serde(other)]
    UnknownWireState,
}

impl CompatibilityStateData {
    pub const fn is_enabled(&self) -> bool {
        matches!(
            self,
            Self::Available | Self::AvailableWithLimitations { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityImplementationData {
    pub id: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityCapabilityData {
    pub id: String,
    pub state: CompatibilityStateData,
    pub evidence: Vec<CompatibilityEvidenceData>,
    pub implementation: Option<CompatibilityImplementationData>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilitySnapshotData {
    pub schema_version: u16,
    pub generation: u64,
    pub environment: CompatibilityEnvironmentIdentity,
    pub capabilities: Vec<CompatibilityCapabilityData>,
}

impl CompatibilitySnapshotData {
    pub fn validate(&self) -> Result<(), CompatibilityProtocolError> {
        if self.schema_version != COMPATIBILITY_SCHEMA_VERSION {
            return Err(CompatibilityProtocolError::UnsupportedSchema(
                self.schema_version,
            ));
        }
        if self.generation == 0 {
            return Err(CompatibilityProtocolError::InvalidGeneration);
        }
        validate_environment(&self.environment)?;
        if self.capabilities.len() > MAX_COMPATIBILITY_CAPABILITIES {
            return Err(CompatibilityProtocolError::Oversized("capabilities"));
        }
        let mut ids = std::collections::BTreeSet::new();
        for capability in &self.capabilities {
            if !valid_id(&capability.id) {
                return Err(CompatibilityProtocolError::InvalidText("capability id"));
            }
            if !ids.insert(&capability.id) {
                return Err(CompatibilityProtocolError::DuplicateCapability(
                    capability.id.clone(),
                ));
            }
            validate_capability(capability)?;
        }
        Ok(())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CompatibilityProtocolError {
    #[error("unsupported compatibility schema version: {0}")]
    UnsupportedSchema(u16),
    #[error("compatibility generation must be non-zero")]
    InvalidGeneration,
    #[error("oversized compatibility field: {0}")]
    Oversized(&'static str),
    #[error("invalid compatibility text field: {0}")]
    InvalidText(&'static str),
    #[error("invalid compatibility path field: {0}")]
    InvalidPath(&'static str),
    #[error("duplicate compatibility capability: {0}")]
    DuplicateCapability(String),
    #[error("compatibility capability evidence does not support its state: {0}")]
    EvidenceMismatch(String),
    #[error("unknown compatibility identity authority cannot establish detected identity")]
    UnknownIdentityAuthority,
}
