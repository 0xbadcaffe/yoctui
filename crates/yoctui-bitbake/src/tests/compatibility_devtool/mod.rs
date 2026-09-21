use super::*;
use crate::{CompatibilityFixtureRole, release_capability_fixtures};
use std::{collections::BTreeMap, fs, path::PathBuf};
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
                subject: format!("devtool {} --help", id.as_str()),
                detail: "The exact initialized Devtool subcommand was observed.".into(),
                argv: vec![executable.display().to_string(), "--help".into()],
            }],
        })
        .collect::<Vec<_>>();
    capabilities.extend(unavailable.iter().map(|id| {
        CapabilityRecord {
            id: *id,
            state: CapabilityState::Unavailable {
                reason: CapabilityReason::new(
                    "devtool.subcommand_missing",
                    format!("Current Devtool does not expose {}.", id.as_str()),
                    Some(format!("Required capability: {}", id.as_str())),
                )
                .unwrap(),
            },
            evidence: vec![CapabilityEvidence {
                kind: CapabilityEvidenceKind::DirectProbe,
                outcome: CapabilityEvidenceOutcome::Negative,
                subject: format!("devtool {} --help", id.as_str()),
                detail: "The exact initialized Devtool subcommand is absent.".into(),
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
                        id: "devtool".into(),
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

fn all_operations() -> Vec<(DevtoolOperation, CapabilityId, &'static [&'static str])> {
    vec![
        (
            DevtoolOperation::Modify {
                recipe: "busybox".into(),
            },
            CapabilityId::DevtoolModify,
            &["modify", "busybox"],
        ),
        (
            DevtoolOperation::UpdateRecipe {
                recipe: "busybox".into(),
            },
            CapabilityId::DevtoolUpdateRecipe,
            &["update-recipe", "busybox"],
        ),
        (
            DevtoolOperation::Finish {
                recipe: "busybox".into(),
                destination: PathBuf::from("/layers/meta-custom"),
            },
            CapabilityId::DevtoolFinish,
            &["finish", "busybox", "/layers/meta-custom"],
        ),
        (
            DevtoolOperation::DeployTarget {
                recipe: "busybox".into(),
                target: "root@board".into(),
            },
            CapabilityId::DevtoolDeployTarget,
            &["deploy-target", "busybox", "root@board"],
        ),
        (
            DevtoolOperation::UndeployTarget {
                recipe: "busybox".into(),
                target: "root@board".into(),
            },
            CapabilityId::DevtoolUndeployTarget,
            &["undeploy-target", "busybox", "root@board"],
        ),
        (
            DevtoolOperation::Reset {
                recipe: "busybox".into(),
            },
            CapabilityId::DevtoolReset,
            &["reset", "busybox"],
        ),
        (
            DevtoolOperation::Upgrade {
                recipe: "busybox".into(),
            },
            CapabilityId::DevtoolUpgrade,
            &["upgrade", "busybox"],
        ),
    ]
}

mod compatibility_command_shared_fixtures_gate_devtool_upgrade_before_argv;

mod compatibility_devtool_generates_exact_argv_for_each_independently_probed_subcommand;

mod compatibility_devtool_old_surface_disables_only_absent_upgrade_with_exact_reason;

mod compatibility_devtool_rejects_stale_environment_executable_and_cross_subcommand_authority;

#[cfg(unix)]
mod compatibility_devtool_unavailable_subcommand_never_spawns_process;
