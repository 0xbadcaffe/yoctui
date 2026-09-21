use super::*;
use crate::{
    CapabilityCatalog, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementation, CapabilityImplementationKind, CapabilityProbeSpec, CapabilityReason,
    CapabilityRecord, CapabilitySnapshot, CapabilityToolId, YoctoEnvironmentIdentity,
};
use std::collections::BTreeMap;

fn reason(message: &str) -> CapabilityReason {
    CapabilityReason::new("test.raw", message, None).unwrap()
}

fn authority(
    records: Vec<(CapabilityId, CapabilityState, Option<&str>)>,
) -> DaemonCompatibilitySnapshot {
    let capabilities = records
        .iter()
        .map(|(id, state, _)| CapabilityRecord {
            id: *id,
            state: state.clone(),
            evidence: match state {
                CapabilityState::Available | CapabilityState::AvailableWithLimitations { .. } => {
                    vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: id.as_str().into(),
                        detail: "positive Raw fixture evidence".into(),
                        argv: vec!["fixture".into()],
                    }]
                }
                CapabilityState::Unavailable { .. } => vec![CapabilityEvidence {
                    kind: CapabilityEvidenceKind::DirectProbe,
                    outcome: CapabilityEvidenceOutcome::Negative,
                    subject: id.as_str().into(),
                    detail: "negative Raw fixture evidence".into(),
                    argv: vec!["fixture".into()],
                }],
                CapabilityState::Unknown { .. } | CapabilityState::Unsupported { .. } => Vec::new(),
            },
        })
        .collect();
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation: 1,
            environment: YoctoEnvironmentIdentity::default(),
            capabilities,
        },
        implementations: records
            .into_iter()
            .filter_map(|(id, _, implementation)| {
                implementation.map(|implementation| {
                    (
                        id,
                        CapabilityImplementation {
                            id: implementation.into(),
                            kind: CapabilityImplementationKind::Command,
                        },
                    )
                })
            })
            .collect::<BTreeMap<_, _>>(),
    }
    .normalize()
    .unwrap()
}

fn command(line: usize) -> RawCommand {
    RawCatalog::builtin()
        .commands
        .into_iter()
        .find(|command| command.reference.id.as_str() == format!("wrynose-6-0.l{line:04}"))
        .unwrap()
}

fn with_requirement(requirement: RawCapabilityRequirement) -> RawCommand {
    let mut command = command(167);
    let RawExecutionPolicy::Executable { template } = &mut command.execution else {
        unreachable!()
    };
    template.capabilities = requirement;
    command
}

mod raw_capability_builtin_commands_have_explicit_fail_closed_requirements;

mod raw_capability_all_of_preserves_available_and_limited_reasons;

mod raw_capability_any_of_prefers_fully_available_implementation;

mod raw_capability_preserves_unavailable_unknown_and_unsupported_states;

mod raw_capability_missing_record_is_unknown_without_version_inference;

mod raw_capability_probe_catalog_is_direct_bounded_and_version_agnostic;

mod raw_capability_probe_maps_representative_options_exactly;
