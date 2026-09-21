use super::*;
use crate::{
    AuthoritativeValue, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementationKind, CapabilityRecord, CapabilitySnapshot, IdentityAuthority,
};
use std::collections::BTreeMap;

fn reason(code: &str, message: &str, requirement: Option<&str>) -> CapabilityReason {
    CapabilityReason::new(code, message, requirement.map(str::to_owned)).unwrap()
}

fn evidence(outcome: CapabilityEvidenceOutcome, subject: &str) -> CapabilityEvidence {
    CapabilityEvidence {
        kind: CapabilityEvidenceKind::DirectProbe,
        outcome,
        subject: subject.into(),
        detail: format!("{subject} fixture evidence"),
        argv: vec![subject.into(), "--help".into()],
    }
}

fn authority(generation: u64) -> DaemonCompatibilitySnapshot {
    let records = vec![
        CapabilityRecord {
            id: CapabilityId::BitBakeBuild,
            state: CapabilityState::Available,
            evidence: vec![evidence(CapabilityEvidenceOutcome::Positive, "bitbake")],
        },
        CapabilityRecord {
            id: CapabilityId::BitBakeGetVar,
            state: CapabilityState::AvailableWithLimitations {
                reason: reason(
                    "compatibility.fallback",
                    "Native getvar is absent; the environment dump fallback is selected.",
                    Some("bitbake -e"),
                ),
                limitations: vec!["The fallback parses a complete environment dump.".into()],
            },
            evidence: vec![evidence(CapabilityEvidenceOutcome::Positive, "bitbake -e")],
        },
        CapabilityRecord {
            id: CapabilityId::DevtoolUpgrade,
            state: CapabilityState::Unavailable {
                reason: reason(
                    "probe.subcommand_absent",
                    "Current Devtool does not expose the upgrade subcommand.",
                    Some("devtool upgrade"),
                ),
            },
            evidence: vec![evidence(CapabilityEvidenceOutcome::Negative, "devtool")],
        },
        CapabilityRecord {
            id: CapabilityId::ResultTool,
            state: CapabilityState::Unknown {
                reason: reason(
                    "probe.timed_out",
                    "The resulttool probe timed out.",
                    Some("resulttool --help"),
                ),
            },
            evidence: vec![evidence(
                CapabilityEvidenceOutcome::Inconclusive,
                "resulttool",
            )],
        },
        CapabilityRecord {
            id: CapabilityId::GitArchive,
            state: CapabilityState::Unsupported {
                reason: reason(
                    "yoctui.not_implemented",
                    "Yoctui does not maintain this environment adapter.",
                    None,
                ),
            },
            evidence: Vec::new(),
        },
    ];
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    "/work/poky/build".into(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                bitbake_version: AuthoritativeValue::detected(
                    "2.18.0".into(),
                    IdentityAuthority::BitBakeVersionProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: records,
        },
        implementations: BTreeMap::from([
            (
                CapabilityId::BitBakeBuild,
                CapabilityImplementation {
                    id: "bitbake.build.command".into(),
                    kind: CapabilityImplementationKind::Command,
                },
            ),
            (
                CapabilityId::BitBakeGetVar,
                CapabilityImplementation {
                    id: "bitbake.getvar.environment-fallback".into(),
                    kind: CapabilityImplementationKind::Command,
                },
            ),
        ]),
    }
    .normalize()
    .unwrap()
}

fn state_with(authority: DaemonCompatibilitySnapshot) -> WorkspaceCompatibilityState {
    let mut state = WorkspaceCompatibilityState::default();
    state.install(authority).unwrap();
    state
}

mod compatibility_ui_model_projects_identity_summary_states_reasons_and_evidence;

mod compatibility_ui_model_absent_authority_is_explicit_for_every_replica_state;

mod compatibility_dynamic_model_filter_search_and_selection_reconcile_by_stable_id;

mod compatibility_ui_model_query_is_bounded_and_rejects_control_input;

mod compatibility_ui_model_action_availability_preserves_exact_workspace_projection;

mod compatibility_ui_action_catalog_classifies_every_destination_and_command;

mod compatibility_ui_action_catalog_preserves_inspection_gating_and_exact_fallback;

mod compatibility_ui_action_catalog_reuses_effect_and_dialog_authority;

mod compatibility_ui_workspace_actions_catalog_is_unique_closed_and_probe_free;

mod compatibility_ui_workspace_actions_project_all_states_without_local_inference;
