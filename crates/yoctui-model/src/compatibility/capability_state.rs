#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CapabilityReasonCode(String);

impl CapabilityReasonCode {
    pub fn new(value: impl Into<String>) -> Result<Self, CapabilityModelError> {
        let value = value.into();
        if !valid_reason_code(&value) {
            return Err(CapabilityModelError::InvalidReasonCode(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReason {
    pub code: CapabilityReasonCode,
    pub message: String,
    pub requirement: Option<String>,
}

impl CapabilityReason {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        requirement: Option<String>,
    ) -> Result<Self, CapabilityModelError> {
        let reason = Self {
            code: CapabilityReasonCode::new(code)?,
            message: message.into(),
            requirement,
        };
        reason.validate()?;
        Ok(reason)
    }

    fn validate(&self) -> Result<(), CapabilityModelError> {
        if !valid_reason_code(self.code.as_str())
            || !valid_text(&self.message)
            || self
                .requirement
                .as_deref()
                .is_some_and(|value| !valid_text(value))
        {
            return Err(CapabilityModelError::InvalidReason);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityEvidenceKind {
    DirectProbe,
    BackendNegotiation,
    ProtocolNegotiation,
    Metadata,
    ExecutableIdentity,
    ReleaseVersionFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityEvidenceOutcome {
    Positive,
    Negative,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEvidence {
    pub kind: CapabilityEvidenceKind,
    pub outcome: CapabilityEvidenceOutcome,
    pub subject: String,
    pub detail: String,
    #[serde(default)]
    pub argv: Vec<String>,
}

impl CapabilityEvidence {
    fn validate(&self) -> Result<(), CapabilityModelError> {
        if !valid_text(&self.subject)
            || !valid_text(&self.detail)
            || self.argv.len() > MAX_CAPABILITY_EVIDENCE_ARGUMENTS
            || self.argv.iter().any(|argument| !valid_text(argument))
        {
            return Err(CapabilityModelError::InvalidEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CapabilityState {
    Available,
    AvailableWithLimitations {
        reason: CapabilityReason,
        limitations: Vec<String>,
    },
    Unavailable {
        reason: CapabilityReason,
    },
    Unknown {
        reason: CapabilityReason,
    },
    Unsupported {
        reason: CapabilityReason,
    },
}

impl CapabilityState {
    pub const fn is_enabled(&self) -> bool {
        matches!(
            self,
            Self::Available | Self::AvailableWithLimitations { .. }
        )
    }

    pub const fn reason(&self) -> Option<&CapabilityReason> {
        match self {
            Self::Available => None,
            Self::AvailableWithLimitations { reason, .. }
            | Self::Unavailable { reason }
            | Self::Unknown { reason }
            | Self::Unsupported { reason } => Some(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRecord {
    pub id: CapabilityId,
    pub state: CapabilityState,
    pub evidence: Vec<CapabilityEvidence>,
}

impl CapabilityRecord {
    fn normalize(&mut self) -> Result<(), CapabilityModelError> {
        if self.evidence.len() > MAX_CAPABILITY_EVIDENCE {
            return Err(CapabilityModelError::TooMuchEvidence {
                id: self.id,
                count: self.evidence.len(),
            });
        }
        if self
            .evidence
            .iter()
            .any(|evidence| evidence.validate().is_err())
        {
            return Err(CapabilityModelError::InvalidEvidence);
        }
        match &mut self.state {
            CapabilityState::Available => {
                require_evidence(self.id, &self.evidence, CapabilityEvidenceOutcome::Positive)?
            }
            CapabilityState::AvailableWithLimitations {
                reason,
                limitations,
            } => {
                reason.validate()?;
                if limitations.is_empty()
                    || limitations.len() > MAX_CAPABILITY_LIMITATIONS
                    || limitations.iter().any(|limitation| !valid_text(limitation))
                {
                    return Err(CapabilityModelError::InvalidLimitations(self.id));
                }
                limitations.sort();
                limitations.dedup();
                require_evidence(self.id, &self.evidence, CapabilityEvidenceOutcome::Positive)?;
            }
            CapabilityState::Unavailable { reason } => {
                reason.validate()?;
                require_evidence(self.id, &self.evidence, CapabilityEvidenceOutcome::Negative)?;
            }
            CapabilityState::Unknown { reason } | CapabilityState::Unsupported { reason } => {
                reason.validate()?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySnapshot {
    pub generation: u64,
    pub environment: YoctoEnvironmentIdentity,
    pub capabilities: Vec<CapabilityRecord>,
}

impl CapabilitySnapshot {
    pub fn normalize(mut self) -> Result<Self, CapabilityModelError> {
        if self.generation == 0 {
            return Err(CapabilityModelError::InvalidGeneration);
        }
        self.environment = self.environment.normalize()?;
        if self.capabilities.len() > MAX_CAPABILITY_RECORDS {
            return Err(CapabilityModelError::TooManyCapabilities(
                self.capabilities.len(),
            ));
        }
        let mut seen = BTreeSet::new();
        for capability in &mut self.capabilities {
            if !seen.insert(capability.id) {
                return Err(CapabilityModelError::DuplicateCapability(capability.id));
            }
            capability.normalize()?;
        }
        self.capabilities.sort_by_key(|capability| capability.id);
        Ok(self)
    }

    pub fn capability(&self, id: CapabilityId) -> Option<&CapabilityRecord> {
        self.capabilities
            .binary_search_by_key(&id, |capability| capability.id)
            .ok()
            .map(|index| &self.capabilities[index])
    }

    pub fn allows(&self, id: CapabilityId) -> bool {
        self.capability(id)
            .is_some_and(|capability| capability.state.is_enabled())
    }

    pub fn availability_summary(&self) -> CapabilityAvailabilitySummary {
        let mut summary = CapabilityAvailabilitySummary::default();
        for capability in &self.capabilities {
            match capability.state {
                CapabilityState::Available => summary.available += 1,
                CapabilityState::AvailableWithLimitations { .. } => summary.limited += 1,
                CapabilityState::Unavailable { .. } => summary.unavailable += 1,
                CapabilityState::Unknown { .. } => summary.unknown += 1,
                CapabilityState::Unsupported { .. } => summary.unsupported += 1,
            }
        }
        summary
    }

    pub fn operating_mode(&self) -> EnvironmentOperatingMode {
        let summary = self.availability_summary();
        if summary.unavailable == 0
            && summary.unknown == 0
            && summary.unsupported == 0
            && summary.limited == 0
        {
            EnvironmentOperatingMode::Full
        } else if summary.available + summary.limited > 0 {
            EnvironmentOperatingMode::Degraded
        } else {
            EnvironmentOperatingMode::Diagnostic
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CapabilityAvailabilitySummary {
    pub available: usize,
    pub limited: usize,
    pub unavailable: usize,
    pub unknown: usize,
    pub unsupported: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentOperatingMode {
    Full,
    Degraded,
    Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityCacheKey {
    pub environment: YoctoEnvironmentIdentity,
    pub workspace_identity: String,
    pub initialized_environment_digest: String,
    pub layer_configuration_digest: String,
    pub build_configuration_digest: String,
    pub daemon_workspace_identity: String,
}

impl CapabilityCacheKey {
    pub fn normalize(mut self) -> Result<Self, CapabilityCacheKeyError> {
        self.environment = self.environment.normalize()?;
        if !valid_text(&self.workspace_identity)
            || !valid_text(&self.daemon_workspace_identity)
            || !valid_digest(&self.initialized_environment_digest)
            || !valid_digest(&self.layer_configuration_digest)
            || !valid_digest(&self.build_configuration_digest)
        {
            return Err(CapabilityCacheKeyError::InvalidField);
        }
        Ok(self)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityCacheKeyError {
    #[error(transparent)]
    InvalidEnvironment(#[from] EnvironmentIdentityError),
    #[error("capability cache key contains an invalid field")]
    InvalidField,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityModelError {
    #[error(transparent)]
    InvalidEnvironment(#[from] EnvironmentIdentityError),
    #[error("capability snapshot generation must be non-zero")]
    InvalidGeneration,
    #[error("too many capabilities in snapshot: {0}")]
    TooManyCapabilities(usize),
    #[error("duplicate capability in snapshot: {0}")]
    DuplicateCapability(CapabilityId),
    #[error("invalid capability reason code: {0}")]
    InvalidReasonCode(String),
    #[error("invalid capability reason")]
    InvalidReason,
    #[error("invalid capability evidence")]
    InvalidEvidence,
    #[error("too much evidence for capability {id}: {count}")]
    TooMuchEvidence { id: CapabilityId, count: usize },
    #[error("capability {id} lacks required {outcome:?} evidence")]
    MissingEvidence {
        id: CapabilityId,
        outcome: CapabilityEvidenceOutcome,
    },
    #[error("invalid limitations for capability {0}")]
    InvalidLimitations(CapabilityId),
}

fn require_evidence(
    id: CapabilityId,
    evidence: &[CapabilityEvidence],
    outcome: CapabilityEvidenceOutcome,
) -> Result<(), CapabilityModelError> {
    if evidence.iter().any(|item| item.outcome == outcome) {
        Ok(())
    } else {
        Err(CapabilityModelError::MissingEvidence { id, outcome })
    }
}

fn valid_reason_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'.'
        })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

