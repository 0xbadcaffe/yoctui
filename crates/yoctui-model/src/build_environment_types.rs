//! Build environment types.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildStatus {
    Idle,
    LoadingWorkspace,
    Parsing,
    Running,
    Cancelling,
    Completed,
    Cancelled,
    Failed,
    Lost,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Trace,
    Info,
    Warning,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRequest {
    pub targets: Vec<String>,
    pub task: Option<String>,
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildEnvironmentProfile {
    pub source_dir: PathBuf,
    pub build_dir: PathBuf,
    pub init_script: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildEnvironmentCloneRequest {
    pub repository: String,
    pub destination: PathBuf,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildEnvironmentClonePlan {
    pub request: BuildEnvironmentCloneRequest,
    pub build_dir: PathBuf,
}

impl BuildEnvironmentClonePlan {
    pub fn validate(&self) -> Result<(), AppError> {
        self.request.validate()?;
        if !self.build_dir.is_absolute()
            || self
                .build_dir
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(AppError::new(
                "Build environment",
                "invalid clone build directory",
                "provide an absolute build directory",
            ));
        }
        Ok(())
    }
}

impl BuildEnvironmentCloneRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        let valid_revision = |value: &str| {
            !value.is_empty()
                && value.len() <= 256
                && value.chars().all(|character| {
                    character.is_ascii_alphanumeric()
                        || matches!(character, '-' | '_' | '.' | '/' | '@' | ':')
                })
        };
        if self.repository.is_empty()
            || self
                .repository
                .chars()
                .any(|character| character.is_ascii_control())
            || !self.destination.is_absolute()
            || self
                .destination
                .components()
                .any(|part| matches!(part, Component::ParentDir))
            || self
                .revision
                .as_deref()
                .is_some_and(|value| !valid_revision(value))
        {
            return Err(AppError::new(
                "Build environment",
                "invalid Poky clone request",
                "provide a repository and an absolute empty destination",
            ));
        }
        Ok(())
    }
}

impl BuildEnvironmentProfile {
    pub fn validate(&self) -> Result<(), AppError> {
        let valid = |path: &Path| {
            path.is_absolute()
                && !path
                    .components()
                    .any(|part| matches!(part, Component::ParentDir))
        };
        if valid(&self.source_dir) && valid(&self.build_dir) && valid(&self.init_script) {
            Ok(())
        } else {
            Err(AppError::new(
                "Build environment",
                "source, build directory, and environment script must be absolute normal paths",
                "choose absolute paths in Build environment settings",
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BuildEnvironmentState {
    #[default]
    Unconfigured,
    Configured(BuildEnvironmentProfile),
    Verifying {
        profile: BuildEnvironmentProfile,
        generation: u64,
    },
    Connected(BuildEnvironmentProfile),
    Failed {
        profile: BuildEnvironmentProfile,
        message: String,
    },
}

impl BuildEnvironmentState {
    pub fn connected(&self) -> bool {
        matches!(self, Self::Connected(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildEnvironmentField {
    Source,
    Build,
    Script,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildEnvironmentDraft {
    pub source: String,
    pub build: String,
    pub script: String,
    pub field: BuildEnvironmentField,
    pub editing: bool,
}
impl BuildRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        let valid_name = |value: &str| {
            !value.is_empty()
                && !matches!(value, "." | "..")
                && value.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
                })
        };
        if self.targets.is_empty()
            || self.targets.iter().any(|x| !valid_name(x))
            || self.task.as_deref().is_some_and(|task| !valid_name(task))
        {
            return Err(AppError::new(
                "Configuration",
                "invalid build target",
                "pass one or more BitBake target names",
            ));
        }
        Ok(())
    }
}
