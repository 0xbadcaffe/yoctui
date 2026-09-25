use super::*;
use crate::{CompatibilityFixtureRole, release_capability_fixtures};
use std::fs;
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementation, CapabilityImplementationKind, CapabilityReason, CapabilityRecord,
    CapabilitySnapshot, CapabilityState, IdentityAuthority, ToolIdentity, YoctoEnvironmentIdentity,
};

fn authority(
    build: &Path,
    executable: &Path,
    generation: u64,
    records: &[(CapabilityId, &str, bool)],
) -> DaemonCompatibilitySnapshot {
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    build.to_owned(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: AuthoritativeValue::detected(
                    vec![ToolIdentity {
                        id: "bitbake-layers".into(),
                        executable: executable.to_owned(),
                        version: None,
                    }],
                    IdentityAuthority::ExecutableProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: records
                .iter()
                .map(|(id, _, available)| CapabilityRecord {
                    id: *id,
                    state: if *available {
                        CapabilityState::Available
                    } else {
                        CapabilityState::Unavailable {
                            reason: CapabilityReason::new(
                                "bitbake_layers.behavior_missing",
                                format!("Current bitbake-layers does not expose {}.", id.as_str()),
                                Some(format!("Required capability: {}", id.as_str())),
                            )
                            .unwrap(),
                        }
                    },
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: if *available {
                            CapabilityEvidenceOutcome::Positive
                        } else {
                            CapabilityEvidenceOutcome::Negative
                        },
                        subject: format!("{} fixture probe", id.as_str()),
                        detail: "The exact initialized Layers behavior was inspected.".into(),
                        argv: vec![executable.display().to_string(), "--help".into()],
                    }],
                })
                .collect(),
        },
        implementations: records
            .iter()
            .filter(|(_, _, available)| *available)
            .map(|(id, implementation, _)| {
                (
                    *id,
                    CapabilityImplementation {
                        id: (*implementation).into(),
                        kind: CapabilityImplementationKind::Command,
                    },
                )
            })
            .collect(),
    }
    .normalize()
    .unwrap()
}

fn all_records() -> [(CapabilityId, &'static str, bool); 7] {
    [
        (
            CapabilityId::BitBakeLayersShowLayers,
            BITBAKE_LAYERS_SHOW_IMPLEMENTATION,
            true,
        ),
        (
            CapabilityId::BitBakeLayersShowRecipes,
            BITBAKE_LAYERS_SHOW_RECIPES_IMPLEMENTATION,
            true,
        ),
        (
            CapabilityId::BitBakeLayersShowOverlayed,
            BITBAKE_LAYERS_SHOW_OVERLAYED_IMPLEMENTATION,
            true,
        ),
        (
            CapabilityId::BitBakeLayersCreateLayer,
            BITBAKE_LAYERS_CREATE_IMPLEMENTATION,
            true,
        ),
        (
            CapabilityId::BitBakeLayersCreateAndAddLayer,
            BITBAKE_LAYERS_CREATE_ADD_IMPLEMENTATION,
            true,
        ),
        (
            CapabilityId::BitBakeLayersAddLayer,
            BITBAKE_LAYERS_ADD_IMPLEMENTATION,
            true,
        ),
        (
            CapabilityId::BitBakeLayersRemoveLayer,
            BITBAKE_LAYERS_REMOVE_IMPLEMENTATION,
            true,
        ),
    ]
}

mod compatibility_command_shared_fixtures_gate_layer_option_before_argv;

mod compatibility_layers_generates_exact_argv_for_every_operation;

mod compatibility_layers_old_surface_disables_only_absent_mutations_and_options;

mod compatibility_layers_rejects_stale_environment_executable_and_cross_command;

#[cfg(unix)]
mod compatibility_layers_unavailable_mutation_never_spawns_process;
