use std::{fs, path::PathBuf};

use thiserror::Error;
use yoctui_model::{
    DevtoolCapability, DevtoolStatus, DevtoolWorkspace, PtyCommandIdentity, PtySessionKind,
    PtyWorkspaceContext, RecipeIdentity,
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
}

impl PtyDevtoolRouter {
    pub fn new(
        contexts: PtyContextAuthority,
        executable: PathBuf,
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
                Ok(PtyDevtoolPreview {
                    action,
                    recipe: status.identity.clone(),
                    name: format!("Edit recipe {}", status.identity.name),
                    kind: PtySessionKind::InteractiveTool,
                    cwd: launch.cwd,
                    command: PtyCommandIdentity {
                        executable: self.executable.clone(),
                        arguments: vec!["edit-recipe".into(), status.identity.name.clone()],
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
    #[error(transparent)]
    Context(#[from] PtyContextError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PtyContextEntry, VerifiedPtyEnvironment};
    use std::{collections::BTreeMap, io::Write as _};
    use yoctui_model::{DevtoolGitState, DevtoolStatusError};

    fn fixture() -> (PathBuf, PtyDevtoolRouter, DevtoolStatus) {
        let nonce = std::time::SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("yoctui-pty-devtool-{}-{nonce}", std::process::id()));
        for path in ["source", "build", "workspace/busybox"] {
            fs::create_dir_all(root.join(path)).unwrap();
        }
        let executable = root.join("devtool");
        let mut file = fs::File::create(&executable).unwrap();
        writeln!(file, "#!/bin/sh").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let environment = VerifiedPtyEnvironment {
            identity: "build-env".into(),
            shell: fs::canonicalize("/bin/sh").unwrap(),
            environment: BTreeMap::from([(
                "BUILDDIR".into(),
                root.join("build").display().to_string(),
            )]),
        };
        let contexts = PtyContextAuthority::new(
            "workspace".into(),
            root.join("source"),
            root.join("build"),
            environment,
            Vec::new(),
            Vec::new(),
            vec![PtyContextEntry {
                identity: "busybox-workspace".into(),
                directory: root.join("workspace/busybox"),
            }],
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let router = PtyDevtoolRouter::new(contexts, executable).unwrap();
        let status = DevtoolStatus {
            identity: RecipeIdentity {
                name: "busybox".into(),
                file: root.join("source/meta/recipes-core/busybox.bb"),
            },
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: root.join("workspace/busybox"),
                recipe_file: None,
            },
            git: DevtoolGitState::Available {
                branch: Some("devtool".into()),
                head: Some("abc".into()),
                modified: 0,
                untracked: 0,
                conflicted: 0,
            },
            error: None,
        };
        (root, router, status)
    }

    #[test]
    fn pty_devtool_previews_workspace_shell_and_exact_edit_recipe() {
        let (root, router, status) = fixture();
        let workspace = router
            .preview(
                &status,
                PtyDevtoolAction::WorkspaceShell {
                    workspace_identity: "busybox-workspace".into(),
                },
            )
            .unwrap();
        assert_eq!(workspace.kind, PtySessionKind::DevtoolShell);
        assert_eq!(
            workspace.cwd,
            fs::canonicalize(root.join("workspace/busybox")).unwrap()
        );
        assert_eq!(workspace.command.arguments, vec!["-i"]);
        let edit = router
            .preview(&status, PtyDevtoolAction::EditRecipe)
            .unwrap();
        assert_eq!(edit.kind, PtySessionKind::InteractiveTool);
        assert_eq!(edit.command.arguments, vec!["edit-recipe", "busybox"]);
        assert_eq!(edit.cwd, fs::canonicalize(root.join("build")).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pty_devtool_keeps_noninteractive_actions_on_existing_job_path() {
        let (root, router, status) = fixture();
        for action in [
            PtyDevtoolAction::Modify,
            PtyDevtoolAction::UpdateRecipe,
            PtyDevtoolAction::Finish,
            PtyDevtoolAction::Deploy,
            PtyDevtoolAction::Reset,
        ] {
            assert_eq!(
                router.preview(&status, action),
                Err(PtyDevtoolError::UseBackgroundJob)
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pty_devtool_rejects_stale_status_workspace_and_recipe() {
        let (root, router, mut status) = fixture();
        status.error = Some(DevtoolStatusError::MalformedOutput { line: "bad".into() });
        assert_eq!(
            router.preview(&status, PtyDevtoolAction::EditRecipe),
            Err(PtyDevtoolError::StaleStatus)
        );
        status.error = None;
        status.identity.name = "bad recipe;touch".into();
        assert_eq!(
            router.preview(&status, PtyDevtoolAction::EditRecipe),
            Err(PtyDevtoolError::InvalidRecipe)
        );
        status.identity.name = "busybox".into();
        status.workspace = DevtoolWorkspace::Present {
            source_path: root.join("source"),
            recipe_file: None,
        };
        assert_eq!(
            router.preview(
                &status,
                PtyDevtoolAction::WorkspaceShell {
                    workspace_identity: "busybox-workspace".into()
                }
            ),
            Err(PtyDevtoolError::StaleWorkspace)
        );
        fs::remove_dir_all(root).unwrap();
    }
}
