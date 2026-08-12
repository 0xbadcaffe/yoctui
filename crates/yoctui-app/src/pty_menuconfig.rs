use std::{
    collections::{BTreeSet, HashSet},
    fs,
    path::PathBuf,
};
use thiserror::Error;
use yoctui_model::{PtyCommandIdentity, PtySessionKind, PtyWorkspaceContext, RecipeIdentity};

use crate::{PtyContextAction, PtyContextAuthority, PtyContextError};

const MAX_RECIPES: usize = 16_384;
const MAX_TASKS_PER_RECIPE: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PtyBitBakeInteractiveTask {
    Menuconfig,
    Devshell,
    Nconfig,
    Xconfig,
}

impl PtyBitBakeInteractiveTask {
    pub const fn task_name(self) -> &'static str {
        match self {
            Self::Menuconfig => "menuconfig",
            Self::Devshell => "devshell",
            Self::Nconfig => "nconfig",
            Self::Xconfig => "xconfig",
        }
    }

    fn kind(self) -> PtySessionKind {
        match self {
            Self::Devshell => PtySessionKind::Devshell,
            Self::Menuconfig | Self::Nconfig | Self::Xconfig => PtySessionKind::Menuconfig,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyInteractiveRecipe {
    pub identity: RecipeIdentity,
    pub tasks: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyMenuconfigAction {
    RecipeTask {
        recipe: RecipeIdentity,
        task: PtyBitBakeInteractiveTask,
    },
    KernelMenuconfig,
    UBootMenuconfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyMenuconfigPreview {
    pub action: PtyMenuconfigAction,
    pub recipe: RecipeIdentity,
    pub task: PtyBitBakeInteractiveTask,
    pub name: String,
    pub kind: PtySessionKind,
    pub cwd: PathBuf,
    pub command: PtyCommandIdentity,
    pub environment_identity: String,
    pub environment: std::collections::BTreeMap<String, String>,
    pub workspace: PtyWorkspaceContext,
}

pub struct PtyMenuconfigRouter {
    contexts: PtyContextAuthority,
    executable: PathBuf,
    recipes: Vec<PtyInteractiveRecipe>,
    kernel: Option<RecipeIdentity>,
    uboot: Option<RecipeIdentity>,
}

impl PtyMenuconfigRouter {
    pub fn new(
        contexts: PtyContextAuthority,
        executable: PathBuf,
        recipes: Vec<PtyInteractiveRecipe>,
        kernel: Option<RecipeIdentity>,
        uboot: Option<RecipeIdentity>,
    ) -> Result<Self, PtyMenuconfigError> {
        if recipes.len() > MAX_RECIPES {
            return Err(PtyMenuconfigError::CatalogTooLarge);
        }
        let executable =
            fs::canonicalize(&executable).map_err(|error| PtyMenuconfigError::Executable {
                path: executable,
                message: error.to_string(),
            })?;
        if !is_executable_file(&executable) {
            return Err(PtyMenuconfigError::Executable {
                path: executable,
                message: "not an executable regular file".into(),
            });
        }
        let mut identities = HashSet::new();
        for recipe in &recipes {
            validate_recipe(&recipe.identity)?;
            if recipe.tasks.len() > MAX_TASKS_PER_RECIPE {
                return Err(PtyMenuconfigError::TaskCatalogTooLarge(
                    recipe.identity.name.clone(),
                ));
            }
            if !identities.insert(recipe.identity.clone()) {
                return Err(PtyMenuconfigError::DuplicateRecipe(recipe.identity.clone()));
            }
        }
        for provider in kernel.iter().chain(uboot.iter()) {
            if !identities.contains(provider) {
                return Err(PtyMenuconfigError::StaleRecipe(provider.clone()));
            }
        }
        Ok(Self {
            contexts,
            executable,
            recipes,
            kernel,
            uboot,
        })
    }

    pub fn preview(
        &self,
        action: PtyMenuconfigAction,
    ) -> Result<PtyMenuconfigPreview, PtyMenuconfigError> {
        let (recipe, task) = match &action {
            PtyMenuconfigAction::RecipeTask { recipe, task } => (recipe.clone(), *task),
            PtyMenuconfigAction::KernelMenuconfig => (
                self.kernel
                    .clone()
                    .ok_or(PtyMenuconfigError::KernelProviderUnavailable)?,
                PtyBitBakeInteractiveTask::Menuconfig,
            ),
            PtyMenuconfigAction::UBootMenuconfig => (
                self.uboot
                    .clone()
                    .ok_or(PtyMenuconfigError::UBootProviderUnavailable)?,
                PtyBitBakeInteractiveTask::Menuconfig,
            ),
        };
        let catalog = self
            .recipes
            .iter()
            .find(|candidate| candidate.identity == recipe)
            .ok_or_else(|| PtyMenuconfigError::StaleRecipe(recipe.clone()))?;
        if !catalog.tasks.contains(task.task_name()) {
            return Err(PtyMenuconfigError::TaskUnavailable {
                recipe: recipe.name.clone(),
                task,
            });
        }
        let launch = self.contexts.resolve(PtyContextAction::BuildDirectory)?;
        Ok(PtyMenuconfigPreview {
            action,
            recipe: recipe.clone(),
            task,
            name: format!("{} {}", recipe.name, task.task_name()),
            kind: task.kind(),
            cwd: launch.cwd,
            command: PtyCommandIdentity {
                executable: self.executable.clone(),
                arguments: vec!["-c".into(), task.task_name().into(), recipe.name.clone()],
            },
            environment_identity: launch.environment_identity,
            environment: launch.environment,
            workspace: launch.workspace,
        })
    }
}

fn validate_recipe(identity: &RecipeIdentity) -> Result<(), PtyMenuconfigError> {
    if identity.name.is_empty()
        || identity.name.len() > 255
        || !identity
            .name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"+_.-/".contains(&byte))
        || !identity.file.is_absolute()
    {
        return Err(PtyMenuconfigError::InvalidRecipe(identity.clone()));
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
pub enum PtyMenuconfigError {
    #[error("BitBake executable {path} is unavailable: {message}")]
    Executable { path: PathBuf, message: String },
    #[error("interactive recipe catalog exceeds configured bounds")]
    CatalogTooLarge,
    #[error("interactive task catalog exceeds bounds for {0}")]
    TaskCatalogTooLarge(String),
    #[error("duplicate authoritative recipe identity: {0:?}")]
    DuplicateRecipe(RecipeIdentity),
    #[error("invalid authoritative recipe identity: {0:?}")]
    InvalidRecipe(RecipeIdentity),
    #[error("stale authoritative recipe identity: {0:?}")]
    StaleRecipe(RecipeIdentity),
    #[error("interactive task {task:?} is unavailable for {recipe}")]
    TaskUnavailable {
        recipe: String,
        task: PtyBitBakeInteractiveTask,
    },
    #[error("authoritative kernel provider is unavailable")]
    KernelProviderUnavailable,
    #[error("authoritative U-Boot provider is unavailable")]
    UBootProviderUnavailable,
    #[error(transparent)]
    Context(#[from] PtyContextError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VerifiedPtyEnvironment;
    use std::{collections::BTreeMap, io::Write as _};

    fn fixture() -> (PathBuf, PtyMenuconfigRouter, RecipeIdentity, RecipeIdentity) {
        let nonce = std::time::SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "yoctui-pty-menuconfig-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir_all(root.join("build")).unwrap();
        let bitbake = root.join("bitbake");
        let mut file = fs::File::create(&bitbake).unwrap();
        writeln!(file, "#!/bin/sh").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&bitbake, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let contexts = PtyContextAuthority::new(
            "workspace".into(),
            root.join("source"),
            root.join("build"),
            VerifiedPtyEnvironment {
                identity: "build-env".into(),
                shell: fs::canonicalize("/bin/sh").unwrap(),
                environment: BTreeMap::new(),
            },
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let kernel = RecipeIdentity {
            name: "virtual/kernel".into(),
            file: root.join("source/linux.bb"),
        };
        let uboot = RecipeIdentity {
            name: "u-boot".into(),
            file: root.join("source/u-boot.bb"),
        };
        let recipes = vec![
            PtyInteractiveRecipe {
                identity: kernel.clone(),
                tasks: BTreeSet::from(["menuconfig".into(), "devshell".into(), "nconfig".into()]),
            },
            PtyInteractiveRecipe {
                identity: uboot.clone(),
                tasks: BTreeSet::from(["menuconfig".into()]),
            },
        ];
        let router = PtyMenuconfigRouter::new(
            contexts,
            bitbake,
            recipes,
            Some(kernel.clone()),
            Some(uboot.clone()),
        )
        .unwrap();
        (root, router, kernel, uboot)
    }

    #[test]
    fn pty_menuconfig_previews_exact_recipe_kernel_uboot_and_devshell_argv() {
        let (root, router, kernel, uboot) = fixture();
        let cases = [
            (
                PtyMenuconfigAction::KernelMenuconfig,
                kernel.clone(),
                "menuconfig",
                PtySessionKind::Menuconfig,
            ),
            (
                PtyMenuconfigAction::UBootMenuconfig,
                uboot,
                "menuconfig",
                PtySessionKind::Menuconfig,
            ),
            (
                PtyMenuconfigAction::RecipeTask {
                    recipe: kernel.clone(),
                    task: PtyBitBakeInteractiveTask::Devshell,
                },
                kernel.clone(),
                "devshell",
                PtySessionKind::Devshell,
            ),
            (
                PtyMenuconfigAction::RecipeTask {
                    recipe: kernel.clone(),
                    task: PtyBitBakeInteractiveTask::Nconfig,
                },
                kernel,
                "nconfig",
                PtySessionKind::Menuconfig,
            ),
        ];
        for (action, recipe, task, kind) in cases {
            let preview = router.preview(action).unwrap();
            assert_eq!(preview.command.arguments, vec!["-c", task, &recipe.name]);
            assert_eq!(preview.kind, kind);
            assert_eq!(preview.cwd, fs::canonicalize(root.join("build")).unwrap());
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pty_menuconfig_rejects_stale_recipe_and_unavailable_task() {
        let (root, router, kernel, _) = fixture();
        let stale = RecipeIdentity {
            name: kernel.name.clone(),
            file: root.join("source/other.bb"),
        };
        assert!(matches!(
            router.preview(PtyMenuconfigAction::RecipeTask {
                recipe: stale,
                task: PtyBitBakeInteractiveTask::Menuconfig
            }),
            Err(PtyMenuconfigError::StaleRecipe(_))
        ));
        assert_eq!(
            router.preview(PtyMenuconfigAction::RecipeTask {
                recipe: kernel,
                task: PtyBitBakeInteractiveTask::Xconfig
            }),
            Err(PtyMenuconfigError::TaskUnavailable {
                recipe: "virtual/kernel".into(),
                task: PtyBitBakeInteractiveTask::Xconfig
            })
        );
        fs::remove_dir_all(root).unwrap();
    }
}
