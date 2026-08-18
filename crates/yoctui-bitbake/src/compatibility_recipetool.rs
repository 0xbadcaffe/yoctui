use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use thiserror::Error;
use yoctui_model::{CapabilityId, DaemonCompatibilitySnapshot, RecipetoolOperation};

pub const RECIPETOOL_CREATE_IMPLEMENTATION: &str = "recipetool.create.argv";
pub const RECIPETOOL_CREATE_OUTFILE_IMPLEMENTATION: &str = "recipetool.create.outfile.argv";
pub const RECIPETOOL_APPEND_FILE_IMPLEMENTATION: &str = "recipetool.appendfile.argv";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipetoolCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
    build_directory: PathBuf,
    capability_generation: u64,
    required_capabilities: Vec<CapabilityId>,
}

impl RecipetoolCommandSpec {
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn build_directory(&self) -> &Path {
        &self.build_directory
    }

    pub fn capability_generation(&self) -> u64 {
        self.capability_generation
    }

    pub fn required_capabilities(&self) -> &[CapabilityId] {
        &self.required_capabilities
    }
}

pub struct RecipetoolCommandPlanner<'a> {
    authority: &'a DaemonCompatibilitySnapshot,
    executable: &'a Path,
    build_directory: &'a Path,
}

impl<'a> RecipetoolCommandPlanner<'a> {
    pub fn from_environment(
        authority: &'a DaemonCompatibilitySnapshot,
        expected_generation: u64,
        build_directory: &'a Path,
    ) -> Result<Self, RecipetoolCompatibilityError> {
        let executable = authority
            .snapshot
            .environment
            .available_tools
            .value()
            .and_then(|tools| tools.iter().find(|tool| tool.id == "recipetool"))
            .map(|tool| tool.executable.as_path())
            .ok_or(RecipetoolCompatibilityError::ToolIdentityUnknown)?;
        Self::new(authority, expected_generation, build_directory, executable)
    }

    pub fn new(
        authority: &'a DaemonCompatibilitySnapshot,
        expected_generation: u64,
        build_directory: &'a Path,
        executable: &'a Path,
    ) -> Result<Self, RecipetoolCompatibilityError> {
        if authority.snapshot.generation != expected_generation {
            return Err(RecipetoolCompatibilityError::StaleGeneration {
                expected: expected_generation,
                actual: authority.snapshot.generation,
            });
        }
        if authority
            .snapshot
            .environment
            .build_directory
            .value()
            .map(PathBuf::as_path)
            != Some(build_directory)
        {
            return Err(RecipetoolCompatibilityError::EnvironmentMismatch);
        }
        let detected = authority
            .snapshot
            .environment
            .available_tools
            .value()
            .and_then(|tools| tools.iter().find(|tool| tool.id == "recipetool"))
            .ok_or(RecipetoolCompatibilityError::ToolIdentityUnknown)?;
        if detected.executable != executable {
            return Err(RecipetoolCompatibilityError::ExecutableMismatch);
        }
        Ok(Self {
            authority,
            executable,
            build_directory,
        })
    }

    pub fn operation(
        &self,
        operation: &RecipetoolOperation,
    ) -> Result<RecipetoolCommandSpec, RecipetoolCompatibilityError> {
        operation
            .validate()
            .map_err(|error| RecipetoolCompatibilityError::InvalidRequest(error.to_string()))?;
        match operation {
            RecipetoolOperation::Create { source, outfile } => self.command(
                &[
                    (
                        CapabilityId::RecipetoolCreate,
                        RECIPETOOL_CREATE_IMPLEMENTATION,
                    ),
                    (
                        CapabilityId::RecipetoolCreateOutfile,
                        RECIPETOOL_CREATE_OUTFILE_IMPLEMENTATION,
                    ),
                ],
                vec![
                    "create".into(),
                    "--outfile".into(),
                    outfile.as_os_str().to_owned(),
                    source.into(),
                ],
            ),
            RecipetoolOperation::AppendFile {
                destination_layer,
                target_path,
                replacement_file,
            } => self.command(
                &[(
                    CapabilityId::RecipetoolAppendFile,
                    RECIPETOOL_APPEND_FILE_IMPLEMENTATION,
                )],
                vec![
                    "appendfile".into(),
                    destination_layer.as_os_str().to_owned(),
                    target_path.as_os_str().to_owned(),
                    replacement_file.as_os_str().to_owned(),
                ],
            ),
        }
    }

    fn command(
        &self,
        requirements: &[(CapabilityId, &str)],
        arguments: Vec<OsString>,
    ) -> Result<RecipetoolCommandSpec, RecipetoolCompatibilityError> {
        for (capability, implementation) in requirements {
            let record = self.authority.snapshot.capability(*capability).ok_or(
                RecipetoolCompatibilityError::CapabilityMissing {
                    capability: *capability,
                },
            )?;
            if !record.state.is_enabled() {
                return Err(RecipetoolCompatibilityError::Unavailable {
                    capability: *capability,
                    reason: record
                        .state
                        .reason()
                        .map(|reason| reason.message.clone())
                        .unwrap_or_else(|| {
                            "No positive Recipetool capability evidence is available.".into()
                        }),
                });
            }
            let selected = self.authority.implementations.get(capability).ok_or(
                RecipetoolCompatibilityError::ImplementationMissing {
                    capability: *capability,
                },
            )?;
            if selected.id != *implementation {
                return Err(RecipetoolCompatibilityError::ImplementationMismatch {
                    capability: *capability,
                    selected: selected.id.clone(),
                    required: (*implementation).into(),
                });
            }
        }
        Ok(RecipetoolCommandSpec {
            executable: self.executable.to_owned(),
            arguments,
            build_directory: self.build_directory.to_owned(),
            capability_generation: self.authority.snapshot.generation,
            required_capabilities: requirements.iter().map(|(id, _)| *id).collect(),
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RecipetoolCompatibilityError {
    #[error("stale Recipetool capability generation: expected {expected}, got {actual}")]
    StaleGeneration { expected: u64, actual: u64 },
    #[error("Recipetool capability snapshot belongs to another build environment")]
    EnvironmentMismatch,
    #[error("Recipetool executable identity is unknown in the initialized environment")]
    ToolIdentityUnknown,
    #[error("Recipetool executable does not match the initialized-environment tool identity")]
    ExecutableMismatch,
    #[error("Recipetool capability {capability:?} is missing")]
    CapabilityMissing { capability: CapabilityId },
    #[error("Recipetool capability {capability:?} is unavailable: {reason}")]
    Unavailable {
        capability: CapabilityId,
        reason: String,
    },
    #[error("Recipetool capability {capability:?} has no selected implementation")]
    ImplementationMissing { capability: CapabilityId },
    #[error("Recipetool capability {capability:?} selected {selected}, not {required}")]
    ImplementationMismatch {
        capability: CapabilityId,
        selected: String,
        required: String,
    },
    #[error("invalid Recipetool request: {0}")]
    InvalidRequest(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, fs};
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

    #[test]
    fn compatibility_recipetool_generates_exact_create_and_appendfile_argv() {
        let build = Path::new("/work/build");
        let executable = Path::new("/work/poky/scripts/recipetool");
        let available = [
            (
                CapabilityId::RecipetoolCreate,
                RECIPETOOL_CREATE_IMPLEMENTATION,
            ),
            (
                CapabilityId::RecipetoolCreateOutfile,
                RECIPETOOL_CREATE_OUTFILE_IMPLEMENTATION,
            ),
            (
                CapabilityId::RecipetoolAppendFile,
                RECIPETOOL_APPEND_FILE_IMPLEMENTATION,
            ),
        ];
        let authority = authority(build, executable, 7, &available, &[]);
        let planner = RecipetoolCommandPlanner::new(&authority, 7, build, executable).unwrap();
        let create = planner.operation(&create()).unwrap();
        assert_eq!(
            create.arguments(),
            [
                "create",
                "--outfile",
                "/layers/meta-demo/recipes-demo/demo.bb",
                "https://example.invalid/demo.tar.gz",
            ]
        );
        assert_eq!(
            create.required_capabilities(),
            [
                CapabilityId::RecipetoolCreate,
                CapabilityId::RecipetoolCreateOutfile,
            ]
        );
        assert_eq!(
            planner.operation(&appendfile()).unwrap().arguments(),
            ["appendfile", "/layers/meta-demo", "/etc/motd", "/work/motd"]
        );
    }

    #[test]
    fn compatibility_recipetool_old_surface_keeps_appendfile_but_rejects_missing_outfile() {
        let build = Path::new("/work/build");
        let executable = Path::new("/work/poky/scripts/recipetool");
        let authority = authority(
            build,
            executable,
            8,
            &[
                (
                    CapabilityId::RecipetoolCreate,
                    RECIPETOOL_CREATE_IMPLEMENTATION,
                ),
                (
                    CapabilityId::RecipetoolAppendFile,
                    RECIPETOOL_APPEND_FILE_IMPLEMENTATION,
                ),
            ],
            &[CapabilityId::RecipetoolCreateOutfile],
        );
        let planner = RecipetoolCommandPlanner::new(&authority, 8, build, executable).unwrap();
        planner.operation(&appendfile()).unwrap();
        assert!(matches!(
            planner.operation(&create()),
            Err(RecipetoolCompatibilityError::Unavailable {
                capability: CapabilityId::RecipetoolCreateOutfile,
                reason,
            }) if reason.contains("recipetool.create_outfile")
        ));
    }

    #[test]
    fn compatibility_recipetool_rejects_stale_environment_executable_and_cross_subcommand() {
        let build = Path::new("/work/build");
        let executable = Path::new("/work/poky/scripts/recipetool");
        let authority = authority(
            build,
            executable,
            9,
            &[(
                CapabilityId::RecipetoolCreate,
                RECIPETOOL_CREATE_IMPLEMENTATION,
            )],
            &[],
        );
        assert!(matches!(
            RecipetoolCommandPlanner::new(&authority, 8, build, executable),
            Err(RecipetoolCompatibilityError::StaleGeneration { .. })
        ));
        assert!(matches!(
            RecipetoolCommandPlanner::new(&authority, 9, Path::new("/other"), executable),
            Err(RecipetoolCompatibilityError::EnvironmentMismatch)
        ));
        assert!(matches!(
            RecipetoolCommandPlanner::new(&authority, 9, build, Path::new("/usr/bin/recipetool")),
            Err(RecipetoolCompatibilityError::ExecutableMismatch)
        ));
        let planner = RecipetoolCommandPlanner::new(&authority, 9, build, executable).unwrap();
        assert!(matches!(
            planner.operation(&appendfile()),
            Err(RecipetoolCompatibilityError::CapabilityMissing {
                capability: CapabilityId::RecipetoolAppendFile,
            })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn compatibility_recipetool_unavailable_option_never_spawns_process() {
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!(
            "yoctui-compatibility-recipetool-no-spawn-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("recipetool");
        let marker = root.join("spawned");
        fs::write(
            &executable,
            format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let authority = authority(
            &root,
            &executable,
            10,
            &[(
                CapabilityId::RecipetoolCreate,
                RECIPETOOL_CREATE_IMPLEMENTATION,
            )],
            &[CapabilityId::RecipetoolCreateOutfile],
        );
        let planner = RecipetoolCommandPlanner::new(&authority, 10, &root, &executable).unwrap();
        assert!(planner.operation(&create()).is_err());
        assert!(!marker.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
