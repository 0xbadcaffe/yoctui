use super::*;
use crate::CapabilityProbeStatus;
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidenceKind, CapabilityEvidenceOutcome, IdentityAuthority,
    ReleaseIdentity,
};

fn observation(status: CapabilityProbeStatus, subject: &str) -> CapabilityProbeObservation {
    CapabilityProbeObservation {
        status,
        evidence: CapabilityEvidence {
            kind: CapabilityEvidenceKind::DirectProbe,
            outcome: match status {
                CapabilityProbeStatus::Positive => CapabilityEvidenceOutcome::Positive,
                CapabilityProbeStatus::Negative => CapabilityEvidenceOutcome::Negative,
                CapabilityProbeStatus::Inconclusive => CapabilityEvidenceOutcome::Inconclusive,
            },
            subject: subject.into(),
            detail: format!("synthetic future observation: {subject}"),
            argv: Vec::new(),
        },
    }
}

fn complete_observations(
    catalog: &CapabilityCatalog,
    id: CapabilityId,
    status: CapabilityProbeStatus,
    subject: &str,
) -> Vec<CapabilityProbeObservation> {
    vec![observation(status, subject); catalog.entry(id).unwrap().probes.len()]
}

fn future_environment() -> YoctoEnvironmentIdentity {
    YoctoEnvironmentIdentity {
        build_directory: AuthoritativeValue::detected(
            "/future/build".into(),
            IdentityAuthority::InitializedEnvironment,
        ),
        bitbake_version: AuthoritativeValue::detected(
            "99.0.0".into(),
            IdentityAuthority::BitBakeVersionProbe,
        ),
        oe_core: AuthoritativeValue::detected(
            ReleaseIdentity {
                name: Some("future-series".into()),
                version: Some("99.0".into()),
            },
            IdentityAuthority::ReleaseMetadata,
        ),
        ..YoctoEnvironmentIdentity::default()
    }
}

mod compatibility_future_unknown_enables_only_positive_exact_capabilities;

mod compatibility_future_unknown_direct_probe_overrides_closed_static_boundary;

mod compatibility_probe_aggregation_keeps_partial_compound_evidence_disabled;

mod compatibility_probe_aggregation_allows_fallback_only_after_complete_inconclusive_probe;

mod compatibility_future_unknown_absent_and_inconclusive_evidence_stays_unknown;

mod compatibility_older_release_preserves_core_selects_fallback_and_disables_newer_feature;

mod compatibility_command_getvar_prefers_direct_utility_and_uses_environment_capability_fallback;
