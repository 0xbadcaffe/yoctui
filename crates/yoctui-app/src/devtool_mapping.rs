use std::path::PathBuf;

use yoctui_model::{
    DevtoolCapability, DevtoolGitState, DevtoolStatus, DevtoolStatusError, DevtoolWorkspace,
    RecipeIdentity,
};
use yoctui_protocol::daemon::{
    DaemonDevtoolCapabilityData, DaemonDevtoolGitData, DaemonDevtoolStatusData,
    DaemonDevtoolStatusErrorData, DaemonDevtoolWorkspaceData,
};

pub fn devtool_status_to_protocol(status: &DevtoolStatus) -> DaemonDevtoolStatusData {
    DaemonDevtoolStatusData {
        recipe: status.identity.name.clone(),
        recipe_file: status.identity.file.display().to_string(),
        capability: match &status.capability {
            DevtoolCapability::Available => DaemonDevtoolCapabilityData::Available,
            DevtoolCapability::MissingExecutable => DaemonDevtoolCapabilityData::MissingExecutable,
            DevtoolCapability::Unavailable { reason } => DaemonDevtoolCapabilityData::Unavailable {
                reason: reason.clone(),
            },
        },
        workspace: match &status.workspace {
            DevtoolWorkspace::NotMember => DaemonDevtoolWorkspaceData::NotMember,
            DevtoolWorkspace::MissingDirectory { source_path } => {
                DaemonDevtoolWorkspaceData::MissingDirectory {
                    source_path: source_path.display().to_string(),
                }
            }
            DevtoolWorkspace::Present {
                source_path,
                recipe_file,
            } => DaemonDevtoolWorkspaceData::Present {
                source_path: source_path.display().to_string(),
                recipe_file: recipe_file.as_ref().map(|path| path.display().to_string()),
            },
        },
        git: match &status.git {
            DevtoolGitState::NotApplicable => DaemonDevtoolGitData::NotApplicable,
            DevtoolGitState::MissingExecutable => DaemonDevtoolGitData::MissingExecutable,
            DevtoolGitState::NotRepository => DaemonDevtoolGitData::NotRepository,
            DevtoolGitState::Available {
                branch,
                head,
                modified,
                untracked,
                conflicted,
            } => DaemonDevtoolGitData::Available {
                branch: branch.clone(),
                head: head.clone(),
                modified: *modified as u64,
                untracked: *untracked as u64,
                conflicted: *conflicted as u64,
            },
            DevtoolGitState::Failed { exit_code, message } => DaemonDevtoolGitData::Failed {
                exit_code: *exit_code,
                message: message.clone(),
            },
            DevtoolGitState::Malformed { message } => DaemonDevtoolGitData::Malformed {
                message: message.clone(),
            },
        },
        error: status.error.as_ref().map(|error| match error {
            DevtoolStatusError::InvalidRecipeIdentity => {
                DaemonDevtoolStatusErrorData::InvalidRecipeIdentity
            }
            DevtoolStatusError::DevtoolFailed { exit_code, message } => {
                DaemonDevtoolStatusErrorData::DevtoolFailed {
                    exit_code: *exit_code,
                    message: message.clone(),
                }
            }
            DevtoolStatusError::MalformedOutput { line } => {
                DaemonDevtoolStatusErrorData::MalformedOutput { line: line.clone() }
            }
        }),
    }
}

pub fn devtool_status_from_protocol(
    status: &DaemonDevtoolStatusData,
) -> Result<DevtoolStatus, String> {
    let count = |value: u64, label: &str| {
        usize::try_from(value).map_err(|_| format!("Devtool {label} count exceeds platform bounds"))
    };
    Ok(DevtoolStatus {
        identity: RecipeIdentity {
            name: status.recipe.clone(),
            file: PathBuf::from(&status.recipe_file),
        },
        capability: match &status.capability {
            DaemonDevtoolCapabilityData::Available => DevtoolCapability::Available,
            DaemonDevtoolCapabilityData::MissingExecutable => DevtoolCapability::MissingExecutable,
            DaemonDevtoolCapabilityData::Unavailable { reason } => DevtoolCapability::Unavailable {
                reason: reason.clone(),
            },
        },
        workspace: match &status.workspace {
            DaemonDevtoolWorkspaceData::NotMember => DevtoolWorkspace::NotMember,
            DaemonDevtoolWorkspaceData::MissingDirectory { source_path } => {
                DevtoolWorkspace::MissingDirectory {
                    source_path: PathBuf::from(source_path),
                }
            }
            DaemonDevtoolWorkspaceData::Present {
                source_path,
                recipe_file,
            } => DevtoolWorkspace::Present {
                source_path: PathBuf::from(source_path),
                recipe_file: recipe_file.as_ref().map(PathBuf::from),
            },
        },
        git: match &status.git {
            DaemonDevtoolGitData::NotApplicable => DevtoolGitState::NotApplicable,
            DaemonDevtoolGitData::MissingExecutable => DevtoolGitState::MissingExecutable,
            DaemonDevtoolGitData::NotRepository => DevtoolGitState::NotRepository,
            DaemonDevtoolGitData::Available {
                branch,
                head,
                modified,
                untracked,
                conflicted,
            } => DevtoolGitState::Available {
                branch: branch.clone(),
                head: head.clone(),
                modified: count(*modified, "modified")?,
                untracked: count(*untracked, "untracked")?,
                conflicted: count(*conflicted, "conflicted")?,
            },
            DaemonDevtoolGitData::Failed { exit_code, message } => DevtoolGitState::Failed {
                exit_code: *exit_code,
                message: message.clone(),
            },
            DaemonDevtoolGitData::Malformed { message } => DevtoolGitState::Malformed {
                message: message.clone(),
            },
        },
        error: status.error.as_ref().map(|error| match error {
            DaemonDevtoolStatusErrorData::InvalidRecipeIdentity => {
                DevtoolStatusError::InvalidRecipeIdentity
            }
            DaemonDevtoolStatusErrorData::DevtoolFailed { exit_code, message } => {
                DevtoolStatusError::DevtoolFailed {
                    exit_code: *exit_code,
                    message: message.clone(),
                }
            }
            DaemonDevtoolStatusErrorData::MalformedOutput { line } => {
                DevtoolStatusError::MalformedOutput { line: line.clone() }
            }
        }),
    })
}
