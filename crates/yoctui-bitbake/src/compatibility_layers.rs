use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use thiserror::Error;
use yoctui_model::{BitBakeLayersOperation, CapabilityId, DaemonCompatibilitySnapshot};

pub const BITBAKE_LAYERS_SHOW_IMPLEMENTATION: &str = "bitbake_layers.show_layers.argv";
pub const BITBAKE_LAYERS_CREATE_IMPLEMENTATION: &str = "bitbake_layers.create_layer.argv";
pub const BITBAKE_LAYERS_CREATE_ADD_IMPLEMENTATION: &str =
    "bitbake_layers.create_and_add_layer.argv";
pub const BITBAKE_LAYERS_ADD_IMPLEMENTATION: &str = "bitbake_layers.add_layer.argv";
pub const BITBAKE_LAYERS_REMOVE_IMPLEMENTATION: &str = "bitbake_layers.remove_layer.argv";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeLayersCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
    build_directory: PathBuf,
    generation: u64,
    capability: CapabilityId,
}

impl BitBakeLayersCommandSpec {
    pub fn executable(&self) -> &Path {
        &self.executable
    }
    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
    pub fn build_directory(&self) -> &Path {
        &self.build_directory
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn capability(&self) -> CapabilityId {
        self.capability
    }
}

pub struct BitBakeLayersCommandPlanner<'a> {
    authority: &'a DaemonCompatibilitySnapshot,
    executable: &'a Path,
    build_directory: &'a Path,
}

impl<'a> BitBakeLayersCommandPlanner<'a> {
    pub fn from_environment(
        authority: &'a DaemonCompatibilitySnapshot,
        expected_generation: u64,
        build_directory: &'a Path,
    ) -> Result<Self, BitBakeLayersCompatibilityError> {
        let executable = authority
            .snapshot
            .environment
            .available_tools
            .value()
            .and_then(|tools| tools.iter().find(|tool| tool.id == "bitbake-layers"))
            .map(|tool| tool.executable.as_path())
            .ok_or(BitBakeLayersCompatibilityError::ToolIdentityUnknown)?;
        Self::new(authority, expected_generation, build_directory, executable)
    }

    pub fn new(
        authority: &'a DaemonCompatibilitySnapshot,
        expected_generation: u64,
        build_directory: &'a Path,
        executable: &'a Path,
    ) -> Result<Self, BitBakeLayersCompatibilityError> {
        if authority.snapshot.generation != expected_generation {
            return Err(BitBakeLayersCompatibilityError::StaleGeneration {
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
            return Err(BitBakeLayersCompatibilityError::EnvironmentMismatch);
        }
        let detected = authority
            .snapshot
            .environment
            .available_tools
            .value()
            .and_then(|tools| tools.iter().find(|tool| tool.id == "bitbake-layers"))
            .ok_or(BitBakeLayersCompatibilityError::ToolIdentityUnknown)?;
        if detected.executable != executable {
            return Err(BitBakeLayersCompatibilityError::ExecutableMismatch);
        }
        Ok(Self {
            authority,
            executable,
            build_directory,
        })
    }

    pub fn operation(
        &self,
        operation: &BitBakeLayersOperation,
    ) -> Result<BitBakeLayersCommandSpec, BitBakeLayersCompatibilityError> {
        operation
            .validate()
            .map_err(|error| BitBakeLayersCompatibilityError::InvalidRequest(error.to_string()))?;
        match operation {
            BitBakeLayersOperation::ShowLayers => self.command(
                CapabilityId::BitBakeLayersShowLayers,
                BITBAKE_LAYERS_SHOW_IMPLEMENTATION,
                vec!["show-layers".into()],
            ),
            BitBakeLayersOperation::CreateLayer {
                directory,
                add: false,
            } => self.command(
                CapabilityId::BitBakeLayersCreateLayer,
                BITBAKE_LAYERS_CREATE_IMPLEMENTATION,
                vec!["create-layer".into(), directory.as_os_str().to_owned()],
            ),
            BitBakeLayersOperation::CreateLayer {
                directory,
                add: true,
            } => self.command(
                CapabilityId::BitBakeLayersCreateAndAddLayer,
                BITBAKE_LAYERS_CREATE_ADD_IMPLEMENTATION,
                vec![
                    "create-layer".into(),
                    "--add-layer".into(),
                    directory.as_os_str().to_owned(),
                ],
            ),
            BitBakeLayersOperation::AddLayers { directories } => self.command(
                CapabilityId::BitBakeLayersAddLayer,
                BITBAKE_LAYERS_ADD_IMPLEMENTATION,
                std::iter::once(OsString::from("add-layer"))
                    .chain(directories.iter().map(|path| path.as_os_str().to_owned()))
                    .collect(),
            ),
            BitBakeLayersOperation::RemoveLayers { directories } => self.command(
                CapabilityId::BitBakeLayersRemoveLayer,
                BITBAKE_LAYERS_REMOVE_IMPLEMENTATION,
                std::iter::once(OsString::from("remove-layer"))
                    .chain(directories.iter().map(|path| path.as_os_str().to_owned()))
                    .collect(),
            ),
        }
    }

    fn command(
        &self,
        capability: CapabilityId,
        implementation: &str,
        arguments: Vec<OsString>,
    ) -> Result<BitBakeLayersCommandSpec, BitBakeLayersCompatibilityError> {
        let record = self
            .authority
            .snapshot
            .capability(capability)
            .ok_or(BitBakeLayersCompatibilityError::CapabilityMissing { capability })?;
        if !record.state.is_enabled() {
            return Err(BitBakeLayersCompatibilityError::Unavailable {
                capability,
                reason: record
                    .state
                    .reason()
                    .map(|reason| reason.message.clone())
                    .unwrap_or_else(|| {
                        "No positive bitbake-layers capability evidence is available.".into()
                    }),
            });
        }
        let selected = self
            .authority
            .implementations
            .get(&capability)
            .ok_or(BitBakeLayersCompatibilityError::ImplementationMissing { capability })?;
        if selected.id != implementation {
            return Err(BitBakeLayersCompatibilityError::ImplementationMismatch {
                capability,
                selected: selected.id.clone(),
                required: implementation.into(),
            });
        }
        Ok(BitBakeLayersCommandSpec {
            executable: self.executable.to_owned(),
            arguments,
            build_directory: self.build_directory.to_owned(),
            generation: self.authority.snapshot.generation,
            capability,
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BitBakeLayersCompatibilityError {
    #[error("stale bitbake-layers capability generation: expected {expected}, got {actual}")]
    StaleGeneration { expected: u64, actual: u64 },
    #[error("bitbake-layers snapshot belongs to another build environment")]
    EnvironmentMismatch,
    #[error("bitbake-layers executable identity is unknown")]
    ToolIdentityUnknown,
    #[error("bitbake-layers executable identity changed")]
    ExecutableMismatch,
    #[error("bitbake-layers capability {capability:?} is missing")]
    CapabilityMissing { capability: CapabilityId },
    #[error("bitbake-layers capability {capability:?} is unavailable: {reason}")]
    Unavailable {
        capability: CapabilityId,
        reason: String,
    },
    #[error("bitbake-layers capability {capability:?} has no implementation")]
    ImplementationMissing { capability: CapabilityId },
    #[error("bitbake-layers capability {capability:?} selected {selected}, not {required}")]
    ImplementationMismatch {
        capability: CapabilityId,
        selected: String,
        required: String,
    },
    #[error("invalid bitbake-layers request: {0}")]
    InvalidRequest(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
                                    format!(
                                        "Current bitbake-layers does not expose {}.",
                                        id.as_str()
                                    ),
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

    fn all_records() -> [(CapabilityId, &'static str, bool); 5] {
        [
            (
                CapabilityId::BitBakeLayersShowLayers,
                BITBAKE_LAYERS_SHOW_IMPLEMENTATION,
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

    #[test]
    fn compatibility_layers_generates_exact_argv_for_every_operation() {
        let build = Path::new("/work/build");
        let executable = Path::new("/work/poky/bitbake/bin/bitbake-layers");
        let authority = authority(build, executable, 11, &all_records());
        let planner = BitBakeLayersCommandPlanner::new(&authority, 11, build, executable).unwrap();
        let cases = [
            (BitBakeLayersOperation::ShowLayers, vec!["show-layers"]),
            (
                BitBakeLayersOperation::CreateLayer {
                    directory: "/layers/meta-demo".into(),
                    add: false,
                },
                vec!["create-layer", "/layers/meta-demo"],
            ),
            (
                BitBakeLayersOperation::CreateLayer {
                    directory: "/layers/meta-demo".into(),
                    add: true,
                },
                vec!["create-layer", "--add-layer", "/layers/meta-demo"],
            ),
            (
                BitBakeLayersOperation::AddLayers {
                    directories: vec!["/layers/meta-one".into(), "/layers/meta-two".into()],
                },
                vec!["add-layer", "/layers/meta-one", "/layers/meta-two"],
            ),
            (
                BitBakeLayersOperation::RemoveLayers {
                    directories: vec!["/layers/meta-old".into()],
                },
                vec!["remove-layer", "/layers/meta-old"],
            ),
        ];
        for (operation, expected) in cases {
            let command = planner.operation(&operation).unwrap();
            assert_eq!(command.executable(), executable);
            assert_eq!(command.arguments(), expected);
            assert_eq!(command.generation(), 11);
        }
    }

    #[test]
    fn compatibility_layers_old_surface_disables_only_absent_mutations_and_options() {
        let build = Path::new("/work/build");
        let executable = Path::new("/work/poky/bitbake/bin/bitbake-layers");
        let records = [
            (
                CapabilityId::BitBakeLayersShowLayers,
                BITBAKE_LAYERS_SHOW_IMPLEMENTATION,
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
                false,
            ),
            (
                CapabilityId::BitBakeLayersRemoveLayer,
                BITBAKE_LAYERS_REMOVE_IMPLEMENTATION,
                false,
            ),
        ];
        let authority = authority(build, executable, 12, &records);
        let planner = BitBakeLayersCommandPlanner::new(&authority, 12, build, executable).unwrap();
        planner
            .operation(&BitBakeLayersOperation::ShowLayers)
            .unwrap();
        planner
            .operation(&BitBakeLayersOperation::CreateLayer {
                directory: "/layers/meta-demo".into(),
                add: false,
            })
            .unwrap();
        assert!(matches!(
            planner.operation(&BitBakeLayersOperation::CreateLayer {
                directory: "/layers/meta-demo".into(),
                add: true
            }),
            Err(BitBakeLayersCompatibilityError::Unavailable {
                capability: CapabilityId::BitBakeLayersCreateAndAddLayer,
                ..
            })
        ));
        assert!(matches!(
            planner.operation(&BitBakeLayersOperation::AddLayers {
                directories: vec!["/layers/meta-demo".into()]
            }),
            Err(BitBakeLayersCompatibilityError::CapabilityMissing {
                capability: CapabilityId::BitBakeLayersAddLayer
            })
        ));
    }

    #[test]
    fn compatibility_layers_rejects_stale_environment_executable_and_cross_command() {
        let build = Path::new("/work/build");
        let executable = Path::new("/work/poky/bitbake/bin/bitbake-layers");
        let authority = authority(
            build,
            executable,
            13,
            &[(
                CapabilityId::BitBakeLayersShowLayers,
                BITBAKE_LAYERS_SHOW_IMPLEMENTATION,
                true,
            )],
        );
        assert!(matches!(
            BitBakeLayersCommandPlanner::new(&authority, 12, build, executable),
            Err(BitBakeLayersCompatibilityError::StaleGeneration { .. })
        ));
        assert!(matches!(
            BitBakeLayersCommandPlanner::new(&authority, 13, Path::new("/other"), executable),
            Err(BitBakeLayersCompatibilityError::EnvironmentMismatch)
        ));
        assert!(matches!(
            BitBakeLayersCommandPlanner::new(
                &authority,
                13,
                build,
                Path::new("/usr/bin/bitbake-layers")
            ),
            Err(BitBakeLayersCompatibilityError::ExecutableMismatch)
        ));
        let planner = BitBakeLayersCommandPlanner::new(&authority, 13, build, executable).unwrap();
        assert!(matches!(
            planner.operation(&BitBakeLayersOperation::RemoveLayers {
                directories: vec!["/layers/meta-old".into()]
            }),
            Err(BitBakeLayersCompatibilityError::CapabilityMissing {
                capability: CapabilityId::BitBakeLayersRemoveLayer
            })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn compatibility_layers_unavailable_mutation_never_spawns_process() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!(
            "yoctui-compatibility-layers-no-spawn-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("bitbake-layers");
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
            14,
            &[(
                CapabilityId::BitBakeLayersRemoveLayer,
                BITBAKE_LAYERS_REMOVE_IMPLEMENTATION,
                false,
            )],
        );
        let planner = BitBakeLayersCommandPlanner::new(&authority, 14, &root, &executable).unwrap();
        assert!(
            planner
                .operation(&BitBakeLayersOperation::RemoveLayers {
                    directories: vec!["/layers/meta-old".into()]
                })
                .is_err()
        );
        assert!(!marker.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
