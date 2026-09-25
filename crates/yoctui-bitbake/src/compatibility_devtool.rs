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
            DevtoolOperation::UpdateRecipePatch { destination, .. } => (
                CapabilityId::DevtoolUpdateRecipe,
                DEVTOOL_UPDATE_RECIPE_IMPLEMENTATION,
                vec![
                    "update-recipe".into(),
                    "--mode".into(),
                    "patch".into(),
                    "--append".into(),
                    destination.as_os_str().to_owned(),
                    recipe,
                ],
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
#[path = "tests/compatibility_devtool/mod.rs"]
mod tests;
