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
#[path = "tests/compatibility_recipetool/mod.rs"]
mod tests;
