use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use thiserror::Error;
use yoctui_model::{BuildRequest, CapabilityId, CapabilityToolId, DaemonCompatibilitySnapshot};

pub const BITBAKE_BUILD_ARGV_IMPLEMENTATION: &str = "bitbake.build.argv";
pub const BITBAKE_FORCE_TASK_ARGV_IMPLEMENTATION: &str = "bitbake.force_task.argv";
pub const BITBAKE_GRAPH_ARGV_IMPLEMENTATION: &str = "bitbake.graph.argv";
pub const BITBAKE_ENVIRONMENT_ARGV_IMPLEMENTATION: &str = "bitbake.environment_dump.argv";
pub const BITBAKE_GETVAR_UTILITY_IMPLEMENTATION: &str = "bitbake_getvar.argv";
pub const BITBAKE_GETVAR_ENVIRONMENT_IMPLEMENTATION: &str = "bitbake.environment_lookup";
pub const BITBAKE_DUMPSIG_ARGV_IMPLEMENTATION: &str = "bitbake_dumpsig.argv";
pub const BITBAKE_DIFFSIGS_ARGV_IMPLEMENTATION: &str = "bitbake_diffsigs.argv";
pub const BITBAKE_SERVER_STATUS_ARGV_IMPLEMENTATION: &str = "bitbake.server.status.argv";
pub const BITBAKE_SERVER_START_ARGV_IMPLEMENTATION: &str = "bitbake.server.start.argv";
pub const BITBAKE_SERVER_STOP_ARGV_IMPLEMENTATION: &str = "bitbake.server.stop.argv";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitBakeServerCommandOperation {
    Status,
    Start,
    Stop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedBitBakeCommand {
    pub capability: CapabilityId,
    pub implementation: String,
    pub executable: PathBuf,
    pub arguments: Vec<OsString>,
    pub generation: u64,
}

pub struct BitBakeCommandPlanner<'a> {
    authority: &'a DaemonCompatibilitySnapshot,
    expected_generation: u64,
}

impl<'a> BitBakeCommandPlanner<'a> {
    pub fn new(
        authority: &'a DaemonCompatibilitySnapshot,
        expected_generation: u64,
        build_directory: &Path,
    ) -> Result<Self, BitBakeCommandAuthorizationError> {
        if authority.snapshot.generation != expected_generation {
            return Err(BitBakeCommandAuthorizationError::StaleGeneration {
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
            return Err(BitBakeCommandAuthorizationError::EnvironmentMismatch);
        }
        Ok(Self {
            authority,
            expected_generation,
        })
    }

    pub fn build(
        &self,
        request: &BuildRequest,
    ) -> Result<AuthorizedBitBakeCommand, BitBakeCommandAuthorizationError> {
        request
            .validate()
            .map_err(|error| BitBakeCommandAuthorizationError::InvalidRequest(error.to_string()))?;
        self.require(
            CapabilityId::BitBakeBuild,
            BITBAKE_BUILD_ARGV_IMPLEMENTATION,
        )?;
        if request.force || request.task.is_some() {
            self.require(
                CapabilityId::BitBakeForceTask,
                BITBAKE_FORCE_TASK_ARGV_IMPLEMENTATION,
            )?;
        }
        let mut arguments = Vec::new();
        if request.force {
            arguments.push("-f".into());
        }
        if let Some(task) = &request.task {
            arguments.extend([OsString::from("-c"), task.into()]);
        }
        arguments.extend(request.targets.iter().map(OsString::from));
        self.command(
            CapabilityId::BitBakeBuild,
            BITBAKE_BUILD_ARGV_IMPLEMENTATION,
            CapabilityToolId::BitBake,
            arguments,
        )
    }

    pub fn dependency_graph(
        &self,
        target: &str,
    ) -> Result<AuthorizedBitBakeCommand, BitBakeCommandAuthorizationError> {
        if target.is_empty() || target.starts_with('-') || target.chars().any(char::is_whitespace) {
            return Err(BitBakeCommandAuthorizationError::InvalidRequest(
                "dependency graph target is invalid".into(),
            ));
        }
        self.require(
            CapabilityId::BitBakeGraphGeneration,
            BITBAKE_GRAPH_ARGV_IMPLEMENTATION,
        )?;
        self.command(
            CapabilityId::BitBakeGraphGeneration,
            BITBAKE_GRAPH_ARGV_IMPLEMENTATION,
            CapabilityToolId::BitBake,
            vec!["-g".into(), target.into()],
        )
    }

    pub fn environment_dump(
        &self,
        recipe: Option<&str>,
    ) -> Result<AuthorizedBitBakeCommand, BitBakeCommandAuthorizationError> {
        self.require(
            CapabilityId::BitBakeEnvironmentDump,
            BITBAKE_ENVIRONMENT_ARGV_IMPLEMENTATION,
        )?;
        let mut arguments = vec![OsString::from("-e")];
        if let Some(recipe) = recipe {
            validate_value(recipe, "recipe")?;
            arguments.push(recipe.into());
        }
        self.command(
            CapabilityId::BitBakeEnvironmentDump,
            BITBAKE_ENVIRONMENT_ARGV_IMPLEMENTATION,
            CapabilityToolId::BitBake,
            arguments,
        )
    }

    pub fn get_variable(
        &self,
        name: &str,
        recipe: Option<&str>,
    ) -> Result<AuthorizedBitBakeCommand, BitBakeCommandAuthorizationError> {
        validate_value(name, "variable")?;
        if let Some(recipe) = recipe {
            validate_value(recipe, "recipe")?;
        }
        let implementation = self.require_one(
            CapabilityId::BitBakeGetVar,
            &[
                BITBAKE_GETVAR_UTILITY_IMPLEMENTATION,
                BITBAKE_GETVAR_ENVIRONMENT_IMPLEMENTATION,
            ],
        )?;
        let (tool, arguments) = if implementation == BITBAKE_GETVAR_UTILITY_IMPLEMENTATION {
            let mut arguments = vec![OsString::from("--value")];
            if let Some(recipe) = recipe {
                arguments.extend([OsString::from("--recipe"), recipe.into()]);
            }
            arguments.push(name.into());
            (CapabilityToolId::BitBakeGetVar, arguments)
        } else {
            let mut arguments = vec![OsString::from("-e")];
            if let Some(recipe) = recipe {
                arguments.push(recipe.into());
            }
            (CapabilityToolId::BitBake, arguments)
        };
        self.command(CapabilityId::BitBakeGetVar, implementation, tool, arguments)
    }

    pub fn signature_dump(
        &self,
        path: &Path,
    ) -> Result<AuthorizedBitBakeCommand, BitBakeCommandAuthorizationError> {
        self.require(
            CapabilityId::BitBakeDumpSig,
            BITBAKE_DUMPSIG_ARGV_IMPLEMENTATION,
        )?;
        self.command(
            CapabilityId::BitBakeDumpSig,
            BITBAKE_DUMPSIG_ARGV_IMPLEMENTATION,
            CapabilityToolId::BitBakeDumpSig,
            vec![path.as_os_str().to_owned()],
        )
    }

    pub fn signature_compare(
        &self,
        left: &Path,
        right: &Path,
    ) -> Result<AuthorizedBitBakeCommand, BitBakeCommandAuthorizationError> {
        self.require(
            CapabilityId::BitBakeDiffSigs,
            BITBAKE_DIFFSIGS_ARGV_IMPLEMENTATION,
        )?;
        self.command(
            CapabilityId::BitBakeDiffSigs,
            BITBAKE_DIFFSIGS_ARGV_IMPLEMENTATION,
            CapabilityToolId::BitBakeDiffSigs,
            vec![
                "-c".into(),
                "never".into(),
                left.as_os_str().to_owned(),
                right.as_os_str().to_owned(),
            ],
        )
    }

    pub fn server_control(
        &self,
        operation: BitBakeServerCommandOperation,
    ) -> Result<AuthorizedBitBakeCommand, BitBakeCommandAuthorizationError> {
        let (capability, implementation, argument) = match operation {
            BitBakeServerCommandOperation::Status => (
                CapabilityId::BitBakeServerStatus,
                BITBAKE_SERVER_STATUS_ARGV_IMPLEMENTATION,
                "--status-only",
            ),
            BitBakeServerCommandOperation::Start => (
                CapabilityId::BitBakeServerStart,
                BITBAKE_SERVER_START_ARGV_IMPLEMENTATION,
                "--server-only",
            ),
            BitBakeServerCommandOperation::Stop => (
                CapabilityId::BitBakeServerStop,
                BITBAKE_SERVER_STOP_ARGV_IMPLEMENTATION,
                "--kill-server",
            ),
        };
        self.require(capability, implementation)?;
        self.command(
            capability,
            implementation,
            CapabilityToolId::BitBake,
            vec![argument.into()],
        )
    }

    fn require(
        &self,
        id: CapabilityId,
        implementation: &str,
    ) -> Result<(), BitBakeCommandAuthorizationError> {
        self.require_one(id, &[implementation]).map(|_| ())
    }

    fn require_one<'b>(
        &self,
        id: CapabilityId,
        allowed: &'b [&str],
    ) -> Result<&'b str, BitBakeCommandAuthorizationError> {
        let record = self
            .authority
            .snapshot
            .capability(id)
            .ok_or(BitBakeCommandAuthorizationError::CapabilityMissing { capability: id })?;
        if !record.state.is_enabled() {
            return Err(BitBakeCommandAuthorizationError::Unavailable {
                capability: id,
                reason: record
                    .state
                    .reason()
                    .map(|reason| reason.message.clone())
                    .unwrap_or_else(|| "No positive capability evidence is available.".into()),
            });
        }
        let selected =
            self.authority.implementations.get(&id).ok_or(
                BitBakeCommandAuthorizationError::ImplementationMissing { capability: id },
            )?;
        allowed
            .iter()
            .copied()
            .find(|allowed| *allowed == selected.id)
            .ok_or_else(
                || BitBakeCommandAuthorizationError::ImplementationMismatch {
                    capability: id,
                    selected: selected.id.clone(),
                    required: allowed.join(" or "),
                },
            )
    }

    fn command(
        &self,
        capability: CapabilityId,
        implementation: &str,
        tool: CapabilityToolId,
        arguments: Vec<OsString>,
    ) -> Result<AuthorizedBitBakeCommand, BitBakeCommandAuthorizationError> {
        let executable = self
            .authority
            .snapshot
            .environment
            .available_tools
            .value()
            .and_then(|tools| {
                tools
                    .iter()
                    .find(|identity| identity.id == tool.executable_name())
            })
            .map(|identity| identity.executable.clone())
            .ok_or(BitBakeCommandAuthorizationError::ToolIdentityUnknown { tool })?;
        Ok(AuthorizedBitBakeCommand {
            capability,
            implementation: implementation.into(),
            executable,
            arguments,
            generation: self.expected_generation,
        })
    }
}

fn validate_value(value: &str, field: &str) -> Result<(), BitBakeCommandAuthorizationError> {
    if value.is_empty()
        || value.len() > 1024
        || value.starts_with('-')
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        return Err(BitBakeCommandAuthorizationError::InvalidRequest(format!(
            "{field} is invalid"
        )));
    }
    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BitBakeCommandAuthorizationError {
    #[error("capability snapshot generation is stale: expected {expected}, got {actual}")]
    StaleGeneration { expected: u64, actual: u64 },
    #[error("capability snapshot belongs to another build environment")]
    EnvironmentMismatch,
    #[error("required capability is absent from the snapshot: {capability}")]
    CapabilityMissing { capability: CapabilityId },
    #[error("{capability} is unavailable: {reason}")]
    Unavailable {
        capability: CapabilityId,
        reason: String,
    },
    #[error("enabled capability has no selected implementation: {capability}")]
    ImplementationMissing { capability: CapabilityId },
    #[error("{capability} selected incompatible implementation {selected}; required {required}")]
    ImplementationMismatch {
        capability: CapabilityId,
        selected: String,
        required: String,
    },
    #[error("invalid BitBake command request: {0}")]
    InvalidRequest(String),
    #[error("initialized environment does not identify required command tool: {tool:?}")]
    ToolIdentityUnknown { tool: CapabilityToolId },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompatibilityFixtureRole, release_capability_fixtures};
    use std::{collections::BTreeMap, path::PathBuf};
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementation, CapabilityImplementationKind, CapabilityReason, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, ToolIdentity,
        YoctoEnvironmentIdentity,
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

    #[test]
    fn compatibility_command_getvar_shared_fixtures_select_fallback_and_utility_argv() {
        let old = fixture_authority(CompatibilityFixtureRole::OldestPolicyCandidate, 31);
        let old_command = fixture_planner(&old)
            .get_variable("MACHINE", Some("busybox"))
            .unwrap();
        assert_eq!(old_command.arguments, ["-e", "busybox"]);
        assert_eq!(
            old_command.executable,
            Path::new("/fixtures/oldest-policy-candidate/bin/bitbake")
        );
        assert_eq!(
            old_command.implementation,
            BITBAKE_GETVAR_ENVIRONMENT_IMPLEMENTATION
        );

        let modern = fixture_authority(CompatibilityFixtureRole::LatestSupportCandidate, 32);
        let modern_command = fixture_planner(&modern)
            .get_variable("MACHINE", Some("busybox"))
            .unwrap();
        assert_eq!(
            modern_command.arguments,
            ["--value", "--recipe", "busybox", "MACHINE"]
        );
        assert_eq!(
            modern_command.implementation,
            BITBAKE_GETVAR_UTILITY_IMPLEMENTATION
        );
        assert_eq!(
            modern_command.executable,
            Path::new("/fixtures/latest-support-candidate/bin/bitbake-getvar")
        );
        assert!(
            !modern_command
                .arguments
                .iter()
                .any(|argument| argument == "--getvar")
        );
    }

    #[test]
    fn compatibility_command_getvar_generates_old_and_new_argv_without_unsupported_option() {
        let old = authority(
            1,
            &[(
                CapabilityId::BitBakeGetVar,
                BITBAKE_GETVAR_ENVIRONMENT_IMPLEMENTATION,
            )],
        );
        let old = planner(&old)
            .get_variable("MACHINE", Some("busybox"))
            .unwrap();
        assert_eq!(old.arguments, ["-e", "busybox"]);
        assert!(!old.arguments.iter().any(|argument| argument == "--getvar"));

        let new = authority(
            2,
            &[(
                CapabilityId::BitBakeGetVar,
                BITBAKE_GETVAR_UTILITY_IMPLEMENTATION,
            )],
        );
        let new = planner(&new)
            .get_variable("MACHINE", Some("busybox"))
            .unwrap();
        assert_eq!(new.arguments, ["--value", "--recipe", "busybox", "MACHINE"]);
        assert!(!new.arguments.iter().any(|argument| argument == "-e"));
        assert!(!new.arguments.iter().any(|argument| argument == "--getvar"));

        let mut missing_tool = authority(
            3,
            &[(
                CapabilityId::BitBakeGetVar,
                BITBAKE_GETVAR_UTILITY_IMPLEMENTATION,
            )],
        );
        missing_tool.snapshot.environment.available_tools = AuthoritativeValue::Unknown;
        let missing_tool = missing_tool.normalize().unwrap();
        assert!(matches!(
            planner(&missing_tool).get_variable("MACHINE", None),
            Err(BitBakeCommandAuthorizationError::ToolIdentityUnknown {
                tool: CapabilityToolId::BitBakeGetVar
            })
        ));
        assert!(matches!(
            BitBakeCommandPlanner::new(&missing_tool, 2, Path::new("/work/build")),
            Err(BitBakeCommandAuthorizationError::StaleGeneration { .. })
        ));
    }

    #[test]
    fn compatibility_command_emits_only_authorized_build_graph_and_server_options() {
        let authority = authority(
            4,
            &[
                (
                    CapabilityId::BitBakeBuild,
                    BITBAKE_BUILD_ARGV_IMPLEMENTATION,
                ),
                (
                    CapabilityId::BitBakeForceTask,
                    BITBAKE_FORCE_TASK_ARGV_IMPLEMENTATION,
                ),
                (
                    CapabilityId::BitBakeGraphGeneration,
                    BITBAKE_GRAPH_ARGV_IMPLEMENTATION,
                ),
                (
                    CapabilityId::BitBakeServerStart,
                    BITBAKE_SERVER_START_ARGV_IMPLEMENTATION,
                ),
                (
                    CapabilityId::BitBakeDumpSig,
                    BITBAKE_DUMPSIG_ARGV_IMPLEMENTATION,
                ),
                (
                    CapabilityId::BitBakeDiffSigs,
                    BITBAKE_DIFFSIGS_ARGV_IMPLEMENTATION,
                ),
            ],
        );
        let planner = planner(&authority);
        assert_eq!(
            planner
                .build(&BuildRequest {
                    targets: vec!["busybox".into()],
                    task: Some("compile".into()),
                    force: true,
                })
                .unwrap()
                .arguments,
            ["-f", "-c", "compile", "busybox"]
        );
        assert_eq!(
            planner.dependency_graph("busybox").unwrap().arguments,
            ["-g", "busybox"]
        );
        assert_eq!(
            planner
                .server_control(BitBakeServerCommandOperation::Start)
                .unwrap()
                .arguments,
            ["--server-only"]
        );
        assert!(matches!(
            planner.server_control(BitBakeServerCommandOperation::Stop),
            Err(BitBakeCommandAuthorizationError::CapabilityMissing {
                capability: CapabilityId::BitBakeServerStop
            })
        ));
        assert_eq!(
            planner
                .signature_dump(Path::new("/work/build/one.sigdata"))
                .unwrap()
                .arguments,
            ["/work/build/one.sigdata"]
        );
        assert_eq!(
            planner
                .signature_compare(
                    Path::new("/work/build/one.sigdata"),
                    Path::new("/work/build/two.sigdata")
                )
                .unwrap()
                .arguments,
            [
                "-c",
                "never",
                "/work/build/one.sigdata",
                "/work/build/two.sigdata"
            ]
        );
    }

    #[test]
    fn compatibility_command_rejects_unavailable_stale_and_other_environment_before_argv() {
        let reason = CapabilityReason::new(
            "command.option_missing",
            "Current BitBake help does not expose -g.",
            Some("Required option: -g".into()),
        )
        .unwrap();
        let mut unavailable = authority(
            8,
            &[(
                CapabilityId::BitBakeGraphGeneration,
                BITBAKE_GRAPH_ARGV_IMPLEMENTATION,
            )],
        );
        unavailable.snapshot.capabilities[0] = CapabilityRecord {
            id: CapabilityId::BitBakeGraphGeneration,
            state: CapabilityState::Unavailable { reason },
            evidence: vec![CapabilityEvidence {
                kind: CapabilityEvidenceKind::DirectProbe,
                outcome: CapabilityEvidenceOutcome::Negative,
                subject: "bitbake --help".into(),
                detail: "The -g option is absent.".into(),
                argv: vec!["bitbake".into(), "--help".into()],
            }],
        };
        unavailable.implementations.clear();
        let unavailable = unavailable.normalize().unwrap();
        assert!(matches!(
            planner(&unavailable).dependency_graph("busybox"),
            Err(BitBakeCommandAuthorizationError::Unavailable { reason, .. })
                if reason.contains("does not expose -g")
        ));
        assert!(matches!(
            BitBakeCommandPlanner::new(&unavailable, 7, Path::new("/work/build")),
            Err(BitBakeCommandAuthorizationError::StaleGeneration { .. })
        ));
        assert!(matches!(
            BitBakeCommandPlanner::new(&unavailable, 8, Path::new("/other/build")),
            Err(BitBakeCommandAuthorizationError::EnvironmentMismatch)
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn compatibility_command_unavailable_action_is_rejected_before_process_spawn() {
        use crate::{BitBakeBackend, ProcessBackend};
        use std::{fs, os::unix::fs::PermissionsExt};

        let root = std::env::temp_dir().join(format!(
            "yoctui-compatibility-command-no-spawn-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("bitbake");
        let marker = root.join("spawned");
        fs::write(
            &executable,
            format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        )
        .unwrap();
        let mut permissions = fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&executable, permissions).unwrap();

        let mut unavailable = authority(
            1,
            &[(
                CapabilityId::BitBakeBuild,
                BITBAKE_BUILD_ARGV_IMPLEMENTATION,
            )],
        );
        unavailable.snapshot.environment.build_directory =
            AuthoritativeValue::detected(root.clone(), IdentityAuthority::InitializedEnvironment);
        unavailable.snapshot.capabilities[0] = CapabilityRecord {
            id: CapabilityId::BitBakeBuild,
            state: CapabilityState::Unavailable {
                reason: CapabilityReason::new(
                    "command.unavailable",
                    "The connected BitBake command was not positively verified.",
                    None,
                )
                .unwrap(),
            },
            evidence: vec![CapabilityEvidence {
                kind: CapabilityEvidenceKind::DirectProbe,
                outcome: CapabilityEvidenceOutcome::Negative,
                subject: "bitbake executable".into(),
                detail: "The required command behavior is absent.".into(),
                argv: Vec::new(),
            }],
        };
        unavailable.implementations.clear();
        let mut backend = ProcessBackend::with_executable(root.clone(), executable)
            .with_compatibility(unavailable.normalize().unwrap())
            .unwrap();
        let error = backend
            .start_build(BuildRequest {
                targets: vec!["busybox".into()],
                task: None,
                force: false,
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("not positively verified"));
        assert!(!marker.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
