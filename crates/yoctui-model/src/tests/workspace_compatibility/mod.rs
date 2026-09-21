use super::*;
use crate::{
    BuildRequest, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementation, CapabilityImplementationKind, CapabilityReason, CapabilityRecord,
    CapabilitySnapshot, CapabilityState, DaemonCompatibilitySnapshot, Effect, VariableIdentity,
    YoctoEnvironmentIdentity,
};
use std::collections::BTreeMap;

fn reason(message: &str) -> CapabilityReason {
    CapabilityReason::new("test.workspace", message, None).unwrap()
}

fn authority(
    generation: u64,
    records: Vec<(CapabilityId, CapabilityState, Option<&str>)>,
) -> DaemonCompatibilitySnapshot {
    let mut capabilities = records
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
                        detail: "positive workspace fixture evidence".into(),
                        argv: vec!["fixture".into()],
                    }]
                }
                CapabilityState::Unavailable { .. } => vec![CapabilityEvidence {
                    kind: CapabilityEvidenceKind::DirectProbe,
                    outcome: CapabilityEvidenceOutcome::Negative,
                    subject: id.as_str().into(),
                    detail: "negative workspace fixture evidence".into(),
                    argv: vec!["fixture".into()],
                }],
                CapabilityState::Unknown { .. } | CapabilityState::Unsupported { .. } => Vec::new(),
            },
        })
        .collect::<Vec<_>>();
    capabilities.sort_by_key(|record| record.id);
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation,
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

mod compatibility_workspace_catalog_covers_every_screen_and_named_destination;

mod raw_navigation_uses_the_authoritative_raw_cli_requirement;

mod compatibility_workspace_catalog_classifies_local_single_all_and_alternative_effects;

mod compatibility_workspace_catalog_marks_environment_inspection_daemon_owned;

mod compatibility_workspace_catalog_new_external_behaviors_have_exact_probe_policy;

mod compatibility_workspace_model_projects_full_limited_all_and_any_requirements;

mod compatibility_workspace_model_absent_unknown_unsupported_and_missing_all_fail_closed;

mod compatibility_dynamic_model_snapshot_install_is_monotonic_and_conflict_safe;

mod compatibility_dynamic_model_snapshot_change_revalidates_dialog_and_effect;

mod compatibility_dynamic_model_invalidation_closes_environment_dialog_but_keeps_local_one;

mod compatibility_dynamic_model_unavailable_effect_is_not_emitted_or_partially_applied;
