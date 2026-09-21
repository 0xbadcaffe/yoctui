use super::*;
use crate::{CompatibilityFixtureRole, release_capability_fixtures};
use std::{collections::BTreeMap, fs};
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementation, CapabilityImplementationKind, CapabilityReason, CapabilityRecord,
    CapabilitySnapshot, CapabilityState, IdentityAuthority, ToolIdentity, YoctoEnvironmentIdentity,
};

fn authority(
    build: &Path,
    executable: &Path,
    generation: u64,
    available: &[(CapabilityId, &str)],
    unavailable: &[CapabilityId],
) -> DaemonCompatibilitySnapshot {
    let mut capabilities = available
        .iter()
        .map(|(id, _)| CapabilityRecord {
            id: *id,
            state: CapabilityState::Available,
            evidence: vec![CapabilityEvidence {
                kind: CapabilityEvidenceKind::DirectProbe,
                outcome: CapabilityEvidenceOutcome::Positive,
                subject: format!("{} fixture probe", id.as_str()),
                detail: "The exact initialized Recipetool behavior was observed.".into(),
                argv: vec![executable.display().to_string(), "--help".into()],
            }],
        })
        .collect::<Vec<_>>();
    capabilities.extend(unavailable.iter().map(|id| {
        CapabilityRecord {
            id: *id,
            state: CapabilityState::Unavailable {
                reason: CapabilityReason::new(
                    "recipetool.behavior_missing",
                    format!("Current Recipetool does not expose {}.", id.as_str()),
                    Some(format!("Required capability: {}", id.as_str())),
                )
                .unwrap(),
            },
            evidence: vec![CapabilityEvidence {
                kind: CapabilityEvidenceKind::DirectProbe,
                outcome: CapabilityEvidenceOutcome::Negative,
                subject: format!("{} fixture probe", id.as_str()),
                detail: "The exact initialized Recipetool behavior is absent.".into(),
                argv: vec![executable.display().to_string(), "--help".into()],
            }],
        }
    }));
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
                        id: "recipetool".into(),
                        executable: executable.to_owned(),
                        version: None,
                    }],
                    IdentityAuthority::ExecutableProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities,
        },
        implementations: available
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

fn create() -> RecipetoolOperation {
    RecipetoolOperation::Create {
        source: "https://example.invalid/demo.tar.gz".into(),
        outfile: "/layers/meta-demo/recipes-demo/demo.bb".into(),
    }
}

fn appendfile() -> RecipetoolOperation {
    RecipetoolOperation::AppendFile {
        destination_layer: "/layers/meta-demo".into(),
        target_path: "/etc/motd".into(),
        replacement_file: "/work/motd".into(),
    }
}

mod compatibility_command_shared_fixtures_gate_recipetool_outfile_before_argv;

mod compatibility_recipetool_generates_exact_create_and_appendfile_argv;

mod compatibility_recipetool_old_surface_keeps_appendfile_but_rejects_missing_outfile;

mod compatibility_recipetool_rejects_stale_environment_executable_and_cross_subcommand;

#[cfg(unix)]
mod compatibility_recipetool_unavailable_option_never_spawns_process;
