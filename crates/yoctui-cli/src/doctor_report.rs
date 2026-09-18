//! Doctor report.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DoctorCompatibilityAuthority {
    Current,
    Unavailable,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DoctorCompatibilityMode {
    Full,
    Degraded,
    Diagnostic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DoctorReleaseSupport {
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct DoctorCompatibilitySummary {
    pub(crate) available: usize,
    pub(crate) limited: usize,
    pub(crate) unavailable: usize,
    pub(crate) unknown: usize,
    pub(crate) unsupported: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DoctorCapabilityIssue {
    pub(crate) id: String,
    pub(crate) state: String,
    pub(crate) reason_code: String,
    pub(crate) reason: String,
    pub(crate) requirement: Option<String>,
    pub(crate) limitations: Vec<String>,
    pub(crate) implementation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DoctorMissingTool {
    pub(crate) tool: String,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DoctorCompatibilityReport {
    pub(crate) schema: &'static str,
    pub(crate) authority: DoctorCompatibilityAuthority,
    pub(crate) authority_reason: Option<String>,
    pub(crate) release_support: DoctorReleaseSupport,
    pub(crate) release_support_reason: String,
    pub(crate) operating_mode: Option<DoctorCompatibilityMode>,
    pub(crate) schema_version: Option<u16>,
    pub(crate) generation: Option<u64>,
    pub(crate) environment: Option<yoctui_protocol::daemon::CompatibilityEnvironmentIdentity>,
    pub(crate) summary: DoctorCompatibilitySummary,
    pub(crate) missing_tools: Vec<DoctorMissingTool>,
    pub(crate) limited_features: Vec<DoctorCapabilityIssue>,
    pub(crate) unavailable_features: Vec<DoctorCapabilityIssue>,
    pub(crate) unsupported_features: Vec<DoctorCapabilityIssue>,
    pub(crate) unknown_features: Vec<DoctorCapabilityIssue>,
    pub(crate) capabilities: Vec<yoctui_protocol::daemon::CompatibilityCapabilityData>,
}

pub(crate) fn doctor_compatibility_report(
    snapshot: Option<&yoctui_protocol::daemon::CompatibilitySnapshotData>,
    unavailable_reason: Option<&str>,
) -> DoctorCompatibilityReport {
    use yoctui_protocol::daemon::{
        CompatibilityEvidenceKind, CompatibilityEvidenceOutcome, CompatibilityStateData,
    };

    let unavailable = |authority, reason: String| DoctorCompatibilityReport {
        schema: "yoctui.doctor.compatibility.v1",
        authority,
        authority_reason: Some(reason),
        release_support: DoctorReleaseSupport::Unknown,
        release_support_reason:
            "No current live-release support classification is present in daemon authority.".into(),
        operating_mode: None,
        schema_version: None,
        generation: None,
        environment: None,
        summary: DoctorCompatibilitySummary::default(),
        missing_tools: Vec::new(),
        limited_features: Vec::new(),
        unavailable_features: Vec::new(),
        unsupported_features: Vec::new(),
        unknown_features: Vec::new(),
        capabilities: Vec::new(),
    };
    let Some(snapshot) = snapshot else {
        return unavailable(
            DoctorCompatibilityAuthority::Unavailable,
            unavailable_reason
                .unwrap_or("Daemon snapshot has no compatibility authority.")
                .to_owned(),
        );
    };
    if let Err(error) = snapshot.validate() {
        return unavailable(
            DoctorCompatibilityAuthority::Invalid,
            format!("Daemon compatibility authority failed validation: {error}"),
        );
    }
    if snapshot.capabilities.iter().any(|capability| {
        matches!(capability.state, CompatibilityStateData::UnknownWireState)
            || capability.evidence.iter().any(|evidence| {
                evidence.kind == CompatibilityEvidenceKind::Unknown
                    || evidence.outcome == CompatibilityEvidenceOutcome::Unknown
            })
    }) {
        return unavailable(
            DoctorCompatibilityAuthority::Invalid,
            "Daemon compatibility authority contains unknown protocol values.".into(),
        );
    }

    let mut summary = DoctorCompatibilitySummary::default();
    let mut missing_tools = BTreeMap::<String, String>::new();
    let mut limited_features = Vec::new();
    let mut unavailable_features = Vec::new();
    let mut unsupported_features = Vec::new();
    let mut unknown_features = Vec::new();
    for capability in &snapshot.capabilities {
        for evidence in &capability.evidence {
            if evidence.kind == CompatibilityEvidenceKind::ExecutableIdentity
                && evidence.outcome == CompatibilityEvidenceOutcome::Negative
            {
                missing_tools
                    .entry(evidence.subject.clone())
                    .or_insert_with(|| evidence.detail.clone());
            }
        }
        let (state, reason, limitations, destination) = match &capability.state {
            CompatibilityStateData::Available => {
                summary.available += 1;
                continue;
            }
            CompatibilityStateData::AvailableWithLimitations {
                reason,
                limitations,
            } => {
                summary.limited += 1;
                (
                    "limited",
                    reason,
                    limitations.clone(),
                    &mut limited_features,
                )
            }
            CompatibilityStateData::Unavailable { reason } => {
                summary.unavailable += 1;
                ("unavailable", reason, Vec::new(), &mut unavailable_features)
            }
            CompatibilityStateData::Unknown { reason } => {
                summary.unknown += 1;
                ("unknown", reason, Vec::new(), &mut unknown_features)
            }
            CompatibilityStateData::Unsupported { reason } => {
                summary.unsupported += 1;
                ("unsupported", reason, Vec::new(), &mut unsupported_features)
            }
            CompatibilityStateData::UnknownWireState => unreachable!("rejected above"),
        };
        destination.push(DoctorCapabilityIssue {
            id: capability.id.clone(),
            state: state.into(),
            reason_code: reason.code.clone(),
            reason: reason.message.clone(),
            requirement: reason.requirement.clone(),
            limitations,
            implementation: capability
                .implementation
                .as_ref()
                .map(|implementation| implementation.id.clone()),
        });
    }
    let operating_mode = if summary.unavailable == 0
        && summary.unknown == 0
        && summary.unsupported == 0
        && summary.limited == 0
    {
        DoctorCompatibilityMode::Full
    } else if summary.available + summary.limited > 0 {
        DoctorCompatibilityMode::Degraded
    } else {
        DoctorCompatibilityMode::Diagnostic
    };
    DoctorCompatibilityReport {
        schema: "yoctui.doctor.compatibility.v1",
        authority: DoctorCompatibilityAuthority::Current,
        authority_reason: None,
        release_support: DoctorReleaseSupport::Unknown,
        release_support_reason:
            "Runtime capability evidence is current; no live release-support claim is encoded yet."
                .into(),
        operating_mode: Some(operating_mode),
        schema_version: Some(snapshot.schema_version),
        generation: Some(snapshot.generation),
        environment: Some(snapshot.environment.clone()),
        summary,
        missing_tools: missing_tools
            .into_iter()
            .map(|(tool, reason)| DoctorMissingTool { tool, reason })
            .collect(),
        limited_features,
        unavailable_features,
        unsupported_features,
        unknown_features,
        capabilities: snapshot.capabilities.clone(),
    }
}

pub(crate) fn doctor_detected<T>(
    detected: &yoctui_protocol::daemon::CompatibilityDetected<T>,
    render: impl FnOnce(&T) -> String,
) -> String {
    match detected {
        yoctui_protocol::daemon::CompatibilityDetected::Unknown => "unknown".into(),
        yoctui_protocol::daemon::CompatibilityDetected::Detected { value, authority } => {
            format!("{} [{authority:?}]", render(value))
        }
    }
}
