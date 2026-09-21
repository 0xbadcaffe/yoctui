use std::{fs, path::PathBuf};

use thiserror::Error;
use yoctui_bitbake::{DevtoolCommandPlanner, DevtoolCompatibilityError};
use yoctui_model::{
    DaemonCompatibilitySnapshot, DevtoolCapability, DevtoolStatus, DevtoolWorkspace,
    PtyCommandIdentity, PtySessionKind, PtyWorkspaceContext, RecipeIdentity,
};

use crate::{PtyContextAction, PtyContextAuthority, PtyContextError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyDevtoolAction {
    WorkspaceShell { workspace_identity: String },
    EditRecipe,
    Modify,
    UpdateRecipe,
    Finish,
    Deploy,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyDevtoolPreview {
    pub action: PtyDevtoolAction,
    pub recipe: RecipeIdentity,
    pub name: String,
    pub kind: PtySessionKind,
    pub cwd: PathBuf,
    pub command: PtyCommandIdentity,
    pub environment_identity: String,
    pub environment: std::collections::BTreeMap<String, String>,
    pub workspace: PtyWorkspaceContext,
}

pub struct PtyDevtoolRouter {
    contexts: PtyContextAuthority,
    executable: PathBuf,
    compatibility: DaemonCompatibilitySnapshot,
}

impl PtyDevtoolRouter {
    pub fn new(
        contexts: PtyContextAuthority,
        executable: PathBuf,
        compatibility: DaemonCompatibilitySnapshot,
    ) -> Result<Self, PtyDevtoolError> {
        let executable =
            fs::canonicalize(&executable).map_err(|error| PtyDevtoolError::Executable {
                path: executable,
                message: error.to_string(),
            })?;
        if !is_executable_file(&executable) {
            return Err(PtyDevtoolError::Executable {
                path: executable,
                message: "not an executable regular file".into(),
            });
        }
        Ok(Self {
            contexts,
            executable,
            compatibility,
        })
    }

    pub fn preview(
        &self,
        status: &DevtoolStatus,
        action: PtyDevtoolAction,
    ) -> Result<PtyDevtoolPreview, PtyDevtoolError> {
        validate_status(status)?;
        match &action {
            PtyDevtoolAction::WorkspaceShell { workspace_identity } => {
                let source_path = match &status.workspace {
                    DevtoolWorkspace::Present { source_path, .. } => source_path,
                    DevtoolWorkspace::NotMember | DevtoolWorkspace::MissingDirectory { .. } => {
                        return Err(PtyDevtoolError::WorkspaceUnavailable);
                    }
                };
                let launch = self.contexts.resolve(PtyContextAction::DevtoolWorkspace {
                    identity: workspace_identity.clone(),
                })?;
                let authoritative = fs::canonicalize(source_path).map_err(|error| {
                    PtyDevtoolError::WorkspacePath {
                        path: source_path.clone(),
                        message: error.to_string(),
                    }
                })?;
                if launch.cwd != authoritative {
                    return Err(PtyDevtoolError::StaleWorkspace);
                }
                Ok(PtyDevtoolPreview {
                    action,
                    recipe: status.identity.clone(),
                    name: launch.name,
                    kind: launch.kind,
                    cwd: launch.cwd,
                    command: launch.command,
                    environment_identity: launch.environment_identity,
                    environment: launch.environment,
                    workspace: launch.workspace,
                })
            }
            PtyDevtoolAction::EditRecipe => {
                validate_recipe(&status.identity)?;
                let launch = self.contexts.resolve(PtyContextAction::BuildDirectory)?;
                let command = DevtoolCommandPlanner::new(
                    &self.compatibility,
                    self.compatibility.snapshot.generation,
                    &launch.cwd,
                    &self.executable,
                )?
                .edit_recipe(&status.identity.name)?;
                Ok(PtyDevtoolPreview {
                    action,
                    recipe: status.identity.clone(),
                    name: format!("Edit recipe {}", status.identity.name),
                    kind: PtySessionKind::InteractiveTool,
                    cwd: launch.cwd,
                    command: PtyCommandIdentity {
                        executable: command.executable().to_owned(),
                        arguments: command
                            .arguments()
                            .iter()
                            .map(|argument| argument.to_string_lossy().into_owned())
                            .collect(),
                    },
                    environment_identity: launch.environment_identity,
                    environment: launch.environment,
                    workspace: launch.workspace,
                })
            }
            PtyDevtoolAction::Modify
            | PtyDevtoolAction::UpdateRecipe
            | PtyDevtoolAction::Finish
            | PtyDevtoolAction::Deploy
            | PtyDevtoolAction::Reset => Err(PtyDevtoolError::UseBackgroundJob),
        }
    }
}

fn validate_status(status: &DevtoolStatus) -> Result<(), PtyDevtoolError> {
    if status.capability != DevtoolCapability::Available {
        return Err(PtyDevtoolError::Unavailable);
    }
    if status.error.is_some() {
        return Err(PtyDevtoolError::StaleStatus);
    }
    Ok(())
}

fn validate_recipe(identity: &RecipeIdentity) -> Result<(), PtyDevtoolError> {
    if identity.name.is_empty()
        || identity.name.len() > 255
        || !identity
            .name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"+_.-".contains(&byte))
        || !identity.file.is_absolute()
    {
        return Err(PtyDevtoolError::InvalidRecipe);
    }
    Ok(())
}

fn is_executable_file(path: &std::path::Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        metadata.is_file()
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PtyDevtoolError {
    #[error("Devtool executable {path} is unavailable: {message}")]
    Executable { path: PathBuf, message: String },
    #[error("Devtool capability is unavailable")]
    Unavailable,
    #[error("authoritative Devtool status must be refreshed")]
    StaleStatus,
    #[error("Devtool workspace source is unavailable")]
    WorkspaceUnavailable,
    #[error("Devtool workspace path {path} is unavailable: {message}")]
    WorkspacePath { path: PathBuf, message: String },
    #[error("Devtool workspace identity/path changed; refresh before opening a PTY")]
    StaleWorkspace,
    #[error("invalid authoritative recipe identity")]
    InvalidRecipe,
    #[error("this Devtool action remains a managed noninteractive background job")]
    UseBackgroundJob,
    #[error("Devtool compatibility: {0}")]
    Compatibility(#[from] DevtoolCompatibilityError),
    #[error(transparent)]
    Context(#[from] PtyContextError),
}

#[cfg(test)]
#[path = "tests/pty_devtool/mod.rs"]
mod tests;
