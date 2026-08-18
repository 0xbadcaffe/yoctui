use std::{ffi::OsString, path::Path};

use thiserror::Error;
use yoctui_model::{CapabilityId, DaemonCompatibilitySnapshot, DevtoolOperation};

use crate::DevtoolCommandSpec;

pub const DEVTOOL_STATUS_IMPLEMENTATION: &str = "devtool.status.argv";
pub const DEVTOOL_EDIT_RECIPE_IMPLEMENTATION: &str = "devtool.edit_recipe.argv";
pub const DEVTOOL_MODIFY_IMPLEMENTATION: &str = "devtool.modify.argv";
pub const DEVTOOL_UPDATE_RECIPE_IMPLEMENTATION: &str = "devtool.update_recipe.argv";
pub const DEVTOOL_FINISH_IMPLEMENTATION: &str = "devtool.finish.argv";
pub const DEVTOOL_DEPLOY_TARGET_IMPLEMENTATION: &str = "devtool.deploy_target.argv";
pub const DEVTOOL_UNDEPLOY_TARGET_IMPLEMENTATION: &str = "devtool.undeploy_target.argv";
pub const DEVTOOL_RESET_IMPLEMENTATION: &str = "devtool.reset.argv";
pub const DEVTOOL_UPGRADE_IMPLEMENTATION: &str = "devtool.upgrade.argv";

pub struct DevtoolCommandPlanner<'a> {
    authority: &'a DaemonCompatibilitySnapshot,
    build_directory: &'a Path,
    executable: &'a Path,
}

impl<'a> DevtoolCommandPlanner<'a> {
    pub fn new(
        authority: &'a DaemonCompatibilitySnapshot,
        expected_generation: u64,
        build_directory: &'a Path,
        executable: &'a Path,
    ) -> Result<Self, DevtoolCompatibilityError> {
        if authority.snapshot.generation != expected_generation {
            return Err(DevtoolCompatibilityError::StaleGeneration {
                expected: expected_generation,
                actual: authority.snapshot.generation,
            });
        }
        if authority
            .snapshot
            .environment
            .build_directory
            .value()
            .map(std::path::PathBuf::as_path)
            != Some(build_directory)
        {
            return Err(DevtoolCompatibilityError::EnvironmentMismatch);
        }
        let detected = authority
            .snapshot
            .environment
            .available_tools
            .value()
            .and_then(|tools| tools.iter().find(|tool| tool.id == "devtool"))
            .ok_or(DevtoolCompatibilityError::ToolIdentityUnknown)?;
        if detected.executable != executable {
            return Err(DevtoolCompatibilityError::ExecutableMismatch);
        }
        Ok(Self {
            authority,
            build_directory,
            executable,
        })
    }

    pub fn status(&self) -> Result<DevtoolCommandSpec, DevtoolCompatibilityError> {
        self.command(
            CapabilityId::DevtoolStatus,
            DEVTOOL_STATUS_IMPLEMENTATION,
            vec!["status".into()],
        )
    }

    pub fn edit_recipe(
        &self,
        recipe: &str,
    ) -> Result<DevtoolCommandSpec, DevtoolCompatibilityError> {
        validate_token(recipe, "recipe")?;
        self.command(
            CapabilityId::DevtoolEditRecipe,
            DEVTOOL_EDIT_RECIPE_IMPLEMENTATION,
            vec!["edit-recipe".into(), recipe.into()],
        )
    }

    pub fn operation(
        &self,
        operation: &DevtoolOperation,
    ) -> Result<DevtoolCommandSpec, DevtoolCompatibilityError> {
        operation
            .validate()
            .map_err(|error| DevtoolCompatibilityError::InvalidRequest(error.to_string()))?;
        let recipe = OsString::from(operation.recipe());
        let (capability, implementation, arguments) = match operation {
            DevtoolOperation::Modify { .. } => (
                CapabilityId::DevtoolModify,
                DEVTOOL_MODIFY_IMPLEMENTATION,
                vec!["modify".into(), recipe],
            ),
            DevtoolOperation::UpdateRecipe { .. } => (
                CapabilityId::DevtoolUpdateRecipe,
                DEVTOOL_UPDATE_RECIPE_IMPLEMENTATION,
                vec!["update-recipe".into(), recipe],
            ),
            DevtoolOperation::Finish { destination, .. } => (
                CapabilityId::DevtoolFinish,
                DEVTOOL_FINISH_IMPLEMENTATION,
                vec!["finish".into(), recipe, destination.as_os_str().to_owned()],
            ),
            DevtoolOperation::DeployTarget { target, .. } => (
                CapabilityId::DevtoolDeployTarget,
                DEVTOOL_DEPLOY_TARGET_IMPLEMENTATION,
                vec!["deploy-target".into(), recipe, target.into()],
            ),
            DevtoolOperation::UndeployTarget { target, .. } => (
                CapabilityId::DevtoolUndeployTarget,
                DEVTOOL_UNDEPLOY_TARGET_IMPLEMENTATION,
                vec!["undeploy-target".into(), recipe, target.into()],
            ),
            DevtoolOperation::Reset { .. } => (
                CapabilityId::DevtoolReset,
                DEVTOOL_RESET_IMPLEMENTATION,
                vec!["reset".into(), recipe],
            ),
            DevtoolOperation::Upgrade { .. } => (
                CapabilityId::DevtoolUpgrade,
                DEVTOOL_UPGRADE_IMPLEMENTATION,
                vec!["upgrade".into(), recipe],
            ),
        };
        self.command(capability, implementation, arguments)
    }

    fn command(
        &self,
        capability: CapabilityId,
        implementation: &str,
        arguments: Vec<OsString>,
    ) -> Result<DevtoolCommandSpec, DevtoolCompatibilityError> {
        let record = self
            .authority
            .snapshot
            .capability(capability)
            .ok_or(DevtoolCompatibilityError::CapabilityMissing { capability })?;
        if !record.state.is_enabled() {
            return Err(DevtoolCompatibilityError::Unavailable {
                capability,
                reason: record
                    .state
                    .reason()
                    .map(|reason| reason.message.clone())
                    .unwrap_or_else(|| {
                        "No positive Devtool capability evidence is available.".into()
                    }),
            });
        }
        let selected = self
            .authority
            .implementations
            .get(&capability)
            .ok_or(DevtoolCompatibilityError::ImplementationMissing { capability })?;
        if selected.id != implementation {
            return Err(DevtoolCompatibilityError::ImplementationMismatch {
                capability,
                selected: selected.id.clone(),
                required: implementation.into(),
            });
        }
        Ok(DevtoolCommandSpec::from_authorized_parts(
            self.executable.to_owned(),
            arguments,
            self.authority.snapshot.generation,
            capability,
            self.build_directory.to_owned(),
        ))
    }
}

fn validate_token(value: &str, field: &'static str) -> Result<(), DevtoolCompatibilityError> {
    if value.is_empty()
        || value.starts_with('-')
        || value
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(DevtoolCompatibilityError::InvalidToken { field });
    }
    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DevtoolCompatibilityError {
    #[error("stale Devtool capability generation: expected {expected}, got {actual}")]
    StaleGeneration { expected: u64, actual: u64 },
    #[error("Devtool capability snapshot belongs to another build environment")]
    EnvironmentMismatch,
    #[error("Devtool executable identity is unknown in the initialized environment")]
    ToolIdentityUnknown,
    #[error("Devtool executable does not match the initialized-environment tool identity")]
    ExecutableMismatch,
    #[error("Devtool capability {capability:?} is missing")]
    CapabilityMissing { capability: CapabilityId },
    #[error("Devtool capability {capability:?} is unavailable: {reason}")]
    Unavailable {
        capability: CapabilityId,
        reason: String,
    },
    #[error("Devtool capability {capability:?} has no selected implementation")]
    ImplementationMissing { capability: CapabilityId },
    #[error(
        "Devtool capability {capability:?} selected {selected}, not required implementation {required}"
    )]
    ImplementationMismatch {
        capability: CapabilityId,
        selected: String,
        required: String,
    },
    #[error("invalid Devtool request: {0}")]
    InvalidRequest(String),
    #[error("invalid Devtool {field} token")]
    InvalidToken { field: &'static str },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompatibilityFixtureRole, release_capability_fixtures};
    use std::{collections::BTreeMap, fs, path::PathBuf};
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementation, CapabilityImplementationKind, CapabilityReason, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, ToolIdentity,
        YoctoEnvironmentIdentity,
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

    #[test]
    fn compatibility_command_shared_fixtures_gate_devtool_upgrade_before_argv() {
        let authority = |role, generation| {
            release_capability_fixtures()
                .into_iter()
                .find(|fixture| fixture.role == role)
                .unwrap()
                .command_authority(generation)
        };
        let upgrade = DevtoolOperation::Upgrade {
            recipe: "busybox".into(),
        };

        let old = authority(CompatibilityFixtureRole::OldestPolicyCandidate, 33);
        let old_build = old.snapshot.environment.build_directory.value().unwrap();
        let old_tool = old
            .snapshot
            .environment
            .available_tools
            .value()
            .unwrap()
            .iter()
            .find(|tool| tool.id == "devtool")
            .unwrap()
            .executable
            .as_path();
        let old_planner = DevtoolCommandPlanner::new(&old, 33, old_build, old_tool).unwrap();
        assert!(matches!(
            old_planner.operation(&upgrade),
            Err(DevtoolCompatibilityError::Unavailable {
                capability: CapabilityId::DevtoolUpgrade,
                ..
            })
        ));

        let modern = authority(CompatibilityFixtureRole::LatestSupportCandidate, 34);
        let modern_build = modern.snapshot.environment.build_directory.value().unwrap();
        let modern_tool = modern
            .snapshot
            .environment
            .available_tools
            .value()
            .unwrap()
            .iter()
            .find(|tool| tool.id == "devtool")
            .unwrap()
            .executable
            .as_path();
        let command = DevtoolCommandPlanner::new(&modern, 34, modern_build, modern_tool)
            .unwrap()
            .operation(&upgrade)
            .unwrap();
        assert_eq!(command.arguments(), ["upgrade", "busybox"]);
    }

    #[test]
    fn compatibility_devtool_generates_exact_argv_for_each_independently_probed_subcommand() {
        let build = Path::new("/work/build");
        let executable = Path::new("/work/poky/scripts/devtool");
        let available = [
            (CapabilityId::DevtoolStatus, DEVTOOL_STATUS_IMPLEMENTATION),
            (
                CapabilityId::DevtoolEditRecipe,
                DEVTOOL_EDIT_RECIPE_IMPLEMENTATION,
            ),
            (CapabilityId::DevtoolModify, DEVTOOL_MODIFY_IMPLEMENTATION),
            (
                CapabilityId::DevtoolUpdateRecipe,
                DEVTOOL_UPDATE_RECIPE_IMPLEMENTATION,
            ),
            (CapabilityId::DevtoolFinish, DEVTOOL_FINISH_IMPLEMENTATION),
            (
                CapabilityId::DevtoolDeployTarget,
                DEVTOOL_DEPLOY_TARGET_IMPLEMENTATION,
            ),
            (
                CapabilityId::DevtoolUndeployTarget,
                DEVTOOL_UNDEPLOY_TARGET_IMPLEMENTATION,
            ),
            (CapabilityId::DevtoolReset, DEVTOOL_RESET_IMPLEMENTATION),
            (CapabilityId::DevtoolUpgrade, DEVTOOL_UPGRADE_IMPLEMENTATION),
        ];
        let authority = authority(build, executable, 3, &available, &[]);
        let planner = DevtoolCommandPlanner::new(&authority, 3, build, executable).unwrap();
        assert_eq!(planner.status().unwrap().arguments(), ["status"]);
        assert_eq!(
            planner.edit_recipe("busybox").unwrap().arguments(),
            ["edit-recipe", "busybox"]
        );
        for (operation, capability, expected) in all_operations() {
            let command = planner.operation(&operation).unwrap();
            assert_eq!(command.capability(), capability);
            assert_eq!(command.capability_generation(), 3);
            assert_eq!(command.arguments(), expected);
            assert_eq!(command.executable(), executable);
        }
    }

    #[test]
    fn compatibility_devtool_old_surface_disables_only_absent_upgrade_with_exact_reason() {
        let build = Path::new("/work/build");
        let executable = Path::new("/work/poky/scripts/devtool");
        let authority = authority(
            build,
            executable,
            4,
            &[(CapabilityId::DevtoolModify, DEVTOOL_MODIFY_IMPLEMENTATION)],
            &[CapabilityId::DevtoolUpgrade],
        );
        let planner = DevtoolCommandPlanner::new(&authority, 4, build, executable).unwrap();
        planner
            .operation(&DevtoolOperation::Modify {
                recipe: "busybox".into(),
            })
            .unwrap();
        assert!(matches!(
            planner.operation(&DevtoolOperation::Upgrade {
                recipe: "busybox".into()
            }),
            Err(DevtoolCompatibilityError::Unavailable { reason, .. })
                if reason.contains("devtool.upgrade")
        ));
    }

    #[test]
    fn compatibility_devtool_rejects_stale_environment_executable_and_cross_subcommand_authority() {
        let build = Path::new("/work/build");
        let executable = Path::new("/work/poky/scripts/devtool");
        let authority = authority(
            build,
            executable,
            5,
            &[(CapabilityId::DevtoolModify, DEVTOOL_MODIFY_IMPLEMENTATION)],
            &[],
        );
        assert!(matches!(
            DevtoolCommandPlanner::new(&authority, 4, build, executable),
            Err(DevtoolCompatibilityError::StaleGeneration { .. })
        ));
        assert!(matches!(
            DevtoolCommandPlanner::new(&authority, 5, Path::new("/other"), executable),
            Err(DevtoolCompatibilityError::EnvironmentMismatch)
        ));
        assert!(matches!(
            DevtoolCommandPlanner::new(&authority, 5, build, Path::new("/usr/bin/devtool")),
            Err(DevtoolCompatibilityError::ExecutableMismatch)
        ));
        let planner = DevtoolCommandPlanner::new(&authority, 5, build, executable).unwrap();
        assert!(matches!(
            planner.operation(&DevtoolOperation::Finish {
                recipe: "busybox".into(),
                destination: "/layers/meta".into()
            }),
            Err(DevtoolCompatibilityError::CapabilityMissing {
                capability: CapabilityId::DevtoolFinish
            })
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn compatibility_devtool_unavailable_subcommand_never_spawns_process() {
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!(
            "yoctui-compatibility-devtool-no-spawn-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("devtool");
        let marker = root.join("spawned");
        fs::write(
            &executable,
            format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let authority = authority(&root, &executable, 6, &[], &[CapabilityId::DevtoolUpgrade]);
        let planner = DevtoolCommandPlanner::new(&authority, 6, &root, &executable).unwrap();
        let planned = planner.operation(&DevtoolOperation::Upgrade {
            recipe: "busybox".into(),
        });
        assert!(planned.is_err());
        assert!(!marker.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
