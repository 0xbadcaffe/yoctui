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
#[path = "tests/compatibility_command/mod.rs"]
mod tests;
