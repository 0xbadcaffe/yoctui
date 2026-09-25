use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use thiserror::Error;
use yoctui_model::{BitBakeLayersOperation, CapabilityId, DaemonCompatibilitySnapshot};

pub const BITBAKE_LAYERS_SHOW_IMPLEMENTATION: &str = "bitbake_layers.show_layers.argv";
pub const BITBAKE_LAYERS_SHOW_RECIPES_IMPLEMENTATION: &str = "bitbake_layers.show_recipes.argv";
pub const BITBAKE_LAYERS_SHOW_OVERLAYED_IMPLEMENTATION: &str = "bitbake_layers.show_overlayed.argv";
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
            BitBakeLayersOperation::ShowRecipes { pattern } => self.command(
                CapabilityId::BitBakeLayersShowRecipes,
                BITBAKE_LAYERS_SHOW_RECIPES_IMPLEMENTATION,
                std::iter::once(OsString::from("show-recipes"))
                    .chain(pattern.iter().map(OsString::from))
                    .collect(),
            ),
            BitBakeLayersOperation::ShowOverlayed => self.command(
                CapabilityId::BitBakeLayersShowOverlayed,
                BITBAKE_LAYERS_SHOW_OVERLAYED_IMPLEMENTATION,
                vec!["show-overlayed".into()],
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
#[path = "tests/compatibility_layers/mod.rs"]
mod tests;
