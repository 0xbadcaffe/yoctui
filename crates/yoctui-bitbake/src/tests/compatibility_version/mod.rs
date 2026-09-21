use super::*;
use yoctui_model::{CapabilityCatalog, CapabilityProbeSpec};

fn entry(id: CapabilityId) -> CapabilityCatalogEntry {
    CapabilityCatalog::builtin().entry(id).unwrap().clone()
}

fn direct(status: CapabilityProbeStatus) -> CapabilityProbeObservation {
    CapabilityProbeObservation {
        status,
        evidence: CapabilityEvidence {
            kind: CapabilityEvidenceKind::DirectProbe,
            outcome: match status {
                CapabilityProbeStatus::Positive => CapabilityEvidenceOutcome::Positive,
                CapabilityProbeStatus::Negative => CapabilityEvidenceOutcome::Negative,
                CapabilityProbeStatus::Inconclusive => CapabilityEvidenceOutcome::Inconclusive,
            },
            subject: "backend handshake".into(),
            detail: "fixture observation".into(),
            argv: Vec::new(),
        },
    }
}

mod compatibility_version_parser_compares_numeric_components_and_suffixes;

mod compatibility_version_selects_old_and_new_adapter_ranges_only_when_declared;

mod compatibility_version_direct_evidence_overrides_static_inference;

mod compatibility_version_unknown_malformed_and_future_versions_fail_closed;

mod compatibility_probe_aggregation_requires_every_direct_requirement;

mod compatibility_version_catalog_fallback_is_not_an_executable_probe;
