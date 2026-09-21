use super::*;
use crate::{CompatibilityFixtureRole, release_capability_fixtures};
use std::{collections::BTreeMap, path::PathBuf};
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementation, CapabilityImplementationKind, CapabilityReason, CapabilityRecord,
    CapabilitySnapshot, CapabilityState, IdentityAuthority, ToolIdentity, YoctoEnvironmentIdentity,
};

fn authority(
    generation: u64,
    capabilities: &[(CapabilityId, &str)],
) -> DaemonCompatibilitySnapshot {
    let environment = YoctoEnvironmentIdentity {
        build_directory: AuthoritativeValue::detected(
            PathBuf::from("/work/build"),
            IdentityAuthority::InitializedEnvironment,
        ),
        available_tools: AuthoritativeValue::detected(
            [
                ("bitbake", "bitbake"),
                ("bitbake-getvar", "bitbake-getvar"),
                ("bitbake-diffsigs", "bitbake-diffsigs"),
                ("bitbake-dumpsig", "bitbake-dumpsig"),
            ]
            .into_iter()
            .map(|(id, executable)| ToolIdentity {
                id: id.into(),
                executable: format!("/work/bin/{executable}").into(),
                version: None,
            })
            .collect(),
            IdentityAuthority::ExecutableProbe,
        ),
        ..YoctoEnvironmentIdentity::default()
    };
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation,
            environment,
            capabilities: capabilities
                .iter()
                .map(|(id, _)| CapabilityRecord {
                    id: *id,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: format!("{} command probe", id.as_str()),
                        detail: "Required command and options were observed directly.".into(),
                        argv: vec!["bitbake".into(), "--help".into()],
                    }],
                })
                .collect(),
        },
        implementations: capabilities
            .iter()
            .map(|(id, implementation)| {
                (
                    *id,
                    CapabilityImplementation {
                        id: (*implementation).into(),
                        kind: CapabilityImplementationKind::Command,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
    }
    .normalize()
    .unwrap()
}

fn planner(authority: &DaemonCompatibilitySnapshot) -> BitBakeCommandPlanner<'_> {
    BitBakeCommandPlanner::new(
        authority,
        authority.snapshot.generation,
        Path::new("/work/build"),
    )
    .unwrap()
}

fn fixture_authority(
    role: CompatibilityFixtureRole,
    generation: u64,
) -> DaemonCompatibilitySnapshot {
    release_capability_fixtures()
        .into_iter()
        .find(|fixture| fixture.role == role)
        .unwrap()
        .command_authority(generation)
}

fn fixture_planner(authority: &DaemonCompatibilitySnapshot) -> BitBakeCommandPlanner<'_> {
    BitBakeCommandPlanner::new(
        authority,
        authority.snapshot.generation,
        authority
            .snapshot
            .environment
            .build_directory
            .value()
            .unwrap(),
    )
    .unwrap()
}

mod compatibility_command_getvar_shared_fixtures_select_fallback_and_utility_argv;

mod compatibility_command_getvar_generates_old_and_new_argv_without_unsupported_option;

mod compatibility_command_emits_only_authorized_build_graph_and_server_options;

mod compatibility_command_rejects_unavailable_stale_and_other_environment_before_argv;

#[cfg(unix)]
mod compatibility_command_unavailable_action_is_rejected_before_process_spawn;
