//! Domain model and pure state transitions. BitBake remains authoritative.
mod bitbake_restart;
mod compatibility;
mod compatibility_catalog;
mod daemon_state;
mod embedded_shell;
mod image;
mod maintenance;
mod package;
mod pane_layout;
mod project_profile;
mod pty_multi;
mod pty_session;
mod qa;
mod qemu;
mod sdk;
mod security;
mod terminal_emulation;
mod testing;
mod utility_menu;
mod wic;

pub use bitbake_restart::*;
pub use compatibility::*;
pub use compatibility_catalog::*;
pub use daemon_state::*;
pub use embedded_shell::*;
pub use image::*;
pub use maintenance::*;
pub use package::*;
pub use pane_layout::*;
pub use project_profile::*;
pub use pty_multi::*;
pub use pty_session::*;
pub use qa::*;
pub use qemu::*;
pub use sdk::*;
pub use security::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    fmt,
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime},
};
pub use terminal_emulation::*;
pub use testing::*;
use thiserror::Error;
pub use utility_menu::*;
pub use wic::*;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppError {
    #[error("{category}: {message}. {remedy}")]
    Message {
        category: &'static str,
        message: String,
        remedy: String,
    },
}
impl AppError {
    pub fn new(
        category: &'static str,
        message: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self::Message {
            category,
            message: message.into(),
            remedy: remedy.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    Dashboard,
    Tasks,
    BuildHistory,
    Dependencies,
    Signatures,
    LayerRelationships,
    Recipes,
    Packages,
    Images,
    Sdk,
    Testing,
    Security,
    Qa,
    Layers,
    Configuration,
    Bbmask,
    Maintenance,
    Logs,
    Errors,
    Help,
    BuildEnvironment,
    Settings,
}
/// The one active target in Yoctui's persistent workbench shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusTarget {
    Navigator,
    Workspace,
    Inspector,
    Dialog,
    CommandPalette,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    #[default]
    #[serde(alias = "dark")]
    DarkPro,
    #[serde(alias = "light")]
    WhiteClassic,
    MatrixGreen,
    VscodeDark,
    VscodeLight,
    AccessibleDark,
    SoftLight,
    HighContrast,
    /// Legacy persisted setting; new selectors expose the Packrat catalog only.
    Monochrome,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AnimationSpeed {
    Slow,
    #[default]
    Fast,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Theme,
    AnimationSpeed,
    ReducedMotion,
    Color,
    LogWrap,
    LogFollow,
}
pub const SETTINGS: [Setting; 6] = [
    Setting::Theme,
    Setting::AnimationSpeed,
    Setting::ReducedMotion,
    Setting::Color,
    Setting::LogWrap,
    Setting::LogFollow,
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    BuildImage,
    SelectImage,
    BuildSelectedRecipe,
    EditBbmask,
    OpenDashboard,
    OpenLayers,
    OpenRecipes,
    OpenImages,
    OpenTasks,
    OpenLogs,
    OpenErrors,
    OpenConfiguration,
    OpenSettings,
    ChooseTheme,
    OpenHelp,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteCommand {
    pub id: CommandId,
    pub label: &'static str,
    pub description: &'static str,
    pub shortcut: &'static str,
    pub disabled_reason: Option<&'static str>,
}
impl PaletteCommand {
    pub fn enabled(&self) -> bool {
        self.disabled_reason.is_none()
    }
}
const NAVIGATOR_SCREENS: [Screen; 18] = [
    Screen::Dashboard,
    Screen::Layers,
    Screen::Recipes,
    Screen::Packages,
    Screen::Images,
    Screen::Sdk,
    Screen::Tasks,
    Screen::Logs,
    Screen::Errors,
    Screen::Configuration,
    Screen::Dependencies,
    Screen::Testing,
    Screen::Security,
    Screen::Qa,
    Screen::Recipes,
    Screen::Maintenance,
    Screen::BuildEnvironment,
    Screen::Settings,
];
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct TaskId(pub String);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskState {
    Waiting,
    #[default]
    Active,
    Completed,
    Failed,
    Cancelled,
    Lost,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskStats {
    pub completed: usize,
    pub total: usize,
    pub active: usize,
    pub failed: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskInfo {
    pub id: TaskId,
    pub recipe: String,
    pub task: String,
    pub progress: Option<u8>,
    #[serde(default)]
    pub state: TaskState,
    #[serde(default)]
    pub worker: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub started: Option<SystemTime>,
    #[serde(default)]
    pub finished: Option<SystemTime>,
    #[serde(default)]
    pub dependencies: Vec<TaskId>,
    #[serde(default)]
    pub log_path: Option<PathBuf>,
    #[serde(default)]
    pub cancellation: Option<String>,
    #[serde(default)]
    pub stats: Option<TaskStats>,
}
impl TaskInfo {
    pub fn active(id: TaskId, recipe: String, task: String) -> Self {
        Self {
            id,
            recipe,
            task,
            progress: None,
            state: TaskState::Active,
            worker: None,
            pid: None,
            started: Some(SystemTime::now()),
            finished: None,
            dependencies: Vec::new(),
            log_path: None,
            cancellation: None,
            stats: None,
        }
    }
    pub fn elapsed_at(&self, now: SystemTime) -> Option<Duration> {
        let end = self.finished.unwrap_or(now);
        self.started
            .and_then(|started| end.duration_since(started).ok())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedTask {
    pub task: TaskInfo,
    pub success: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskStateFilter {
    #[default]
    All,
    Active,
    Waiting,
    Completed,
    Failed,
}
impl TaskStateFilter {
    fn next(self) -> Self {
        match self {
            Self::All => Self::Active,
            Self::Active => Self::Waiting,
            Self::Waiting => Self::Completed,
            Self::Completed => Self::Failed,
            Self::Failed => Self::All,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskFilterField {
    #[default]
    Recipe,
    Task,
    Worker,
}
impl TaskFilterField {
    fn next(self) -> Self {
        match self {
            Self::Recipe => Self::Task,
            Self::Task => Self::Worker,
            Self::Worker => Self::Recipe,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskFilters {
    pub state: TaskStateFilter,
    pub recipe: String,
    pub task: String,
    pub worker: String,
    pub minimum_duration: Option<Duration>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskRow {
    Task(Box<TaskInfo>),
    WaitingSummary(usize),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolFinishRequest {
    pub recipe: String,
    pub destination: PathBuf,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolFinishPicker {
    pub identity: RecipeIdentity,
    pub layers: Vec<Layer>,
    pub selection: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolFinishPlan {
    pub identity: RecipeIdentity,
    pub layer: Layer,
}
impl DevtoolFinishPlan {
    pub fn request(&self) -> DevtoolFinishRequest {
        DevtoolFinishRequest {
            recipe: self.identity.name.clone(),
            destination: self.layer.path.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolDeployRequest {
    pub recipe: String,
    pub target: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolDeployDraft {
    pub identity: RecipeIdentity,
    pub target: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolDeployPlan {
    pub identity: RecipeIdentity,
    pub target: String,
}
impl DevtoolDeployPlan {
    pub fn request(&self) -> DevtoolDeployRequest {
        DevtoolDeployRequest {
            recipe: self.identity.name.clone(),
            target: self.target.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolResetPlan {
    pub identity: RecipeIdentity,
    pub source_path: PathBuf,
}
impl DevtoolResetPlan {
    pub fn operation(&self) -> DevtoolOperation {
        DevtoolOperation::Reset {
            recipe: self.identity.name.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevtoolOperation {
    Modify {
        recipe: String,
    },
    UpdateRecipe {
        recipe: String,
    },
    Finish {
        recipe: String,
        destination: PathBuf,
    },
    DeployTarget {
        recipe: String,
        target: String,
    },
    UndeployTarget {
        recipe: String,
        target: String,
    },
    Reset {
        recipe: String,
    },
    Upgrade {
        recipe: String,
    },
}
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DevtoolOperationError {
    #[error("Devtool recipe must be one non-option value without whitespace or control characters")]
    InvalidRecipe,
    #[error("Devtool target must be one non-option value without whitespace or control characters")]
    InvalidTarget,
    #[error("Devtool finish destination must be an absolute path")]
    RelativeFinishDestination,
}
impl DevtoolOperation {
    pub fn recipe(&self) -> &str {
        match self {
            Self::Modify { recipe }
            | Self::UpdateRecipe { recipe }
            | Self::Finish { recipe, .. }
            | Self::DeployTarget { recipe, .. }
            | Self::UndeployTarget { recipe, .. }
            | Self::Reset { recipe }
            | Self::Upgrade { recipe } => recipe,
        }
    }

    pub fn validate(&self) -> Result<(), DevtoolOperationError> {
        let valid_token = |value: &str| {
            !value.is_empty()
                && !value.starts_with('-')
                && value
                    .chars()
                    .all(|character| !character.is_whitespace() && !character.is_control())
        };
        if !valid_token(self.recipe()) {
            return Err(DevtoolOperationError::InvalidRecipe);
        }
        match self {
            Self::Finish { destination, .. } if !destination.is_absolute() => {
                Err(DevtoolOperationError::RelativeFinishDestination)
            }
            Self::DeployTarget { target, .. } | Self::UndeployTarget { target, .. }
                if !valid_token(target) =>
            {
                Err(DevtoolOperationError::InvalidTarget)
            }
            _ => Ok(()),
        }
    }
}
impl From<DevtoolFinishRequest> for DevtoolOperation {
    fn from(request: DevtoolFinishRequest) -> Self {
        Self::Finish {
            recipe: request.recipe,
            destination: request.destination,
        }
    }
}
impl From<DevtoolDeployRequest> for DevtoolOperation {
    fn from(request: DevtoolDeployRequest) -> Self {
        Self::DeployTarget {
            recipe: request.recipe,
            target: request.target,
        }
    }
}
const MAX_COMPLETED_TASKS: usize = 1_024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    pub category: String,
    pub summary: String,
    #[serde(default)]
    pub event_metadata: Vec<(String, String)>,
    #[serde(default)]
    pub suggestions: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    #[serde(default)]
    pub id: u64,
    pub severity: Severity,
    pub message: String,
    pub recipe: Option<String>,
    pub task: Option<String>,
    pub path: Option<PathBuf>,
    pub timestamp: SystemTime,
    #[serde(default)]
    pub build: Option<String>,
    #[serde(default)]
    pub protected: bool,
    #[serde(default)]
    pub diagnostic: Option<DiagnosticInfo>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Workspace {
    pub build_dir: Option<PathBuf>,
    pub source_dir: Option<PathBuf>,
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub variable_provenance: HashMap<String, String>,
    #[serde(default)]
    pub variable_provenance_chain: HashMap<String, Vec<String>>,
    pub bitbake_version: Option<String>,
    #[serde(default)]
    pub release: Option<String>,
    pub layers: Vec<Layer>,
    pub recipes: Vec<Recipe>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariableIdentity {
    pub name: String,
    pub recipe: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableOperation {
    pub operation: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub value: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDetail {
    pub identity: VariableIdentity,
    pub effective_value: Option<String>,
    pub unexpanded_value: Option<String>,
    pub provenance: Option<String>,
    pub operations: Vec<VariableOperation>,
    pub active_overrides: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigCopyValue {
    Effective,
    Unexpanded,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostTelemetry {
    pub cpu_utilization_percent: Option<u8>,
    pub logical_cpu_count: Option<u16>,
    pub memory_total_bytes: Option<u64>,
    pub memory_available_bytes: Option<u64>,
    pub disk_available_bytes: Option<u64>,
    pub disk_total_bytes: Option<u64>,
    pub load_average_milli: Option<[u32; 3]>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub path: PathBuf,
    pub priority: Option<i32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Recipe {
    pub name: String,
    pub version: Option<String>,
    pub layer: Option<String>,
    #[serde(default)]
    pub preferred_version: Option<String>,
    #[serde(default)]
    pub file: Option<PathBuf>,
    #[serde(default)]
    pub append_count: Option<usize>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeWorkspaceStatus {
    Clean,
    Modified,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecipeIdentity {
    pub name: String,
    pub file: PathBuf,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevtoolCapability {
    Available,
    MissingExecutable,
    Unavailable { reason: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevtoolWorkspace {
    NotMember,
    MissingDirectory {
        source_path: PathBuf,
    },
    Present {
        source_path: PathBuf,
        recipe_file: Option<PathBuf>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevtoolGitState {
    NotApplicable,
    MissingExecutable,
    NotRepository,
    Available {
        branch: Option<String>,
        head: Option<String>,
        modified: usize,
        untracked: usize,
        conflicted: usize,
    },
    Failed {
        exit_code: Option<i32>,
        message: String,
    },
    Malformed {
        message: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevtoolStatusError {
    InvalidRecipeIdentity,
    DevtoolFailed {
        exit_code: Option<i32>,
        message: String,
    },
    MalformedOutput {
        line: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolStatus {
    pub identity: RecipeIdentity,
    pub capability: DevtoolCapability,
    pub workspace: DevtoolWorkspace,
    pub git: DevtoolGitState,
    pub error: Option<DevtoolStatusError>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevtoolAction {
    ModifyOrEdit,
    UpdateRecipe,
    Finish,
    Deploy,
    Reset,
}
impl DevtoolStatus {
    pub fn disabled_reason(&self, action: DevtoolAction) -> Option<String> {
        match &self.capability {
            DevtoolCapability::Available => {}
            DevtoolCapability::MissingExecutable => {
                return Some("Devtool executable is missing.".into());
            }
            DevtoolCapability::Unavailable { reason } => return Some(reason.clone()),
        }
        if let Some(error) = &self.error {
            return Some(format!("Devtool status is unavailable: {error:?}."));
        }
        match (&self.workspace, action) {
            (DevtoolWorkspace::NotMember, DevtoolAction::ModifyOrEdit) => None,
            (DevtoolWorkspace::NotMember, _) => {
                Some("Recipe is not in the Devtool workspace.".into())
            }
            (DevtoolWorkspace::MissingDirectory { .. }, DevtoolAction::Reset) => None,
            (DevtoolWorkspace::MissingDirectory { .. }, _) => {
                Some("Devtool workspace source directory is missing.".into())
            }
            (DevtoolWorkspace::Present { .. }, DevtoolAction::Finish) => match &self.git {
                DevtoolGitState::Available {
                    head: Some(_),
                    modified: 0,
                    untracked: 0,
                    conflicted: 0,
                    ..
                } => None,
                DevtoolGitState::Available { .. } => {
                    Some("Commit all workspace changes before Devtool finish.".into())
                }
                _ => Some("Authoritative Git status is unavailable.".into()),
            },
            (DevtoolWorkspace::Present { .. }, _) => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeBuildStatus {
    Idle,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecipeMetadata {
    pub recipe: String,
    pub workspace_status: Option<RecipeWorkspaceStatus>,
    pub build_status: Option<RecipeBuildStatus>,
    pub tasks: Option<Vec<String>>,
    pub sources: Option<Vec<PathBuf>>,
    pub patches: Option<Vec<String>>,
    pub packages: Option<Vec<String>>,
    pub history: Option<Vec<String>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeEditor {
    pub recipe: String,
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
    pub selection: usize,
    pub content: String,
    pub editing: bool,
    pub dirty: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeTaskPicker {
    pub recipe: String,
    pub tasks: Vec<String>,
    pub selection: usize,
    pub force: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureTaskPicker {
    pub recipe: RecipeIdentity,
    pub tasks: Vec<String>,
    pub selection: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeTaskLogChoice {
    pub task: String,
    pub state: TaskState,
    pub path: PathBuf,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeTaskLogPicker {
    pub recipe: String,
    pub logs: Vec<RecipeTaskLogChoice>,
    pub selection: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipePatchPicker {
    pub recipe: String,
    pub patches: Vec<PathBuf>,
    pub selection: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigSourceChoice {
    pub operation: String,
    pub path: PathBuf,
    pub line: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSourcePicker {
    pub identity: VariableIdentity,
    pub sources: Vec<ConfigSourceChoice>,
    pub selection: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigScopePicker {
    pub variable: String,
    pub scopes: Vec<Option<String>>,
    pub selection: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigComparisonOutcome {
    Equal,
    Different,
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigComparisonField {
    pub global: Option<String>,
    pub recipe: Option<String>,
    pub outcome: ConfigComparisonOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigComparison {
    pub variable: String,
    pub recipe: String,
    pub effective: ConfigComparisonField,
    pub unexpanded: ConfigComparisonField,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEditRequest {
    pub identity: VariableIdentity,
    pub value: String,
    pub destination: PathBuf,
    pub assignment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupEditor {
    pub text: String,
    pub cursor: usize,
    pub selection: Option<(usize, usize)>,
    pub editing: bool,
    clipboard: String,
    history: Vec<String>,
}

impl PopupEditor {
    pub fn new(text: String) -> Self {
        let cursor = text.len();
        Self {
            text,
            cursor,
            selection: None,
            editing: false,
            clipboard: String::new(),
            history: Vec::new(),
        }
    }
    pub fn select_range(&mut self, start: usize, end: usize) {
        let start = self.clamp_boundary(start);
        let end = self.clamp_boundary(end);
        self.selection = Some((start.min(end), start.max(end)));
        self.cursor = end.min(self.text.len());
    }
    pub fn insert(&mut self, value: &str) {
        self.remember();
        if let Some((start, end)) = self.selection.take() {
            self.text.replace_range(start..end, value);
            self.cursor = start + value.len();
        } else {
            self.text.insert_str(self.cursor, value);
            self.cursor += value.len();
        }
    }
    pub fn backspace(&mut self) {
        if let Some((start, end)) = self.selection.take() {
            self.remember();
            self.text.replace_range(start..end, "");
            self.cursor = start;
        } else if self.cursor > 0 {
            self.remember();
            let previous = self.text[..self.cursor]
                .char_indices()
                .next_back()
                .map_or(0, |(index, _)| index);
            self.text.replace_range(previous..self.cursor, "");
            self.cursor = previous;
        }
    }
    pub fn left(&mut self) {
        self.cursor = self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(index, _)| index);
        self.selection = None;
    }
    pub fn right(&mut self) {
        self.cursor = self.text[self.cursor..]
            .chars()
            .next()
            .map_or(self.text.len(), |character| {
                self.cursor + character.len_utf8()
            });
        self.selection = None;
    }
    pub fn up(&mut self) {
        let line_start = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        if line_start == 0 {
            self.home();
            return;
        }
        let previous_end = line_start - 1;
        let previous_start = self.text[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        self.cursor = self.clamp_boundary(
            previous_start + (self.cursor - line_start).min(previous_end - previous_start),
        );
        self.selection = None;
    }
    pub fn down(&mut self) {
        let line_start = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let Some(relative_end) = self.text[self.cursor..].find('\n') else {
            self.end();
            return;
        };
        let next_start = self.cursor + relative_end + 1;
        let next_end = self.text[next_start..]
            .find('\n')
            .map_or(self.text.len(), |index| next_start + index);
        self.cursor =
            self.clamp_boundary(next_start + (self.cursor - line_start).min(next_end - next_start));
        self.selection = None;
    }
    pub fn home(&mut self) {
        self.cursor = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        self.selection = None;
    }
    pub fn end(&mut self) {
        self.cursor = self.text[self.cursor..]
            .find('\n')
            .map_or(self.text.len(), |index| self.cursor + index);
        self.selection = None;
    }
    pub fn selected_text(&self) -> Option<&str> {
        self.selection.map(|(start, end)| &self.text[start..end])
    }
    pub fn copy_selection_or_line(&mut self) -> String {
        let value = self.selected_text().map(str::to_owned).unwrap_or_else(|| {
            let start = self.text[..self.cursor]
                .rfind('\n')
                .map_or(0, |index| index + 1);
            let end = self.text[self.cursor..]
                .find('\n')
                .map_or(self.text.len(), |index| self.cursor + index);
            self.text[start..end].to_owned()
        });
        self.clipboard.clone_from(&value);
        value
    }
    pub fn paste(&mut self) {
        if !self.clipboard.is_empty() {
            let value = self.clipboard.clone();
            self.insert(&value);
        }
    }
    pub fn select_toml_value(&mut self, key: &str) -> Result<(), String> {
        let prefix = format!("{key} = ");
        let line_start = self
            .text
            .lines()
            .scan(0usize, |offset, line| {
                let start = *offset;
                *offset += line.len() + 1;
                Some((start, line))
            })
            .find_map(|(start, line)| line.trim_start().starts_with(&prefix).then_some(start))
            .ok_or_else(|| format!("Missing `{key}` TOML value."))?;
        let line_end = self.text[line_start..]
            .find('\n')
            .map_or(self.text.len(), |index| line_start + index);
        let line = &self.text[line_start..line_end];
        let (value_start, value_end) = popup_toml_value_range(line, line_start)?;
        self.select_range(value_start, value_end);
        Ok(())
    }
    pub fn select_toml_value_at_cursor(&mut self) -> Result<(), String> {
        let line_start = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let line_end = self.text[self.cursor..]
            .find('\n')
            .map_or(self.text.len(), |index| self.cursor + index);
        let line = &self.text[line_start..line_end];
        let (value_start, value_end) = popup_toml_value_range(line, line_start)?;
        self.select_range(value_start, value_end);
        Ok(())
    }
    pub fn undo(&mut self) -> bool {
        let Some(text) = self.history.pop() else {
            return false;
        };
        self.text = text;
        self.cursor = self.cursor.min(self.text.len());
        self.selection = None;
        true
    }
    fn remember(&mut self) {
        const MAX_HISTORY: usize = 32;
        if self.history.last() != Some(&self.text) {
            self.history.push(self.text.clone());
            if self.history.len() > MAX_HISTORY {
                self.history.remove(0);
            }
        }
    }
    fn clamp_boundary(&self, mut index: usize) -> usize {
        index = index.min(self.text.len());
        while !self.text.is_char_boundary(index) {
            index -= 1;
        }
        index
    }
}

fn popup_toml_value_range(line: &str, line_start: usize) -> Result<(usize, usize), String> {
    let equals = line
        .find('=')
        .ok_or_else(|| "The current TOML line has no value.".to_owned())?;
    let raw = &line[equals + 1..];
    let leading = raw.len() - raw.trim_start().len();
    let value = raw.trim_start();
    let value_start = line_start + equals + 1 + leading;
    if let Some(quoted) = value.strip_prefix('"') {
        let end = quoted
            .find('"')
            .ok_or_else(|| "The current TOML line has no closing quote.".to_owned())?;
        return Ok((value_start + 1, value_start + 1 + end));
    }
    let value = value.split_once('#').map_or(value, |(value, _)| value);
    let value = value.trim_end();
    if value.is_empty() {
        return Err("The current TOML line has no value.".into());
    }
    Ok((value_start, value_start + value.len()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupEditorCommand {
    ToggleInsert,
    Insert(char),
    Backspace,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    SelectValue,
    Copy,
    Paste,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    BuildEnvironmentCloneEditor(PopupEditor),
    BuildEnvironmentCloneReview(BuildEnvironmentClonePlan),
    BuildEnvironmentEditor(PopupEditor),
    ThemePicker {
        selection: usize,
        original_theme: Theme,
        original_color_enabled: bool,
        original_settings_dirty: bool,
    },
    BuildOptions,
    BuildCompletion,
    BuildTarget {
        editor: PopupEditor,
        task: Option<String>,
    },
    ImagePicker(ImagePicker),
    QemuLaunch(QemuLaunchDialog),
    QemuLaunchConfirmation(QemuLaunchPreview),
    QemuCancellationConfirmation(QemuSessionId),
    WicCreate(WicCreateDialog),
    WicCreateTomlEditor {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    WicCreateConfirmation(WicCreatePreview),
    WicDevicePicker(WicDevicePickerDialog),
    WicWritePhrase(WicWritePhraseDialog),
    WicWriteConfirmation(WicWritePreview),
    WicCancellationConfirmation {
        id: WicSessionId,
        incomplete_device_warning: bool,
    },
    SdkBuildConfirmation(SdkBuildPreview),
    SdkPublish(SdkPublishDraft),
    SdkPublishTomlEditor(PopupEditor),
    SdkPublishConfirmation(SdkPublishPreview),
    SdkNative(SdkNativeDialog),
    SdkNativeTomlEditor(PopupEditor),
    SdkNativeConfirmation(SdkNativePreview),
    SdkCancellationConfirmation(SdkSessionId),
    TestLaunch(TestLaunchDialog),
    TestLaunchTomlEditor {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    TestLaunchConfirmation(TestLaunchPreview),
    TestCancellationConfirmation(TestSessionId),
    TestResultImport(TestResultImportDialog),
    TestResultImportTomlEditor {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    TestComparison(TestComparisonPicker),
    TestComparisonTomlEditor {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    TestComparisonConfirmation(TestComparisonPreview),
    TestJunitExport(TestJunitExportDialog),
    TestJunitTomlEditor {
        result: TestResultIdentity,
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    TestJunitExportConfirmation(TestJunitExportPreview),
    Security(SecurityDialog),
    Qa(QaDialog),
    Maintenance(Box<MaintenanceDialog>),
    RecipeTaskConfirmation(BuildRequest),
    RecipeTaskPicker(RecipeTaskPicker),
    SignatureTaskPicker(SignatureTaskPicker),
    RecipeTaskLogPicker(RecipeTaskLogPicker),
    RecipePatchPicker(RecipePatchPicker),
    ConfigSourcePicker(ConfigSourcePicker),
    ConfigScopePicker(ConfigScopePicker),
    ConfigComparison(ConfigComparison),
    ConfigEdit {
        identity: VariableIdentity,
        editor: PopupEditor,
    },
    ConfigEditConfirmation(ConfigEditRequest),
    DevtoolModifyConfirmation(RecipeIdentity),
    DevtoolResetConfirmation(DevtoolResetPlan),
    DevtoolUpdateConfirmation(RecipeIdentity),
    DevtoolFinishPicker(DevtoolFinishPicker),
    DevtoolFinishConfirmation(DevtoolFinishPlan),
    DevtoolDeploy(DevtoolDeployDraft),
    DevtoolDeployConfirmation(DevtoolDeployPlan),
    BbmaskEdit(PopupEditor),
    BbmaskConfirmation(String),
    RecipeEditor(RecipeEditor),
    QuitConfirmation,
}
impl RecipeEditor {
    fn selected_path(&self) -> Option<PathBuf> {
        self.files
            .get(self.selection)
            .map(|path| self.root.join(path))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildState {
    pub status: BuildStatus,
    pub target: Option<String>,
    pub started: Option<SystemTime>,
    pub completed: usize,
    pub total: Option<usize>,
    pub parse_current: Option<u64>,
    pub parse_total: Option<u64>,
    pub warnings: usize,
    pub errors: usize,
    pub exit_code: Option<i32>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRecord {
    pub target: Option<String>,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub elapsed: Option<Duration>,
    pub completed_tasks: usize,
    pub warnings: usize,
    pub errors: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BackgroundJobId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundJobKind {
    Build,
    CveCheck,
    Spdx,
    Qemu,
    Wic,
    Sdk,
    Test,
    Devtool,
    Maintenance,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundJobStatus {
    Queued,
    Starting,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    Lost,
}
impl BackgroundJobStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Lost
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundJobProgress {
    Indeterminate,
    Percent(u8),
    Units { completed: u64, total: u64 },
}
impl BackgroundJobProgress {
    fn is_valid(&self) -> bool {
        match self {
            Self::Indeterminate => true,
            Self::Percent(percent) => *percent <= 100,
            Self::Units { completed, total } => *total > 0 && completed <= total,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackgroundJobContext {
    pub workspace: Option<Screen>,
    pub target: Option<String>,
    pub recipe: Option<String>,
    pub task: Option<String>,
    pub image: Option<String>,
    pub path: Option<PathBuf>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundJobSpec {
    pub id: BackgroundJobId,
    pub kind: BackgroundJobKind,
    pub title: String,
    pub context: BackgroundJobContext,
    pub cancellation_supported: bool,
    pub queued_at: SystemTime,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundJobOutputSource {
    Backend,
    Stdout,
    Stderr,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundJobOutputEntry {
    pub severity: Severity,
    pub message: String,
    pub source: BackgroundJobOutputSource,
    pub truncated: bool,
    pub timestamp: SystemTime,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundJobResult {
    pub summary: String,
    pub artifacts: Vec<PathBuf>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundJobError {
    pub summary: String,
    pub detail: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundJob {
    pub id: BackgroundJobId,
    pub kind: BackgroundJobKind,
    pub title: String,
    pub status: BackgroundJobStatus,
    pub context: BackgroundJobContext,
    pub cancellation_supported: bool,
    pub progress: BackgroundJobProgress,
    pub output: VecDeque<BackgroundJobOutputEntry>,
    pub retained_output_bytes: usize,
    pub dropped_output_entries: usize,
    pub warnings: usize,
    pub errors: usize,
    pub queued_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
    pub result: Option<BackgroundJobResult>,
    pub error: Option<BackgroundJobError>,
}
impl BackgroundJob {
    fn from_spec(spec: BackgroundJobSpec) -> Self {
        Self {
            id: spec.id,
            kind: spec.kind,
            title: spec.title,
            status: BackgroundJobStatus::Queued,
            context: spec.context,
            cancellation_supported: spec.cancellation_supported,
            progress: BackgroundJobProgress::Indeterminate,
            output: VecDeque::new(),
            retained_output_bytes: 0,
            dropped_output_entries: 0,
            warnings: 0,
            errors: 0,
            queued_at: spec.queued_at,
            started_at: None,
            finished_at: None,
            result: None,
            error: None,
        }
    }
}
const MAX_BACKGROUND_JOBS: usize = 128;
const MAX_BACKGROUND_JOB_OUTPUT_ENTRIES: usize = 512;
const MAX_BACKGROUND_JOB_OUTPUT_BYTES: usize = 1024 * 1024;
pub const MAX_SIGNATURE_RECORDS: usize = 256;
pub const MAX_SIGNATURE_DIFFERENCES: usize = 4_096;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundJobs {
    pub jobs: VecDeque<BackgroundJob>,
    pub dropped_jobs: usize,
    pub rejected_jobs: usize,
    pub ignored_transitions: usize,
    max_jobs: usize,
    max_output_entries: usize,
    max_output_bytes: usize,
}
impl BackgroundJobs {
    pub fn new(max_jobs: usize, max_output_entries: usize, max_output_bytes: usize) -> Self {
        Self {
            jobs: VecDeque::new(),
            dropped_jobs: 0,
            rejected_jobs: 0,
            ignored_transitions: 0,
            max_jobs: max_jobs.max(1),
            max_output_entries: max_output_entries.max(1),
            max_output_bytes: max_output_bytes.max(1),
        }
    }

    pub fn get(&self, id: BackgroundJobId) -> Option<&BackgroundJob> {
        self.jobs.iter().find(|job| job.id == id)
    }

    fn queue(&mut self, spec: BackgroundJobSpec) {
        if spec.title.trim().is_empty() || self.get(spec.id).is_some() {
            self.rejected_jobs += 1;
            return;
        }
        while self.jobs.len() >= self.max_jobs {
            let Some(index) = self.jobs.iter().position(|job| job.status.is_terminal()) else {
                self.rejected_jobs += 1;
                return;
            };
            self.jobs.remove(index);
            self.dropped_jobs += 1;
        }
        self.jobs.push_back(BackgroundJob::from_spec(spec));
    }

    fn update_if(
        &mut self,
        id: BackgroundJobId,
        allowed: &[BackgroundJobStatus],
        mutation: impl FnOnce(&mut BackgroundJob),
    ) {
        let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) else {
            self.ignored_transitions += 1;
            return;
        };
        if !allowed.contains(&job.status) {
            self.ignored_transitions += 1;
            return;
        }
        mutation(job);
    }

    fn append_output(&mut self, id: BackgroundJobId, entry: BackgroundJobOutputEntry) {
        let max_entries = self.max_output_entries;
        let max_bytes = self.max_output_bytes;
        self.update_if(
            id,
            &[
                BackgroundJobStatus::Queued,
                BackgroundJobStatus::Starting,
                BackgroundJobStatus::Running,
                BackgroundJobStatus::Cancelling,
            ],
            |job| {
                match entry.severity {
                    Severity::Warning => job.warnings += 1,
                    Severity::Error => job.errors += 1,
                    Severity::Trace | Severity::Info => {}
                }
                job.retained_output_bytes += entry.message.len();
                job.output.push_back(entry);
                while job.output.len() > max_entries || job.retained_output_bytes > max_bytes {
                    let Some(dropped) = job.output.pop_front() else {
                        break;
                    };
                    job.retained_output_bytes = job
                        .retained_output_bytes
                        .saturating_sub(dropped.message.len());
                    job.dropped_output_entries += 1;
                }
            },
        );
    }

    fn request_cancellation(&mut self, id: BackgroundJobId) {
        let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) else {
            self.ignored_transitions += 1;
            return;
        };
        if !job.cancellation_supported
            || !matches!(
                job.status,
                BackgroundJobStatus::Queued
                    | BackgroundJobStatus::Starting
                    | BackgroundJobStatus::Running
            )
        {
            self.ignored_transitions += 1;
            return;
        }
        job.status = BackgroundJobStatus::Cancelling;
    }
}
impl Default for BackgroundJobs {
    fn default() -> Self {
        Self::new(
            MAX_BACKGROUND_JOBS,
            MAX_BACKGROUND_JOB_OUTPUT_ENTRIES,
            MAX_BACKGROUND_JOB_OUTPUT_BYTES,
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecipeDependencies {
    pub recipe: String,
    pub build: Vec<String>,
    pub runtime: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyNodeId {
    Recipe(String),
    Task { recipe: String, task: String },
}
impl DependencyNodeId {
    pub fn recipe(name: impl Into<String>) -> Self {
        Self::Recipe(name.into())
    }

    pub fn task(recipe: impl Into<String>, task: impl Into<String>) -> Self {
        Self::Task {
            recipe: recipe.into(),
            task: task.into(),
        }
    }

    pub fn recipe_name(&self) -> &str {
        match self {
            Self::Recipe(recipe) | Self::Task { recipe, .. } => recipe,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyNode {
    pub id: DependencyNodeId,
    pub provider: Option<PathBuf>,
    pub log: Option<PathBuf>,
}
impl DependencyNode {
    pub fn identity(id: DependencyNodeId) -> Self {
        Self {
            id,
            provider: None,
            log: None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyEdgeKind {
    Build,
    Runtime,
    Task,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DependencyEdge {
    pub from: DependencyNodeId,
    pub to: DependencyNodeId,
    pub kind: DependencyEdgeKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DependencyNormalizationReport {
    pub duplicate_nodes: usize,
    pub duplicate_edges: usize,
    pub self_edges: usize,
    pub synthesized_nodes: usize,
    pub truncated_nodes: usize,
    pub truncated_edges: usize,
}
impl DependencyNormalizationReport {
    pub fn is_partial(&self) -> bool {
        self.truncated_nodes > 0 || self.truncated_edges > 0
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyGraph {
    pub root: DependencyNodeId,
    pub nodes: Vec<DependencyNode>,
    pub edges: Vec<DependencyEdge>,
}
impl DependencyGraph {
    pub fn normalize(
        root: DependencyNodeId,
        mut nodes: Vec<DependencyNode>,
        mut edges: Vec<DependencyEdge>,
        max_nodes: usize,
        max_edges: usize,
    ) -> (Self, DependencyNormalizationReport) {
        let mut report = DependencyNormalizationReport::default();
        nodes.sort();

        let mut normalized_nodes = BTreeMap::new();
        for node in nodes {
            if normalized_nodes.contains_key(&node.id) {
                report.duplicate_nodes += 1;
                continue;
            }
            normalized_nodes.insert(node.id.clone(), node);
        }
        normalized_nodes
            .entry(root.clone())
            .or_insert_with(|| DependencyNode::identity(root.clone()));

        let node_limit = max_nodes.max(1);
        if normalized_nodes.len() > node_limit {
            let removable = normalized_nodes
                .keys()
                .filter(|id| **id != root)
                .skip(node_limit.saturating_sub(1))
                .cloned()
                .collect::<Vec<_>>();
            report.truncated_nodes += removable.len();
            for id in removable {
                normalized_nodes.remove(&id);
            }
        }

        edges.sort();
        let mut unique_edges = BTreeSet::new();
        for edge in edges {
            if edge.from == edge.to {
                report.self_edges += 1;
                continue;
            }
            if !unique_edges.insert(edge.clone()) {
                report.duplicate_edges += 1;
            }
        }

        let mut normalized_edges = Vec::new();
        for edge in unique_edges {
            let missing = [&edge.from, &edge.to]
                .into_iter()
                .filter(|id| !normalized_nodes.contains_key(*id))
                .cloned()
                .collect::<BTreeSet<_>>();
            if normalized_nodes.len() + missing.len() > node_limit {
                report.truncated_edges += 1;
                continue;
            }
            for id in missing {
                normalized_nodes.insert(id.clone(), DependencyNode::identity(id));
                report.synthesized_nodes += 1;
            }
            if normalized_edges.len() == max_edges {
                report.truncated_edges += 1;
                continue;
            }
            normalized_edges.push(edge);
        }

        (
            Self {
                root,
                nodes: normalized_nodes.into_values().collect(),
                edges: normalized_edges,
            },
            report,
        )
    }

    pub fn contains(&self, id: &DependencyNodeId) -> bool {
        self.nodes.iter().any(|node| &node.id == id)
    }

    pub fn incoming(&self, id: &DependencyNodeId) -> Vec<&DependencyEdge> {
        self.edges.iter().filter(|edge| &edge.to == id).collect()
    }

    pub fn outgoing(&self, id: &DependencyNodeId) -> Vec<&DependencyEdge> {
        self.edges.iter().filter(|edge| &edge.from == id).collect()
    }

    pub fn why_built(
        &self,
        target: &DependencyNodeId,
        max_depth: usize,
        max_visited: usize,
    ) -> DependencyPathResult {
        if !self.contains(target) {
            return DependencyPathResult::Unreachable;
        }
        if self.root == *target {
            return DependencyPathResult::Found(vec![self.root.clone()]);
        }
        if max_visited == 0 {
            return DependencyPathResult::LimitReached;
        }

        let mut queue = VecDeque::from([(self.root.clone(), 0_usize)]);
        let mut visited = BTreeSet::from([self.root.clone()]);
        let mut parents = BTreeMap::new();
        let mut limited = false;
        while let Some((current, depth)) = queue.pop_front() {
            if depth == max_depth {
                limited |= !self.outgoing(&current).is_empty();
                continue;
            }
            for edge in self.outgoing(&current) {
                if visited.contains(&edge.to) {
                    continue;
                }
                if visited.len() == max_visited {
                    limited = true;
                    break;
                }
                visited.insert(edge.to.clone());
                parents.insert(edge.to.clone(), current.clone());
                if edge.to == *target {
                    let mut path = vec![target.clone()];
                    let mut cursor = target;
                    while let Some(parent) = parents.get(cursor) {
                        path.push(parent.clone());
                        cursor = parent;
                    }
                    path.reverse();
                    return DependencyPathResult::Found(path);
                }
                queue.push_back((edge.to.clone(), depth + 1));
            }
        }
        if limited {
            DependencyPathResult::LimitReached
        } else {
            DependencyPathResult::Unreachable
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyPathResult {
    Found(Vec<DependencyNodeId>),
    Unreachable,
    LimitReached,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DependencyGraphState {
    #[default]
    NotLoaded,
    Loading {
        root: DependencyNodeId,
    },
    AvailableEmpty {
        root: DependencyNodeId,
    },
    Available(DependencyGraph),
    Partial {
        graph: DependencyGraph,
        limitations: Vec<String>,
    },
    Failed {
        root: DependencyNodeId,
        message: String,
    },
}
impl DependencyGraphState {
    pub fn graph(&self) -> Option<&DependencyGraph> {
        match self {
            Self::Available(graph) | Self::Partial { graph, .. } => Some(graph),
            Self::NotLoaded
            | Self::Loading { .. }
            | Self::AvailableEmpty { .. }
            | Self::Failed { .. } => None,
        }
    }

    pub fn root(&self) -> Option<&DependencyNodeId> {
        match self {
            Self::NotLoaded => None,
            Self::Loading { root } | Self::AvailableEmpty { root } | Self::Failed { root, .. } => {
                Some(root)
            }
            Self::Available(graph) | Self::Partial { graph, .. } => Some(&graph.root),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignatureTarget {
    pub recipe: String,
    pub task: String,
}
impl SignatureTarget {
    pub fn validate(&self) -> Result<(), &'static str> {
        if signature_component_is_valid(&self.recipe) && signature_component_is_valid(&self.task) {
            Ok(())
        } else {
            Err("signature recipe and task must be non-empty tokens without whitespace or controls")
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignatureIdentity {
    pub target: SignatureTarget,
    pub hash: Option<String>,
    pub path: Option<PathBuf>,
}
impl SignatureIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.target.validate()?;
        if self.hash.as_ref().is_some_and(|hash| {
            hash.is_empty()
                || hash.len() > 256
                || hash.chars().any(char::is_whitespace)
                || hash.chars().any(char::is_control)
        }) {
            return Err("signature hashes must be bounded tokens");
        }
        if self.path.as_ref().is_some_and(|path| !path.is_absolute()) {
            return Err("signature paths must be absolute");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SignatureValue {
    pub name: String,
    pub value: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SignatureRecord {
    pub identity: SignatureIdentity,
    pub base_hash: Option<String>,
    pub task_hash: Option<String>,
    pub variables: Vec<SignatureValue>,
    pub dependencies: Vec<String>,
}
impl SignatureRecord {
    fn normalize(mut self) -> Self {
        self.variables.sort();
        self.variables
            .dedup_by(|left, right| left.name == right.name);
        self.dependencies.sort();
        self.dependencies.dedup();
        self
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SignatureNormalizationReport {
    pub duplicate_records: usize,
    pub invalid_records: usize,
    pub truncated_records: usize,
}
impl SignatureNormalizationReport {
    pub fn is_partial(&self) -> bool {
        self.invalid_records > 0 || self.truncated_records > 0
    }
}
pub fn normalize_signature_records(
    target: &SignatureTarget,
    records: Vec<SignatureRecord>,
    max_records: usize,
) -> (Vec<SignatureRecord>, SignatureNormalizationReport) {
    let mut report = SignatureNormalizationReport::default();
    let mut normalized = BTreeMap::new();
    for record in records {
        if record.identity.target != *target || record.identity.validate().is_err() {
            report.invalid_records += 1;
            continue;
        }
        let record = record.normalize();
        if let Some(existing) = normalized.get(&record.identity) {
            report.duplicate_records += 1;
            if &record < existing {
                normalized.insert(record.identity.clone(), record);
            }
        } else {
            normalized.insert(record.identity.clone(), record);
        }
    }
    let mut records = normalized.into_values().collect::<Vec<_>>();
    if records.len() > max_records {
        report.truncated_records = records.len() - max_records;
        records.truncate(max_records);
    }
    (records, report)
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SignatureDumpState {
    #[default]
    NotLoaded,
    Loading {
        target: SignatureTarget,
    },
    AvailableEmpty {
        target: SignatureTarget,
    },
    Available {
        target: SignatureTarget,
        records: Vec<SignatureRecord>,
    },
    Partial {
        target: SignatureTarget,
        records: Vec<SignatureRecord>,
        limitations: Vec<String>,
    },
    Failed {
        target: SignatureTarget,
        message: String,
    },
}
impl SignatureDumpState {
    pub fn records(&self) -> Option<&[SignatureRecord]> {
        match self {
            Self::Available { records, .. } | Self::Partial { records, .. } => Some(records),
            Self::NotLoaded
            | Self::Loading { .. }
            | Self::AvailableEmpty { .. }
            | Self::Failed { .. } => None,
        }
    }

    pub fn target(&self) -> Option<&SignatureTarget> {
        match self {
            Self::NotLoaded => None,
            Self::Loading { target }
            | Self::AvailableEmpty { target }
            | Self::Available { target, .. }
            | Self::Partial { target, .. }
            | Self::Failed { target, .. } => Some(target),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureComparisonSide {
    Left,
    Right,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignatureComparisonRequest {
    pub left: SignatureIdentity,
    pub right: SignatureIdentity,
}
impl SignatureComparisonRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.left.validate()?;
        self.right.validate()?;
        if self.left == self.right {
            return Err("signature comparison requires two distinct identities");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SignatureDifferenceCategory {
    BaseHash,
    ChangedValue,
    Dependency,
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignatureDifference {
    pub category: SignatureDifferenceCategory,
    pub key: String,
    pub left: Option<String>,
    pub right: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SignatureDifferenceReport {
    pub duplicate_differences: usize,
    pub truncated_differences: usize,
}
impl SignatureDifferenceReport {
    pub fn is_partial(&self) -> bool {
        self.truncated_differences > 0
    }
}
pub fn normalize_signature_differences(
    mut differences: Vec<SignatureDifference>,
    max_differences: usize,
) -> (Vec<SignatureDifference>, SignatureDifferenceReport) {
    differences.sort();
    let before = differences.len();
    differences.dedup();
    let mut report = SignatureDifferenceReport {
        duplicate_differences: before - differences.len(),
        ..SignatureDifferenceReport::default()
    };
    if differences.len() > max_differences {
        report.truncated_differences = differences.len() - max_differences;
        differences.truncate(max_differences);
    }
    (differences, report)
}
pub fn compare_signature_records(
    left: &SignatureRecord,
    right: &SignatureRecord,
    max_differences: usize,
) -> (Vec<SignatureDifference>, SignatureDifferenceReport) {
    let mut differences = Vec::new();
    signature_hash_difference(
        &mut differences,
        "base_hash",
        left.base_hash.as_ref(),
        right.base_hash.as_ref(),
    );
    signature_hash_difference(
        &mut differences,
        "task_hash",
        left.task_hash.as_ref(),
        right.task_hash.as_ref(),
    );

    let left_values = left
        .variables
        .iter()
        .map(|value| (value.name.as_str(), value))
        .collect::<BTreeMap<_, _>>();
    let right_values = right
        .variables
        .iter()
        .map(|value| (value.name.as_str(), value))
        .collect::<BTreeMap<_, _>>();
    for name in left_values
        .keys()
        .chain(right_values.keys())
        .copied()
        .collect::<BTreeSet<_>>()
    {
        let left = left_values.get(name).copied();
        let right = right_values.get(name).copied();
        if left.map(|value| &value.value) != right.map(|value| &value.value) {
            differences.push(SignatureDifference {
                category: if matches!((left, right), (Some(left), Some(right))
                    if left.value.is_some() && right.value.is_some())
                {
                    SignatureDifferenceCategory::ChangedValue
                } else {
                    SignatureDifferenceCategory::Unavailable
                },
                key: name.to_owned(),
                left: left.and_then(|value| value.value.clone()),
                right: right.and_then(|value| value.value.clone()),
            });
        }
    }

    let left_dependencies = left
        .dependencies
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let right_dependencies = right
        .dependencies
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for dependency in left_dependencies.symmetric_difference(&right_dependencies) {
        differences.push(SignatureDifference {
            category: SignatureDifferenceCategory::Dependency,
            key: (*dependency).to_owned(),
            left: left_dependencies
                .contains(dependency)
                .then(|| "present".into()),
            right: right_dependencies
                .contains(dependency)
                .then(|| "present".into()),
        });
    }
    normalize_signature_differences(differences, max_differences)
}
fn signature_hash_difference(
    differences: &mut Vec<SignatureDifference>,
    key: &str,
    left: Option<&String>,
    right: Option<&String>,
) {
    if left != right {
        differences.push(SignatureDifference {
            category: if left.is_some() && right.is_some() {
                SignatureDifferenceCategory::BaseHash
            } else {
                SignatureDifferenceCategory::Unavailable
            },
            key: key.into(),
            left: left.cloned(),
            right: right.cloned(),
        });
    }
}
fn signature_component_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !value.chars().any(char::is_whitespace)
        && !value.chars().any(char::is_control)
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SignatureComparisonState {
    #[default]
    NotSelected,
    Ready {
        left: Option<SignatureIdentity>,
        right: Option<SignatureIdentity>,
    },
    Loading {
        request: SignatureComparisonRequest,
    },
    AvailableEmpty {
        request: SignatureComparisonRequest,
    },
    Available {
        request: SignatureComparisonRequest,
        differences: Vec<SignatureDifference>,
    },
    Partial {
        request: SignatureComparisonRequest,
        differences: Vec<SignatureDifference>,
        limitations: Vec<String>,
    },
    Failed {
        request: SignatureComparisonRequest,
        message: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerRelationships {
    pub layers: Vec<LayerRelationship>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerRelationship {
    pub name: String,
    pub priority: Option<i32>,
    pub compatible: Vec<String>,
    pub depends: Vec<String>,
    pub overlays: Vec<String>,
    pub appends: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitFileState {
    Clean,
    Modified,
    Untracked,
    Ignored,
    #[default]
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreviewKind {
    Text,
    Binary,
    #[default]
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayerInspectorMode {
    #[default]
    Preview,
    Git,
    Metadata,
    Dependencies,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerBrowserEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub depth: usize,
    pub is_hidden: bool,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    pub git: GitFileState,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerBrowser {
    pub layer: String,
    pub root: PathBuf,
    pub directory: PathBuf,
    pub entries: Vec<LayerBrowserEntry>,
    pub nodes: HashMap<PathBuf, Vec<LayerBrowserEntry>>,
    pub expanded: HashSet<PathBuf>,
    pub show_hidden: bool,
    pub selection: usize,
    pub preview: String,
    pub preview_kind: PreviewKind,
    pub preview_truncated: bool,
    pub inspector_mode: LayerInspectorMode,
}
impl LayerBrowser {
    pub fn new(layer: String, root: PathBuf) -> Self {
        let mut expanded = HashSet::new();
        expanded.insert(root.clone());
        Self {
            layer,
            directory: root.clone(),
            root,
            entries: Vec::new(),
            nodes: HashMap::new(),
            expanded,
            show_hidden: false,
            selection: 0,
            preview: String::new(),
            preview_kind: PreviewKind::Unavailable,
            preview_truncated: false,
            inspector_mode: LayerInspectorMode::Preview,
        }
    }
    pub fn selected_entry(&self) -> Option<&LayerBrowserEntry> {
        self.entries.get(self.selection)
    }
    fn rebuild(&mut self, preferred: Option<&PathBuf>) {
        fn collect(
            directory: &PathBuf,
            depth: usize,
            nodes: &HashMap<PathBuf, Vec<LayerBrowserEntry>>,
            expanded: &HashSet<PathBuf>,
            show_hidden: bool,
            output: &mut Vec<LayerBrowserEntry>,
        ) {
            let Some(children) = nodes.get(directory) else {
                return;
            };
            for child in children {
                if child.is_hidden && !show_hidden {
                    continue;
                }
                let mut visible = child.clone();
                visible.depth = depth;
                output.push(visible);
                if child.is_dir && expanded.contains(&child.path) {
                    collect(&child.path, depth + 1, nodes, expanded, show_hidden, output);
                }
            }
        }
        let mut entries = Vec::new();
        collect(
            &self.root,
            0,
            &self.nodes,
            &self.expanded,
            self.show_hidden,
            &mut entries,
        );
        self.entries = entries;
        self.selection = preferred
            .and_then(|path| self.entries.iter().position(|entry| &entry.path == path))
            .unwrap_or_else(|| self.selection.min(self.entries.len().saturating_sub(1)));
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePicker {
    pub images: Vec<String>,
    pub selection: usize,
}
const MAX_BUILD_HISTORY: usize = 50;
impl Default for BuildState {
    fn default() -> Self {
        Self {
            status: BuildStatus::Idle,
            target: None,
            started: None,
            completed: 0,
            total: None,
            parse_current: None,
            parse_total: None,
            warnings: 0,
            errors: 0,
            exit_code: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogState {
    pub entries: VecDeque<LogEntry>,
    pub max_entries: usize,
    pub max_bytes: usize,
    pub retained_bytes: usize,
    pub dropped: usize,
    pub dropped_warnings: usize,
    pub dropped_errors: usize,
    pub coalesced: usize,
    pub follow: bool,
    pub paused_len: Option<usize>,
    pub wrap: bool,
    pub filter: Option<Severity>,
    pub recipe_filter: Option<String>,
    pub task_filter: Option<String>,
    pub build_filter: Option<String>,
    pub query: String,
    pub searching: bool,
    pub scroll_offset: usize,
    pub horizontal_offset: usize,
    pub selection: usize,
    pub jump_target: Option<u64>,
    next_id: u64,
}
impl LogState {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
            max_bytes,
            retained_bytes: 0,
            dropped: 0,
            dropped_warnings: 0,
            dropped_errors: 0,
            coalesced: 0,
            follow: true,
            paused_len: None,
            wrap: false,
            filter: None,
            recipe_filter: None,
            task_filter: None,
            build_filter: None,
            query: String::new(),
            searching: false,
            scroll_offset: 0,
            horizontal_offset: 0,
            selection: 0,
            jump_target: None,
            next_id: 1,
        }
    }
    pub fn insert(&mut self, mut entry: LogEntry) {
        if entry.diagnostic.is_none()
            && matches!(entry.severity, Severity::Warning | Severity::Error)
        {
            entry.diagnostic = Some(diagnostic_for_entry(&entry));
        }
        if self.max_entries == 0 || self.max_bytes == 0 {
            self.record_drop(&entry);
            return;
        }
        if self.paused_len.is_none()
            && !self.is_important(&entry)
            && self.entries.back().is_some_and(|last| {
                last.severity == entry.severity
                    && last.message == entry.message
                    && last.recipe == entry.recipe
                    && last.task == entry.task
                    && last.path == entry.path
                    && last.build == entry.build
            })
        {
            self.coalesced += 1;
            if let Some(last) = self.entries.back_mut() {
                last.timestamp = entry.timestamp;
            }
            return;
        }
        if entry.message.len() > self.max_bytes {
            let suffix = "\n[entry truncated to retention byte limit]";
            let mut keep = self
                .max_bytes
                .saturating_sub(suffix.len())
                .min(entry.message.len());
            while keep > 0 && !entry.message.is_char_boundary(keep) {
                keep -= 1;
            }
            entry.message.truncate(keep);
            if suffix.len() <= self.max_bytes {
                entry.message.push_str(suffix);
            }
        }
        if entry.id == 0 {
            entry.id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1).max(1);
        }
        let bytes = entry.message.len();
        self.retained_bytes += bytes;
        self.entries.push_back(entry);
        while self.entries.len() > self.max_entries || self.retained_bytes > self.max_bytes {
            let ordinary = self
                .entries
                .iter()
                .position(|candidate| !self.is_important(candidate));
            let index = ordinary.unwrap_or(0);
            let Some(old) = self.entries.remove(index) else {
                break;
            };
            if self.paused_len.is_some_and(|visible| index < visible) {
                self.paused_len = self.paused_len.map(|visible| visible.saturating_sub(1));
            }
            self.retained_bytes = self.retained_bytes.saturating_sub(old.message.len());
            self.record_drop(&old);
        }
        self.clamp_selection();
        if self.follow {
            self.selection = self.filtered().count().saturating_sub(1);
            self.scroll_offset = 0;
        }
    }
    pub fn filtered(&self) -> impl Iterator<Item = &LogEntry> {
        let query = self.query.to_lowercase();
        let visible_len = self.paused_len.unwrap_or(self.entries.len());
        self.entries.iter().take(visible_len).filter(move |e| {
            self.jump_target == Some(e.id)
                || (self.filter.is_none_or(|s| s == e.severity)
                    && self
                        .recipe_filter
                        .as_ref()
                        .is_none_or(|recipe| e.recipe.as_ref() == Some(recipe))
                    && self
                        .task_filter
                        .as_ref()
                        .is_none_or(|task| e.task.as_ref() == Some(task))
                    && self
                        .build_filter
                        .as_ref()
                        .is_none_or(|build| e.build.as_ref() == Some(build))
                    && (query.is_empty() || e.message.to_lowercase().contains(&query)))
        })
    }
    pub fn diagnostics(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.diagnostic.is_some())
    }
    pub fn selected(&self) -> Option<&LogEntry> {
        self.filtered().nth(self.selection)
    }
    pub fn match_position(&self) -> Option<(usize, usize)> {
        let count = self.filtered().count();
        (count > 0).then(|| (self.selection.min(count - 1) + 1, count))
    }
    fn is_important(&self, entry: &LogEntry) -> bool {
        entry.protected || matches!(entry.severity, Severity::Warning | Severity::Error)
    }
    fn record_drop(&mut self, entry: &LogEntry) {
        self.dropped += 1;
        match entry.severity {
            Severity::Warning => self.dropped_warnings += 1,
            Severity::Error => self.dropped_errors += 1,
            Severity::Trace | Severity::Info => {}
        }
    }
    fn clamp_selection(&mut self) {
        self.selection = self
            .selection
            .min(self.filtered().count().saturating_sub(1));
        self.scroll_offset = self
            .filtered()
            .count()
            .saturating_sub(self.selection.saturating_add(1));
    }
}
fn diagnostic_for_entry(entry: &LogEntry) -> DiagnosticInfo {
    let summary = entry
        .message
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Diagnostic without a message")
        .trim()
        .chars()
        .take(120)
        .collect();
    let mut event_metadata = vec![
        ("severity".into(), format!("{:?}", entry.severity)),
        ("protected".into(), entry.protected.to_string()),
    ];
    if let Some(build) = entry.build.as_ref() {
        event_metadata.push(("build".into(), build.clone()));
    }
    if let Some(path) = entry.path.as_ref() {
        event_metadata.push(("source".into(), path.display().to_string()));
    }
    let mut suggestions = vec!["Inspect the matching retained log context.".into()];
    if entry.path.is_some() {
        suggestions.push("Open the source log and inspect surrounding output.".into());
    }
    if entry.recipe.is_some() {
        suggestions.push("Inspect the recipe task and its metadata.".into());
    }
    DiagnosticInfo {
        category: match entry.severity {
            Severity::Warning => "BitBake warning",
            Severity::Error => "BitBake error",
            Severity::Trace | Severity::Info => "Build diagnostic",
        }
        .into(),
        summary,
        event_metadata,
        suggestions,
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub daemon: ClientDaemonView,
    pub pty_selection: usize,
    pub pane_layout: PaneLayout,
    pub screen: Screen,
    pub focus: FocusTarget,
    pub focus_return: Option<FocusTarget>,
    pub navigator_selection: usize,
    pub backend: String,
    pub project_profile: ProjectProfileState,
    pub project_profile_selection: usize,
    pub build_environment: BuildEnvironmentState,
    pub build_environment_generation: u64,
    pub build_environment_draft: Option<BuildEnvironmentDraft>,
    pub available_images: Vec<String>,
    pub color_enabled: bool,
    pub color_forced_off: bool,
    pub theme: Theme,
    pub animation_speed: AnimationSpeed,
    pub reduced_motion: bool,
    pub settings_selection: usize,
    pub settings_dirty: bool,
    pub animation_frame: u64,
    pub workspace: Workspace,
    pub host_telemetry: HostTelemetry,
    pub host_cpu_history: VecDeque<u8>,
    pub host_memory_history: VecDeque<u8>,
    pub build: BuildState,
    pub background_jobs: BackgroundJobs,
    pub build_history: VecDeque<BuildRecord>,
    pub build_history_selection: usize,
    pub dependencies: Option<RecipeDependencies>,
    pub dependency_selection: usize,
    pub dependency_graph: DependencyGraphState,
    pub dependency_graph_selection: Option<DependencyNodeId>,
    pub signature_dump: SignatureDumpState,
    pub signature_selection: Option<SignatureIdentity>,
    pub signature_comparison: SignatureComparisonState,
    pub signature_recipe: Option<RecipeIdentity>,
    pub package_inventory: PackageInventoryState,
    pub package_selection: Option<PackageIdentity>,
    pub package_details: HashMap<PackageIdentity, PackageDetailState>,
    pub package_query: String,
    pub package_searching: bool,
    pub package_request_generation: u64,
    pub package_dependency_reverse: bool,
    pub package_dependency_selection: usize,
    pub package_navigation: Vec<PackageIdentity>,
    pub image_artifacts: ImageArtifactInventoryState,
    pub image_artifact_selection: Option<ImageArtifactIdentity>,
    pub image_artifact_query: String,
    pub image_artifact_searching: bool,
    pub image_artifact_request_generation: u64,
    pub sdk_artifacts: SdkArtifactInventoryState,
    pub sdk_artifact_selection: Option<SdkArtifactIdentity>,
    pub sdk_artifact_query: String,
    pub sdk_artifact_searching: bool,
    pub sdk_artifact_generation: u64,
    pub sdk_tool_capability: SdkToolCapability,
    pub sdk_sessions: VecDeque<SdkSession>,
    pub sdk_session_generation: u64,
    pub test_capability: TestCapability,
    pub test_family_selection: TestFamily,
    pub test_sessions: VecDeque<TestSession>,
    pub test_session_generation: u64,
    pub result_tool_capability: ResultToolCapability,
    pub test_view: TestWorkspaceView,
    pub test_results: TestResultInventoryState,
    pub test_result_selection: Option<TestResultIdentity>,
    pub test_result_query: String,
    pub test_result_searching: bool,
    pub test_result_drilled: bool,
    pub test_case_selection: Option<TestCaseIdentity>,
    pub test_result_generation: u64,
    pub test_comparison: TestComparisonState,
    pub test_comparison_selection: Option<TestCaseIdentity>,
    pub test_comparison_generation: u64,
    pub test_junit_export: TestJunitExportState,
    pub test_junit_generation: u64,
    pub security: SecurityState,
    pub qa: QaState,
    pub maintenance: MaintenanceState,
    pub qemu_capability: QemuCapability,
    pub qemu_sessions: VecDeque<QemuSession>,
    pub qemu_session_generation: u64,
    pub wic_capability: WicCapability,
    pub wic_outputs: WicOutputInventoryState,
    pub wic_output_selection: Option<WicOutputIdentity>,
    pub wic_output_generation: u64,
    pub wic_devices: WicDeviceInventoryState,
    pub wic_device_selection: Option<WicDeviceIdentity>,
    pub wic_device_generation: u64,
    pub wic_sessions: VecDeque<WicSession>,
    pub wic_session_generation: u64,
    pub layer_relationships: Option<LayerRelationships>,
    pub recipe_sources: HashMap<String, Vec<PathBuf>>,
    pub recipe_metadata: HashMap<String, RecipeMetadata>,
    pub devtool_statuses: HashMap<RecipeIdentity, DevtoolStatus>,
    pub devtool_status_loading: HashSet<RecipeIdentity>,
    pub variable_details: HashMap<VariableIdentity, VariableDetail>,
    pub variable_detail_loading: HashSet<VariableIdentity>,
    pub variable_detail_errors: HashMap<VariableIdentity, String>,
    pub recipe_metadata_loading: HashSet<String>,
    pub recipe_metadata_errors: HashMap<String, String>,
    pub layer_browser: Option<LayerBrowser>,
    pub dialogs: VecDeque<Dialog>,
    pub tasks: HashMap<TaskId, TaskInfo>,
    pub completed_tasks: VecDeque<CompletedTask>,
    pub task_progress_scroll: usize,
    pub task_filters: TaskFilters,
    pub task_filter_field: TaskFilterField,
    pub task_filter_editing: bool,
    pub logs: LogState,
    pub should_quit: bool,
    pub notification: Option<String>,
    pub command_palette_open: bool,
    pub command_palette_selection: usize,
    pub command_palette_query: String,
    pub error_selection: usize,
    pub recipe_selection: usize,
    pub layer_selection: usize,
    pub config_selection: usize,
    pub config_scope: Option<String>,
    pub metadata_query: String,
    pub metadata_searching: bool,
}
impl App {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            daemon: ClientDaemonView::default(),
            pty_selection: 0,
            pane_layout: PaneLayout::new(PaneId(1)).expect("valid root pane"),
            screen: Screen::Dashboard,
            focus: FocusTarget::Workspace,
            focus_return: None,
            navigator_selection: 0,
            backend: "unknown".into(),
            project_profile: ProjectProfileState::NotLoaded,
            project_profile_selection: 0,
            build_environment: BuildEnvironmentState::Connected(BuildEnvironmentProfile {
                source_dir: PathBuf::from("/"),
                build_dir: PathBuf::from("/"),
                init_script: PathBuf::from("/"),
            }),
            build_environment_generation: 0,
            build_environment_draft: None,
            available_images: Vec::new(),
            color_enabled: true,
            color_forced_off: false,
            theme: Theme::DarkPro,
            animation_speed: AnimationSpeed::Fast,
            reduced_motion: false,
            settings_selection: 0,
            settings_dirty: false,
            animation_frame: 0,
            workspace: Workspace::default(),
            host_telemetry: HostTelemetry::default(),
            host_cpu_history: VecDeque::new(),
            host_memory_history: VecDeque::new(),
            build: BuildState::default(),
            background_jobs: BackgroundJobs::default(),
            build_history: VecDeque::new(),
            build_history_selection: 0,
            dependencies: None,
            dependency_selection: 0,
            dependency_graph: DependencyGraphState::NotLoaded,
            dependency_graph_selection: None,
            signature_dump: SignatureDumpState::NotLoaded,
            signature_selection: None,
            signature_comparison: SignatureComparisonState::NotSelected,
            signature_recipe: None,
            package_inventory: PackageInventoryState::NotLoaded,
            package_selection: None,
            package_details: HashMap::new(),
            package_query: String::new(),
            package_searching: false,
            package_request_generation: 0,
            package_dependency_reverse: false,
            package_dependency_selection: 0,
            package_navigation: Vec::new(),
            image_artifacts: ImageArtifactInventoryState::NotLoaded,
            image_artifact_selection: None,
            image_artifact_query: String::new(),
            image_artifact_searching: false,
            image_artifact_request_generation: 0,
            sdk_artifacts: SdkArtifactInventoryState::NotLoaded,
            sdk_artifact_selection: None,
            sdk_artifact_query: String::new(),
            sdk_artifact_searching: false,
            sdk_artifact_generation: 0,
            sdk_tool_capability: SdkToolCapability::NotInspected,
            sdk_sessions: VecDeque::new(),
            sdk_session_generation: 0,
            test_capability: TestCapability::default(),
            test_family_selection: TestFamily::OeSelftest,
            test_sessions: VecDeque::new(),
            test_session_generation: 0,
            result_tool_capability: ResultToolCapability::default(),
            test_view: TestWorkspaceView::Launches,
            test_results: TestResultInventoryState::default(),
            test_result_selection: None,
            test_result_query: String::new(),
            test_result_searching: false,
            test_result_drilled: false,
            test_case_selection: None,
            test_result_generation: 0,
            test_comparison: TestComparisonState::default(),
            test_comparison_selection: None,
            test_comparison_generation: 0,
            test_junit_export: TestJunitExportState::default(),
            test_junit_generation: 0,
            security: SecurityState::default(),
            qa: QaState::default(),
            maintenance: MaintenanceState::default(),
            qemu_capability: QemuCapability::default(),
            qemu_sessions: VecDeque::new(),
            qemu_session_generation: 0,
            wic_capability: WicCapability::default(),
            wic_outputs: WicOutputInventoryState::default(),
            wic_output_selection: None,
            wic_output_generation: 0,
            wic_devices: WicDeviceInventoryState::default(),
            wic_device_selection: None,
            wic_device_generation: 0,
            wic_sessions: VecDeque::new(),
            wic_session_generation: 0,
            layer_relationships: None,
            recipe_sources: HashMap::new(),
            recipe_metadata: HashMap::new(),
            devtool_statuses: HashMap::new(),
            devtool_status_loading: HashSet::new(),
            variable_details: HashMap::new(),
            variable_detail_loading: HashSet::new(),
            variable_detail_errors: HashMap::new(),
            recipe_metadata_loading: HashSet::new(),
            recipe_metadata_errors: HashMap::new(),
            layer_browser: None,
            dialogs: VecDeque::new(),
            tasks: HashMap::new(),
            completed_tasks: VecDeque::new(),
            task_progress_scroll: 0,
            task_filters: TaskFilters::default(),
            task_filter_field: TaskFilterField::default(),
            task_filter_editing: false,
            logs: LogState::new(max_entries, max_bytes),
            should_quit: false,
            notification: None,
            command_palette_open: false,
            command_palette_selection: 0,
            command_palette_query: String::new(),
            error_selection: 0,
            recipe_selection: 0,
            layer_selection: 0,
            config_selection: 0,
            config_scope: None,
            metadata_query: String::new(),
            metadata_searching: false,
        }
    }
    pub fn new_unconfigured(max_entries: usize, max_bytes: usize) -> Self {
        let mut app = Self::new(max_entries, max_bytes);
        app.build_environment = BuildEnvironmentState::Unconfigured;
        app.screen = Screen::BuildEnvironment;
        app.focus = FocusTarget::Navigator;
        app
    }
    pub fn elapsed(&self) -> Option<Duration> {
        self.build
            .started
            .and_then(|s| SystemTime::now().duration_since(s).ok())
    }
    pub fn navigator_screen(&self) -> Screen {
        NAVIGATOR_SCREENS
            .get(self.navigator_selection)
            .copied()
            .unwrap_or(Screen::Dashboard)
    }
    pub fn waiting_task_count(&self) -> usize {
        if matches!(
            self.build.status,
            BuildStatus::Completed | BuildStatus::Cancelled | BuildStatus::Failed
        ) {
            return 0;
        }
        self.build.total.map_or(0, |total| {
            total.saturating_sub(self.build.completed.saturating_add(self.tasks.len()))
        })
    }
    pub fn visible_task_rows(&self) -> Vec<TaskRow> {
        let now = SystemTime::now();
        let state_matches = |state: TaskState| match self.task_filters.state {
            TaskStateFilter::All => true,
            TaskStateFilter::Active => state == TaskState::Active,
            TaskStateFilter::Waiting => state == TaskState::Waiting,
            TaskStateFilter::Completed => state == TaskState::Completed,
            TaskStateFilter::Failed => {
                matches!(
                    state,
                    TaskState::Failed | TaskState::Cancelled | TaskState::Lost
                )
            }
        };
        let text_matches = |task: &TaskInfo| {
            contains_case_insensitive(&task.recipe, &self.task_filters.recipe)
                && contains_case_insensitive(&task.task, &self.task_filters.task)
                && contains_case_insensitive(
                    task.worker.as_deref().unwrap_or(""),
                    &self.task_filters.worker,
                )
                && self.task_filters.minimum_duration.is_none_or(|minimum| {
                    task.elapsed_at(now)
                        .is_some_and(|elapsed| elapsed >= minimum)
                })
        };
        let retained = self
            .tasks
            .values()
            .cloned()
            .chain(self.completed_tasks.iter().map(|completed| {
                let mut task = completed.task.clone();
                if matches!(task.state, TaskState::Active | TaskState::Waiting) {
                    task.state = if completed.success {
                        TaskState::Completed
                    } else {
                        TaskState::Failed
                    };
                }
                task
            }));
        let mut rows = retained
            .filter(|task| state_matches(task.state) && text_matches(task))
            .map(|task| TaskRow::Task(Box::new(task)))
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            let TaskRow::Task(left) = left else {
                return std::cmp::Ordering::Less;
            };
            let TaskRow::Task(right) = right else {
                return std::cmp::Ordering::Greater;
            };
            (
                left.started.is_none(),
                left.started,
                task_state_order(left.state),
                left.recipe.as_str(),
                left.task.as_str(),
                left.id.0.as_str(),
            )
                .cmp(&(
                    right.started.is_none(),
                    right.started,
                    task_state_order(right.state),
                    right.recipe.as_str(),
                    right.task.as_str(),
                    right.id.0.as_str(),
                ))
        });
        let waiting = self.waiting_task_count();
        let waiting_filter_matches = matches!(
            self.task_filters.state,
            TaskStateFilter::All | TaskStateFilter::Waiting
        ) && self.task_filters.recipe.is_empty()
            && self.task_filters.task.is_empty()
            && self.task_filters.worker.is_empty()
            && self.task_filters.minimum_duration.is_none();
        if waiting > 0 && waiting_filter_matches {
            rows.push(TaskRow::WaitingSummary(waiting));
        }
        rows
    }
    pub fn selected_task_row(&self) -> Option<TaskRow> {
        self.visible_task_rows()
            .get(self.task_progress_scroll)
            .cloned()
    }
    pub fn filtered_packages(&self) -> Vec<&PackageSummary> {
        let query = self.package_query.to_ascii_lowercase();
        self.package_inventory
            .packages()
            .unwrap_or_default()
            .iter()
            .filter(|package| {
                query.is_empty()
                    || [
                        Some(package.identity.name.as_str()),
                        package.recipe.available().map(String::as_str),
                        package.version.available().map(String::as_str),
                        package.license.available().map(String::as_str),
                        package.provider.available().and_then(|path| path.to_str()),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|value| value.to_ascii_lowercase().contains(&query))
            })
            .collect()
    }
    pub fn selected_package(&self) -> Option<&PackageSummary> {
        let selected = self.package_selection.as_ref()?;
        self.filtered_packages()
            .into_iter()
            .find(|package| &package.identity == selected)
    }
    pub fn selected_package_detail(&self) -> Option<&PackageDetailState> {
        self.package_selection
            .as_ref()
            .and_then(|identity| self.package_details.get(identity))
    }
    pub fn selected_package_dependencies(&self) -> Option<&[PackageIdentity]> {
        let detail = self.selected_package_detail()?.detail()?;
        if self.package_dependency_reverse {
            detail.reverse_dependencies.available().map(Vec::as_slice)
        } else {
            detail.runtime_dependencies.available().map(Vec::as_slice)
        }
    }
    pub fn selected_package_dependency(&self) -> Option<&PackageIdentity> {
        self.selected_package_dependencies()?
            .get(self.package_dependency_selection)
    }
    pub fn filtered_image_artifacts(&self) -> Vec<&ImageArtifact> {
        self.image_artifacts
            .artifacts()
            .unwrap_or_default()
            .iter()
            .filter(|artifact| artifact.matches_query(&self.image_artifact_query))
            .collect()
    }
    pub fn selected_image_artifact(&self) -> Option<&ImageArtifact> {
        let selected = self.image_artifact_selection.as_ref()?;
        self.filtered_image_artifacts()
            .into_iter()
            .find(|artifact| &artifact.identity == selected)
    }
    pub fn filtered_sdk_artifacts(&self) -> Vec<&SdkArtifact> {
        self.sdk_artifacts
            .artifacts()
            .unwrap_or_default()
            .iter()
            .filter(|artifact| artifact.matches_query(&self.sdk_artifact_query))
            .collect()
    }
    pub fn selected_sdk_artifact(&self) -> Option<&SdkArtifact> {
        let selected = self.sdk_artifact_selection.as_ref()?;
        self.filtered_sdk_artifacts()
            .into_iter()
            .find(|artifact| &artifact.identity == selected)
    }
    pub fn sdk_session(&self, id: SdkSessionId) -> Option<&SdkSession> {
        self.sdk_sessions.iter().find(|session| session.id == id)
    }
    pub fn active_sdk_session(&self) -> Option<&SdkSession> {
        self.sdk_sessions.iter().rev().find(|session| {
            self.background_jobs
                .get(session.background_job_id)
                .is_some_and(|job| !job.status.is_terminal())
        })
    }
    pub fn latest_sdk_session(&self) -> Option<&SdkSession> {
        self.sdk_sessions.back()
    }
    pub fn test_session(&self, id: TestSessionId) -> Option<&TestSession> {
        self.test_sessions.iter().find(|session| session.id == id)
    }
    pub fn active_test_session(&self) -> Option<&TestSession> {
        self.test_sessions.iter().rev().find(|session| {
            session.background_job_id.is_none()
                || session.background_job_id.is_some_and(|job_id| {
                    self.background_jobs
                        .get(job_id)
                        .is_some_and(|job| !job.status.is_terminal())
                })
        })
    }
    pub fn latest_test_session(&self) -> Option<&TestSession> {
        self.test_sessions.back()
    }
    pub fn filtered_test_results(&self) -> Vec<&TestResultRecord> {
        let query = self.test_result_query.to_ascii_lowercase();
        self.test_results
            .records()
            .iter()
            .filter(|record| {
                query.is_empty()
                    || [
                        record.identity.path.to_str(),
                        Some(record.identity.fingerprint.as_str()),
                        record.machine.as_deref(),
                        record.image.as_deref(),
                        record.revision.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .chain(
                        record
                            .metadata
                            .iter()
                            .flat_map(|entry| [entry.key.as_str(), entry.value.as_str()]),
                    )
                    .any(|value| value.to_ascii_lowercase().contains(&query))
            })
            .collect()
    }
    pub fn selected_test_result(&self) -> Option<&TestResultRecord> {
        let selected = self.test_result_selection.as_ref()?;
        self.filtered_test_results()
            .into_iter()
            .find(|record| &record.identity == selected)
    }
    pub fn selected_test_case(&self) -> Option<&TestCaseRecord> {
        let identity = self.test_case_selection.as_ref()?;
        self.selected_test_result()?.case(identity)
    }
    pub fn test_comparison_transitions(&self) -> &[TestCaseTransition] {
        match &self.test_comparison {
            TestComparisonState::Available { comparison, .. }
            | TestComparisonState::Partial { comparison, .. } => &comparison.transitions,
            _ => &[],
        }
    }
    pub fn selected_test_transition(&self) -> Option<&TestCaseTransition> {
        let selected = self.test_comparison_selection.as_ref()?;
        self.test_comparison_transitions()
            .iter()
            .find(|transition| &transition.identity == selected)
    }
    pub fn qemu_session(&self, id: QemuSessionId) -> Option<&QemuSession> {
        self.qemu_sessions.iter().find(|session| session.id == id)
    }
    pub fn active_qemu_session(&self) -> Option<&QemuSession> {
        self.qemu_sessions.iter().rev().find(|session| {
            self.background_jobs
                .get(session.background_job_id)
                .is_some_and(|job| !job.status.is_terminal())
        })
    }
    pub fn latest_qemu_session(&self) -> Option<&QemuSession> {
        self.qemu_sessions.back()
    }
    pub fn wic_session(&self, id: WicSessionId) -> Option<&WicSession> {
        self.wic_sessions.iter().find(|session| session.id == id)
    }
    pub fn active_wic_session(&self) -> Option<&WicSession> {
        self.wic_sessions.iter().rev().find(|session| {
            self.background_jobs
                .get(session.background_job_id)
                .is_some_and(|job| !job.status.is_terminal())
        })
    }
    pub fn latest_wic_session(&self) -> Option<&WicSession> {
        self.wic_sessions.back()
    }
    pub fn wic_output_rows(&self) -> &[WicOutput] {
        match &self.wic_outputs {
            WicOutputInventoryState::Available { outputs, .. }
            | WicOutputInventoryState::Partial { outputs, .. } => outputs,
            _ => &[],
        }
    }
    pub fn selected_wic_output(&self) -> Option<&WicOutput> {
        let selected = self.wic_output_selection.as_ref()?;
        self.wic_output_rows()
            .iter()
            .find(|output| &output.identity == selected)
    }
    pub fn wic_device_rows(&self) -> &[WicDevice] {
        match &self.wic_devices {
            WicDeviceInventoryState::Available { devices, .. }
            | WicDeviceInventoryState::Partial { devices, .. } => devices,
            _ => &[],
        }
    }
    pub fn selected_wic_device(&self) -> Option<&WicDevice> {
        let selected = self.wic_device_selection.as_ref()?;
        self.wic_device_rows()
            .iter()
            .find(|device| &device.identity == selected)
    }
    pub fn selected_wic_write_image(&self) -> Result<WicOutputIdentity, String> {
        if self.wic_output_selection.is_some() {
            let output = self
                .selected_wic_output()
                .ok_or_else(|| "The selected generated Wic output is stale.".to_owned())?;
            if !matches!(output.kind, WicOutputKind::Wic | WicOutputKind::Direct)
                || !is_uncompressed_wic_path(&output.identity.path)
            {
                return Err("Select an uncompressed generated .wic or .direct image first.".into());
            }
            return Ok(output.identity.clone());
        }
        let artifact = self
            .selected_image_artifact()
            .ok_or_else(|| "Select a deployed Wic or generated Wic output first.".to_owned())?;
        if artifact.kind != ImageArtifactKind::Wic
            || !is_uncompressed_wic_path(&artifact.identity.path)
        {
            return Err("Select an uncompressed deployed .wic or .direct image first.".into());
        }
        let size_bytes = artifact
            .size_bytes
            .available()
            .copied()
            .ok_or_else(|| "The selected Wic image size is unavailable.".to_owned())?;
        let modified_unix_seconds = artifact
            .modified_unix_seconds
            .available()
            .copied()
            .ok_or_else(|| "The selected Wic image timestamp is unavailable.".to_owned())?;
        Ok(WicOutputIdentity {
            path: artifact.identity.path.clone(),
            size_bytes,
            modified_unix_seconds,
        })
    }
    pub fn wic_device_write_unavailable_reason(&self) -> Option<String> {
        if self.active_wic_session().is_some() {
            return Some("A managed Wic operation is already active.".into());
        }
        match &self.wic_capability {
            WicCapability::Available { executable, .. }
            | WicCapability::MissingKickstarts { executable }
                if executable.is_absolute() => {}
            WicCapability::NotInspected => {
                return Some("Wic capability has not been inspected.".into());
            }
            WicCapability::MissingTool => return Some("wic is not available.".into()),
            WicCapability::Failed { message } => {
                return Some(format!("Wic capability inspection failed: {message}"));
            }
            WicCapability::Available { .. } | WicCapability::MissingKickstarts { .. } => {
                return Some("The inspected Wic executable identity is invalid.".into());
            }
        }
        self.selected_wic_write_image().err()
    }
    pub fn wic_create_unavailable_reason(&self) -> Option<String> {
        if self.active_wic_session().is_some() {
            return Some("A managed Wic operation is already active.".into());
        }
        let Some(artifact) = self.selected_image_artifact() else {
            return Some("Select a deployed image artifact first.".into());
        };
        let WicCapability::Available {
            kickstarts,
            image_targets,
            ..
        } = &self.wic_capability
        else {
            return Some(match &self.wic_capability {
                WicCapability::NotInspected => "Wic capability has not been inspected.".into(),
                WicCapability::MissingTool => "wic is not available.".into(),
                WicCapability::MissingKickstarts { .. } => {
                    "No Wic kickstarts are available.".into()
                }
                WicCapability::Failed { message } => {
                    format!("Wic capability inspection failed: {message}")
                }
                WicCapability::Available { .. } => unreachable!(),
            });
        };
        if kickstarts.is_empty()
            || !image_targets
                .iter()
                .any(|target| target == &artifact.identity.image)
        {
            return Some("The selected image is not in the inspected Wic capability.".into());
        }
        None
    }
    pub fn qemu_launch_unavailable_reason(&self) -> Option<String> {
        if self.active_qemu_session().is_some() {
            return Some("A managed runqemu session is already active.".into());
        }
        let Some(artifact) = self.selected_image_artifact() else {
            return Some("Select a deployed image artifact first.".into());
        };
        if !matches!(
            artifact.kind,
            ImageArtifactKind::RootFilesystem | ImageArtifactKind::Wic
        ) {
            return Some("runqemu requires a root filesystem or Wic artifact.".into());
        }
        match &self.qemu_capability {
            QemuCapability::NotInspected => {
                Some("runqemu capability has not been inspected.".into())
            }
            QemuCapability::MissingTool => Some("runqemu is not available.".into()),
            QemuCapability::MissingCompatibleImage => {
                Some("No compatible deployed runqemu image is available.".into())
            }
            QemuCapability::Failed { message } => {
                Some(format!("runqemu capability inspection failed: {message}"))
            }
            QemuCapability::Available {
                executable: _,
                compatible_images,
            } if !compatible_images.contains(&artifact.identity) => {
                Some("The selected artifact is not in the inspected runqemu capability.".into())
            }
            QemuCapability::Available { .. } => None,
        }
    }
    pub fn active_dialog(&self) -> Option<&Dialog> {
        self.dialogs.front()
    }
    pub fn active_dialog_mut(&mut self) -> Option<&mut Dialog> {
        self.dialogs.front_mut()
    }
    pub fn command_palette_commands(&self) -> Vec<PaletteCommand> {
        let workspace_missing = self.workspace.build_dir.is_none();
        let recipe_missing = self.screen != Screen::Recipes
            || self.workspace.recipes.get(self.recipe_selection).is_none();
        vec![
            PaletteCommand {
                id: CommandId::BuildImage,
                label: "Build image",
                description: "Open image build options for the active machine",
                shortcut: "F5",
                disabled_reason: workspace_missing.then_some("Load a Yocto workspace first"),
            },
            PaletteCommand {
                id: CommandId::SelectImage,
                label: "Select image",
                description: "Choose an image recipe discovered in active layers",
                shortcut: "i",
                disabled_reason: (!self
                    .workspace
                    .recipes
                    .iter()
                    .any(|recipe| recipe.name.contains("image")))
                .then_some("No image recipes are available"),
            },
            PaletteCommand {
                id: CommandId::BuildSelectedRecipe,
                label: "Build selected recipe",
                description: "Confirm and build the selected recipe",
                shortcut: "b",
                disabled_reason: recipe_missing.then_some("Open Recipes and select a recipe"),
            },
            PaletteCommand {
                id: CommandId::EditBbmask,
                label: "Edit BBMASK",
                description: "Preview and save the effective BBMASK value",
                shortcut: "x then e",
                disabled_reason: workspace_missing.then_some("Load a Yocto workspace first"),
            },
            PaletteCommand {
                id: CommandId::OpenDashboard,
                label: "Open Dashboard",
                description: "Show build status, task progress, and recent output",
                shortcut: "Esc",
                disabled_reason: None,
            },
            PaletteCommand {
                id: CommandId::OpenLayers,
                label: "Open Layers",
                description: "Browse active layer metadata and files",
                shortcut: "y",
                disabled_reason: None,
            },
            PaletteCommand {
                id: CommandId::OpenRecipes,
                label: "Open Recipes",
                description: "Browse recipes and typed recipe actions",
                shortcut: "r",
                disabled_reason: None,
            },
            PaletteCommand {
                id: CommandId::OpenImages,
                label: "Open Images",
                description: "Browse image recipes and artifacts",
                shortcut: "i",
                disabled_reason: None,
            },
            PaletteCommand {
                id: CommandId::OpenTasks,
                label: "Open Tasks",
                description: "Inspect active and completed BitBake tasks",
                shortcut: "t",
                disabled_reason: None,
            },
            PaletteCommand {
                id: CommandId::OpenLogs,
                label: "Open Logs",
                description: "Inspect retained structured build logs",
                shortcut: "l",
                disabled_reason: None,
            },
            PaletteCommand {
                id: CommandId::OpenErrors,
                label: "Open Errors",
                description: "Inspect retained warnings and errors",
                shortcut: "e",
                disabled_reason: None,
            },
            PaletteCommand {
                id: CommandId::OpenConfiguration,
                label: "Open Configuration",
                description: "Inspect effective BitBake variables and provenance",
                shortcut: "v",
                disabled_reason: None,
            },
            PaletteCommand {
                id: CommandId::OpenSettings,
                label: "Open Settings",
                description: "Edit persistent visual and log preferences",
                shortcut: "none",
                disabled_reason: None,
            },
            PaletteCommand {
                id: CommandId::ChooseTheme,
                label: "Choose theme",
                description: "Preview and apply a named workbench palette",
                shortcut: "Ctrl+P theme",
                disabled_reason: None,
            },
            PaletteCommand {
                id: CommandId::OpenHelp,
                label: "Open Help",
                description: "Show all global and contextual shortcuts",
                shortcut: "?",
                disabled_reason: None,
            },
        ]
    }
    pub fn filtered_command_palette_commands(&self) -> Vec<PaletteCommand> {
        let query = self.command_palette_query.trim().to_lowercase();
        self.command_palette_commands()
            .into_iter()
            .filter(|command| {
                query.is_empty()
                    || command.label.to_lowercase().contains(&query)
                    || command.description.to_lowercase().contains(&query)
                    || command.shortcut.to_lowercase().contains(&query)
            })
            .collect()
    }
}
fn contains_case_insensitive(value: &str, query: &str) -> bool {
    query.is_empty() || value.to_lowercase().contains(&query.to_lowercase())
}
fn task_state_order(state: TaskState) -> u8 {
    match state {
        TaskState::Active => 0,
        TaskState::Waiting => 1,
        TaskState::Failed | TaskState::Cancelled | TaskState::Lost => 2,
        TaskState::Completed => 3,
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Tick,
    ProjectProfileAbsent,
    ProjectProfileLoaded(ProjectProfile),
    ProjectProfileLoadFailed(String),
    PreviewProjectProfileGeneration(ProjectProfile),
    ConfirmProjectProfileGeneration {
        replace: bool,
    },
    ProjectProfileGenerated(ProjectProfile),
    ProjectProfileGenerationFailed(String),
    SelectProjectProfileItem {
        delta: isize,
    },
    ActivateProjectProfileItem,
    Open(Screen),
    SelectNavigator {
        delta: isize,
    },
    SelectPtySession {
        delta: isize,
    },
    ResizeFocusedPane {
        delta_per_mille: i16,
    },
    ActivateNavigator,
    CycleFocus {
        backwards: bool,
    },
    Focus(FocusTarget),
    OpenCommandPalette,
    SelectCommandPalette {
        delta: isize,
    },
    AppendCommandPaletteQuery(char),
    BackspaceCommandPaletteQuery,
    ActivateCommandPalette,
    CloseCommandPalette,
    SelectSetting {
        delta: isize,
    },
    ChangeSelectedSetting {
        backwards: bool,
    },
    RetrySettingsPersistence,
    SettingsPersisted,
    SettingsPersistenceFailed(String),
    EditActivePopup(PopupEditorCommand),
    ConfigureBuildEnvironment(BuildEnvironmentProfile),
    OpenBuildEnvironmentCloneEditor,
    ToggleBuildEnvironmentCloneEditor,
    AppendBuildEnvironmentCloneEditor(char),
    BackspaceBuildEnvironmentCloneEditor,
    ReviewBuildEnvironmentClone,
    ConfirmBuildEnvironmentClone,
    CancelBuildEnvironmentClone,
    OpenBuildEnvironmentEditor,
    ToggleBuildEnvironmentEditor,
    AppendBuildEnvironmentEditor(char),
    BackspaceBuildEnvironmentEditor,
    ApplyBuildEnvironmentEditor,
    CloseBuildEnvironmentEditor,
    BeginBuildEnvironmentEdit,
    OpenThemePicker,
    SelectTheme {
        delta: isize,
    },
    ApplySelectedTheme,
    CloseThemePicker,
    SelectBuildEnvironmentField {
        delta: isize,
    },
    AppendBuildEnvironmentField(char),
    BackspaceBuildEnvironmentField,
    FinishBuildEnvironmentEdit,
    CancelBuildEnvironmentEdit,
    ApplyBuildEnvironmentProfile,
    BeginBuildEnvironmentVerification,
    BuildEnvironmentVerified {
        generation: u64,
    },
    BuildEnvironmentVerificationFailed {
        generation: u64,
        message: String,
    },
    OpenBuildOptions,
    CloseBuildOptions,
    OpenImagePicker(Vec<String>),
    SelectImage {
        delta: isize,
    },
    ConfirmImagePicker,
    CancelImagePicker,
    BeginCurrentImageBuild,
    BeginImageArtifactInventory,
    RefreshImageArtifactInventory,
    CancelImageArtifactOperation,
    ImageArtifactInventoryLoaded {
        request: ImageArtifactRequest,
        inventory: ImageArtifactInventory,
    },
    ImageArtifactInventoryPartial {
        request: ImageArtifactRequest,
        inventory: ImageArtifactInventory,
        limitations: Vec<String>,
    },
    ImageArtifactInventoryFailed {
        request: ImageArtifactRequest,
        message: String,
    },
    SelectImageArtifact {
        delta: isize,
    },
    BeginImageArtifactSearch,
    AppendImageArtifactQuery(char),
    BackspaceImageArtifactQuery,
    FinishImageArtifactSearch,
    BeginSelectedImageArtifactBuild,
    OpenSelectedImageArtifact,
    OpenSelectedImageArtifactAssociation(ImageArtifactAssociation),
    BeginSdkBuild(SdkBuildAction),
    ConfirmSdkBuild,
    CancelSdkBuild,
    BeginSdkArtifactInventory,
    RefreshSdkArtifactInventory,
    SdkArtifactInventoryLoaded {
        request: SdkArtifactInventoryRequest,
        artifacts: Vec<SdkArtifact>,
        limitations: Vec<String>,
    },
    SdkArtifactInventoryFailed {
        request: SdkArtifactInventoryRequest,
        message: String,
    },
    SelectSdkArtifact {
        delta: isize,
    },
    BeginSdkArtifactSearch,
    AppendSdkArtifactQuery(char),
    BackspaceSdkArtifactQuery,
    FinishSdkArtifactSearch,
    OpenSelectedSdkArtifact,
    SdkToolCapabilityLoaded(SdkToolCapability),
    BeginSelectedSdkPublish,
    ToggleSdkPublishTomlEditor,
    AppendSdkPublishTomlEditor(char),
    BackspaceSdkPublishTomlEditor,
    AppendSdkPublishDestination(char),
    BackspaceSdkPublishDestination,
    PreviewSdkPublish,
    CancelSdkPublish,
    CancelSdkPublishPreview,
    ConfirmSdkPublish,
    BeginSdkNative,
    ToggleSdkNativeTomlEditor,
    AppendSdkNativeTomlEditor(char),
    BackspaceSdkNativeTomlEditor,
    UpdateSdkNativeDraft(SdkNativeDraft),
    SelectSdkNativeField {
        delta: isize,
    },
    ActivateSdkNativeField,
    CycleSdkNativeMode,
    AppendSdkNativeField(char),
    BackspaceSdkNativeField,
    FinishSdkNativeFieldEdit,
    PreviewSdkNative,
    CancelSdkNative,
    CancelSdkNativePreview,
    ConfirmSdkNative,
    SdkSessionStarting {
        id: SdkSessionId,
        started_at: SystemTime,
    },
    SdkSessionRunning {
        id: SdkSessionId,
    },
    AppendSdkSessionOutput {
        id: SdkSessionId,
        stream: SdkOutputStream,
        line: String,
        truncated: bool,
        timestamp: SystemTime,
    },
    CompleteSdkSession {
        id: SdkSessionId,
        exit_code: i32,
        artifacts: Vec<PathBuf>,
        finished_at: SystemTime,
    },
    FailSdkSession {
        id: SdkSessionId,
        message: String,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    LoseSdkSession {
        id: SdkSessionId,
        message: String,
        finished_at: SystemTime,
    },
    BeginActiveSdkSessionCancellation,
    ConfirmSdkSessionCancellation,
    CancelSdkSessionCancellation,
    RejectSdkSessionCancellation {
        id: SdkSessionId,
        message: String,
    },
    CancelSdkSession {
        id: SdkSessionId,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    InspectTestCapability,
    TestCapabilityLoaded(TestCapability),
    SelectTestFamily {
        delta: isize,
    },
    BeginSelectedTestLaunch,
    ToggleTestLaunchTomlEditor,
    AppendTestLaunchTomlEditor(char),
    BackspaceTestLaunchTomlEditor,
    UpdateTestLaunchDraft(TestLaunchDraft),
    SelectTestLaunchField {
        delta: isize,
    },
    ActivateTestLaunchField,
    AppendTestLaunchField(char),
    BackspaceTestLaunchField,
    FinishTestLaunchFieldEdit,
    PreviewTestLaunch,
    CancelTestLaunch,
    CancelTestLaunchPreview,
    ConfirmTestLaunch,
    AttachTestBuildSession {
        id: TestSessionId,
        background_job_id: BackgroundJobId,
    },
    TestSessionStarting {
        id: TestSessionId,
        started_at: SystemTime,
    },
    TestSessionRunning {
        id: TestSessionId,
    },
    AppendTestSessionOutput {
        id: TestSessionId,
        stream: TestOutputStream,
        line: String,
        truncated: bool,
        timestamp: SystemTime,
    },
    CompleteTestSession {
        id: TestSessionId,
        exit_code: i32,
        result_paths: Vec<PathBuf>,
        finished_at: SystemTime,
    },
    FailTestSession {
        id: TestSessionId,
        message: String,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    TimeoutTestSession {
        id: TestSessionId,
        forced: bool,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    LoseTestSession {
        id: TestSessionId,
        message: String,
        finished_at: SystemTime,
    },
    BeginActiveTestSessionCancellation,
    ConfirmTestSessionCancellation,
    CancelTestSessionCancellation,
    RejectTestSessionCancellation {
        id: TestSessionId,
        message: String,
    },
    CancelTestSession {
        id: TestSessionId,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    InspectResultToolCapability,
    ResultToolCapabilityLoaded(ResultToolCapability),
    CycleTestView,
    BeginTestResultImport,
    ToggleTestResultImportTomlEditor,
    AppendTestResultImportTomlEditor(char),
    BackspaceTestResultImportTomlEditor,
    AppendTestResultImport(char),
    BackspaceTestResultImport,
    ConfirmTestResultImport,
    CancelTestResultImport,
    RefreshTestResults,
    TestResultsLoaded {
        request: TestResultImportRequest,
        records: Vec<TestResultRecord>,
        limitations: Vec<String>,
    },
    TestResultsFailed {
        request: TestResultImportRequest,
        message: String,
    },
    TestResultsCancelled {
        request: TestResultImportRequest,
    },
    TestResultsTimedOut {
        request: TestResultImportRequest,
    },
    TestResultsLost {
        request: TestResultImportRequest,
        message: String,
    },
    SelectTestResult {
        delta: isize,
    },
    BeginTestResultSearch,
    AppendTestResultQuery(char),
    BackspaceTestResultQuery,
    FinishTestResultSearch,
    OpenSelectedTestResult,
    DrillIntoSelectedTestResult,
    LeaveTestResultCases,
    SelectTestCase {
        delta: isize,
    },
    OpenSelectedTestCaseLog,
    BeginTestComparison,
    ToggleTestComparisonTomlEditor,
    AppendTestComparisonTomlEditor(char),
    BackspaceTestComparisonTomlEditor,
    SelectTestComparisonChoice {
        delta: isize,
    },
    CycleTestComparisonField,
    ActivateTestComparisonChoice,
    PreviewTestComparison,
    CancelTestComparison,
    CancelTestComparisonPreview,
    ConfirmTestComparison,
    TestComparisonLoaded {
        request: TestComparisonRequest,
        comparison: TestComparison,
        limitations: Vec<String>,
    },
    TestComparisonFailed {
        request: TestComparisonRequest,
        message: String,
    },
    TestComparisonCancelled {
        request: TestComparisonRequest,
    },
    TestComparisonTimedOut {
        request: TestComparisonRequest,
    },
    TestComparisonLost {
        request: TestComparisonRequest,
        message: String,
    },
    SelectTestComparisonTransition {
        delta: isize,
    },
    OpenSelectedTestTransitionLog,
    BeginTestJunitExport,
    ToggleTestJunitTomlEditor,
    AppendTestJunitTomlEditor(char),
    BackspaceTestJunitTomlEditor,
    MoveTestJunitTomlEditorLeft,
    MoveTestJunitTomlEditorRight,
    MoveTestJunitTomlEditorUp,
    MoveTestJunitTomlEditorDown,
    MoveTestJunitTomlEditorHome,
    MoveTestJunitTomlEditorEnd,
    SelectTestJunitDestination,
    CopyTestJunitTomlEditor,
    PasteTestJunitTomlEditor,
    AppendTestJunitDestination(char),
    BackspaceTestJunitDestination,
    PreviewTestJunitExport,
    CancelTestJunitExport,
    TestJunitDestinationInspected {
        result: TestResultIdentity,
        inspection: TestJunitDestinationInspection,
    },
    CancelTestJunitExportPreview,
    ConfirmTestJunitExport,
    TestJunitExportSucceeded {
        request: TestJunitExportRequest,
    },
    TestJunitExportFailed {
        request: TestJunitExportRequest,
        message: String,
    },
    TestJunitExportCancelled {
        request: TestJunitExportRequest,
    },
    TestJunitExportTimedOut {
        request: TestJunitExportRequest,
    },
    TestJunitExportLost {
        request: TestJunitExportRequest,
        message: String,
    },
    Security(SecurityAction),
    Qa(QaAction),
    Maintenance(MaintenanceAction),
    InspectQemuCapability,
    QemuCapabilityLoaded(QemuCapability),
    BeginSelectedQemuLaunch,
    UpdateQemuLaunchDraft(QemuLaunchDraft),
    SelectQemuLaunchField {
        delta: isize,
    },
    ActivateQemuLaunchField,
    CycleQemuLaunchChoice {
        backwards: bool,
    },
    AppendQemuLaunchField(char),
    BackspaceQemuLaunchField,
    FinishQemuLaunchFieldEdit,
    PreviewQemuLaunch,
    CancelQemuLaunch,
    CancelQemuLaunchPreview,
    ConfirmQemuLaunch,
    QemuSessionStarting {
        id: QemuSessionId,
        started_at: SystemTime,
    },
    QemuSessionRunning {
        id: QemuSessionId,
    },
    AppendQemuSessionOutput {
        id: QemuSessionId,
        stream: QemuOutputStream,
        line: String,
        truncated: bool,
        timestamp: SystemTime,
    },
    CompleteQemuSession {
        id: QemuSessionId,
        exit_code: i32,
        finished_at: SystemTime,
    },
    FailQemuSession {
        id: QemuSessionId,
        message: String,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    LoseQemuSession {
        id: QemuSessionId,
        message: String,
        finished_at: SystemTime,
    },
    BeginQemuSessionCancellation {
        id: QemuSessionId,
    },
    BeginActiveQemuSessionCancellation,
    ConfirmQemuSessionCancellation,
    CancelQemuSessionCancellation,
    RejectQemuSessionCancellation {
        id: QemuSessionId,
        message: String,
    },
    CancelQemuSession {
        id: QemuSessionId,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    InspectWicCapability,
    WicCapabilityLoaded(WicCapability),
    BeginSelectedWicCreate,
    ToggleWicCreateTomlEditor,
    AppendWicCreateTomlEditor(char),
    BackspaceWicCreateTomlEditor,
    SelectWicCreateField {
        delta: isize,
    },
    ActivateWicCreateField,
    CycleWicCreateChoice {
        backwards: bool,
    },
    AppendWicCreateField(char),
    BackspaceWicCreateField,
    FinishWicCreateFieldEdit,
    PreviewWicCreate,
    CancelWicCreate,
    CancelWicCreatePreview,
    ConfirmWicCreate,
    SelectWicOutput {
        delta: isize,
    },
    OpenSelectedWicOutput,
    BeginActiveWicSessionCancellation,
    BeginActiveImageRuntimeCancellation,
    CancelWicSessionCancellation,
    BeginWicOutputInventory(WicOutputInventoryRequest),
    WicOutputInventoryLoaded {
        request: WicOutputInventoryRequest,
        outputs: Vec<WicOutput>,
        limitations: Vec<String>,
    },
    WicOutputInventoryFailed {
        request: WicOutputInventoryRequest,
        message: String,
    },
    BeginWicDeviceInventory(WicDeviceInventoryRequest),
    WicDeviceInventoryLoaded {
        request: WicDeviceInventoryRequest,
        devices: Vec<WicDevice>,
        limitations: Vec<String>,
    },
    WicDeviceInventoryFailed {
        request: WicDeviceInventoryRequest,
        message: String,
    },
    BeginSelectedWicDeviceWrite,
    SelectWicDevice {
        delta: isize,
    },
    ConfirmWicDeviceSelection,
    CancelWicDevicePicker,
    AppendWicWritePhrase(char),
    BackspaceWicWritePhrase,
    PreviewWicDeviceWrite,
    CancelWicWritePhrase,
    ConfirmWicDeviceWrite,
    CancelWicWritePreview,
    StartConfirmedWicCreate(WicCreatePreview),
    WicSessionStarting {
        id: WicSessionId,
        started_at: SystemTime,
    },
    WicSessionRunning {
        id: WicSessionId,
    },
    AppendWicSessionOutput {
        id: WicSessionId,
        stream: WicOutputStream,
        line: String,
        truncated: bool,
        timestamp: SystemTime,
    },
    CompleteWicSession {
        id: WicSessionId,
        exit_code: i32,
        outputs: Vec<WicOutput>,
        limitations: Vec<String>,
        finished_at: SystemTime,
    },
    FailWicSession {
        id: WicSessionId,
        message: String,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    LoseWicSession {
        id: WicSessionId,
        message: String,
        finished_at: SystemTime,
    },
    ConfirmWicSessionCancellation {
        id: WicSessionId,
        acknowledge_incomplete_device: bool,
    },
    RejectWicSessionCancellation {
        id: WicSessionId,
        message: String,
    },
    CancelWicSession {
        id: WicSessionId,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    BeginBuildTargetEdit,
    ToggleBuildTargetEdit,
    BeginBuildTargetTask(Option<String>),
    AppendBuildTarget(char),
    BackspaceBuildTarget,
    ConfirmBuildTarget,
    CancelBuildTargetEdit,
    Start(BuildRequest),
    QueueBackgroundJob(BackgroundJobSpec),
    StartBackgroundJob {
        id: BackgroundJobId,
        started_at: SystemTime,
    },
    RunBackgroundJob {
        id: BackgroundJobId,
    },
    UpdateBackgroundJobProgress {
        id: BackgroundJobId,
        progress: BackgroundJobProgress,
    },
    AppendBackgroundJobOutput {
        id: BackgroundJobId,
        entry: BackgroundJobOutputEntry,
    },
    RequestBackgroundJobCancellation {
        id: BackgroundJobId,
    },
    RejectBackgroundJobCancellation {
        id: BackgroundJobId,
    },
    SucceedBackgroundJob {
        id: BackgroundJobId,
        result: BackgroundJobResult,
        finished_at: SystemTime,
    },
    FailBackgroundJob {
        id: BackgroundJobId,
        error: BackgroundJobError,
        finished_at: SystemTime,
    },
    CancelBackgroundJob {
        id: BackgroundJobId,
        finished_at: SystemTime,
    },
    LoseBackgroundJob {
        id: BackgroundJobId,
        error: BackgroundJobError,
        finished_at: SystemTime,
    },
    BuildRequested {
        target: Option<String>,
    },
    BuildStarted,
    ParseProgress {
        current: Option<u64>,
        total: Option<u64>,
    },
    TaskStarted(TaskInfo),
    TaskQueued(TaskInfo),
    TaskProgress {
        id: TaskId,
        progress: Option<u8>,
    },
    TaskCompleted {
        id: TaskId,
        success: bool,
    },
    ScrollBuildTasks {
        delta: isize,
    },
    CycleTaskStateFilter,
    CycleTaskFilterField,
    BeginTaskFilterEdit,
    AppendTaskFilter(char),
    BackspaceTaskFilter,
    FinishTaskFilterEdit,
    CycleTaskDurationFilter,
    Log(LogEntry),
    BuildCompleted {
        success: bool,
        exit_code: Option<i32>,
    },
    BuildCancelled {
        exit_code: Option<i32>,
    },
    BuildCancellationRejected(String),
    DismissBuildCompletion,
    OpenBuildCompletionErrors,
    SelectBuildHistory {
        delta: isize,
    },
    Cancel,
    ToggleLogFollow,
    ToggleLogWrap,
    CycleLogSeverity,
    ScrollLogs {
        delta: isize,
    },
    BeginLogSearch,
    AppendLogQuery(char),
    BackspaceLogQuery,
    FinishLogSearch,
    NextLogMatch,
    PreviousLogMatch,
    ScrollLogsHorizontally {
        delta: isize,
    },
    CycleLogRecipeFilter,
    CycleLogTaskFilter,
    CycleLogBuildFilter,
    OpenSelectedLogSource,
    CopySelectedLog,
    SelectError {
        delta: isize,
    },
    JumpToSelectedError,
    OpenSelectedErrorSource,
    SelectRecipe {
        delta: isize,
    },
    BeginSelectedRecipeBuild,
    BeginSelectedRecipeClean,
    BeginSelectedRecipeMenuConfig,
    BeginSelectedRecipeCleanState,
    BeginSelectedRecipeDevshell,
    BeginSelectedRecipeDiffconfig,
    BeginSelectedRecipeDiffsigs,
    BeginSelectedRecipeSignatures,
    BeginSelectedRecipeCveCheck,
    BeginSelectedRecipeSpdx,
    BeginSelectedRecipeForceTask,
    BeginSelectedRecipeTask {
        task: Option<String>,
        force: bool,
    },
    SelectRecipeTask {
        delta: isize,
    },
    PreviewSelectedRecipeTask,
    CancelRecipeTaskPicker,
    SelectSignatureTask {
        delta: isize,
    },
    ConfirmSignatureTask,
    CancelSignatureTaskPicker,
    OpenSelectedRecipeProvider,
    BeginSelectedRecipeTaskLog,
    SelectRecipeTaskLog {
        delta: isize,
    },
    OpenSelectedRecipeTaskLog,
    CancelRecipeTaskLogPicker,
    BeginSelectedRecipePatchReview,
    SelectRecipePatch {
        delta: isize,
    },
    OpenSelectedRecipePatch,
    CancelRecipePatchPicker,
    BeginSelectedRecipeDevtoolModify,
    ConfirmDevtoolModify,
    CancelDevtoolModify,
    BeginSelectedRecipeDevtoolStatus,
    DevtoolStatusLoaded(DevtoolStatus),
    BeginSelectedRecipeDevtoolReset,
    BeginSelectedRecipeDevtoolUpdateRecipe,
    BeginSelectedRecipeDevtoolFinish,
    BeginSelectedRecipeDevtoolDeploy,
    BeginSelectedRecipeDependencies,
    BeginDependencyGraph {
        root: DependencyNodeId,
    },
    DependencyGraphLoaded(DependencyGraph),
    DependencyGraphPartial {
        graph: DependencyGraph,
        limitations: Vec<String>,
    },
    DependencyGraphFailed {
        root: DependencyNodeId,
        message: String,
    },
    SelectDependencyGraphNode {
        delta: isize,
    },
    RefreshDependencyGraph,
    OpenSelectedDependencyRecipe,
    OpenSelectedDependencyProvider,
    OpenSelectedDependencyTaskLog,
    BeginSignatureDump(SignatureTarget),
    RefreshSignatureDump,
    LeaveSignatureWorkspace,
    OpenSignatureProvider,
    SignatureDumpLoaded {
        target: SignatureTarget,
        records: Vec<SignatureRecord>,
    },
    SignatureDumpPartial {
        target: SignatureTarget,
        records: Vec<SignatureRecord>,
        limitations: Vec<String>,
    },
    SignatureDumpFailed {
        target: SignatureTarget,
        message: String,
    },
    SelectSignatureRecord {
        delta: isize,
    },
    SetSelectedSignatureComparisonSide(SignatureComparisonSide),
    BeginSignatureComparison,
    SignatureComparisonLoaded {
        request: SignatureComparisonRequest,
        differences: Vec<SignatureDifference>,
    },
    SignatureComparisonPartial {
        request: SignatureComparisonRequest,
        differences: Vec<SignatureDifference>,
        limitations: Vec<String>,
    },
    SignatureComparisonFailed {
        request: SignatureComparisonRequest,
        message: String,
    },
    BeginPackageInventory,
    RefreshPackageInventory,
    CancelPackageOperation,
    PackageInventoryLoaded {
        request: PackageInventoryRequest,
        packages: Vec<PackageSummary>,
    },
    PackageInventoryPartial {
        request: PackageInventoryRequest,
        packages: Vec<PackageSummary>,
        limitations: Vec<String>,
    },
    PackageInventoryFailed {
        request: PackageInventoryRequest,
        message: String,
    },
    SelectPackage {
        delta: isize,
    },
    BeginPackageSearch,
    AppendPackageQuery(char),
    BackspacePackageQuery,
    FinishPackageSearch,
    BeginSelectedPackageDetail,
    PackageDetailLoaded {
        request: PackageDetailRequest,
        detail: PackageDetail,
    },
    PackageDetailPartial {
        request: PackageDetailRequest,
        detail: PackageDetail,
        limitations: Vec<String>,
    },
    PackageDetailFailed {
        request: PackageDetailRequest,
        message: String,
    },
    OpenPackageDependency {
        identity: PackageIdentity,
        reverse: bool,
    },
    TogglePackageDependencyKind,
    SelectPackageDependency {
        delta: isize,
    },
    OpenSelectedPackageDependency,
    BackPackageNavigation,
    OpenSelectedPackageRecipe,
    OpenSelectedPackageProvider,
    BeginSelectedRecipeMetadata,
    RecipeMetadataLoaded(RecipeMetadata),
    RecipeMetadataFailed {
        recipe: String,
        message: String,
    },
    DependenciesLoaded(RecipeDependencies),
    SelectDependency {
        delta: isize,
    },
    OpenSelectedDependency,
    OpenRecipeEditor {
        recipe: String,
        root: PathBuf,
        files: Vec<PathBuf>,
    },
    SelectRecipeEditorFile {
        delta: isize,
    },
    LoadRecipeEditorContent(String),
    ToggleRecipeEditorEditing,
    AppendRecipeEditor(char),
    BackspaceRecipeEditor,
    SaveRecipeEditor,
    RecipeEditorSaved,
    BeginRecipeEditorBuild,
    CloseRecipeEditor,
    ConfirmRecipeTask,
    CancelRecipeTask,
    ConfirmDevtoolReset,
    CancelDevtoolReset,
    ConfirmDevtoolUpdateRecipe,
    CancelDevtoolUpdateRecipe,
    SelectDevtoolFinishLayer {
        delta: isize,
    },
    PreviewDevtoolFinish,
    CancelDevtoolFinish,
    ConfirmDevtoolFinish,
    CancelDevtoolFinishConfirmation,
    AppendDevtoolDeployTarget(char),
    BackspaceDevtoolDeployTarget,
    PreviewDevtoolDeploy,
    CancelDevtoolDeploy,
    ConfirmDevtoolDeploy,
    CancelDevtoolDeployConfirmation,
    SelectLayer {
        delta: isize,
    },
    OpenSelectedLayer,
    BeginSelectedLayerWorkspaceEditor,
    BeginSelectedLayerBrowser,
    LoadLayerBrowserDirectory {
        layer: String,
        root: PathBuf,
        directory: PathBuf,
        entries: Vec<LayerBrowserEntry>,
    },
    SelectLayerBrowserEntry {
        delta: isize,
    },
    LayerBrowserExpand,
    LayerBrowserEnter,
    LayerBrowserUp,
    CloseLayerBrowser,
    RefreshLayerBrowser,
    ToggleLayerBrowserHidden,
    SetLayerInspectorMode(LayerInspectorMode),
    LoadLayerBrowserPreview {
        path: PathBuf,
        content: String,
        kind: PreviewKind,
        truncated: bool,
    },
    EditSelectedLayerBrowserFile,
    BeginLayerRelationships,
    LayerRelationshipsLoaded(LayerRelationships),
    SelectConfigVariable {
        delta: isize,
    },
    BeginSelectedConfigDetail,
    CopySelectedConfigEffective,
    CopySelectedConfigUnexpanded,
    SelectConfigSource {
        delta: isize,
    },
    OpenSelectedConfigSourceChoice,
    CancelConfigSourcePicker,
    OpenConfigScopePicker,
    SelectConfigScope {
        delta: isize,
    },
    ConfirmConfigScope,
    CancelConfigScopePicker,
    OpenConfigComparison,
    CloseConfigComparison,
    BeginConfigEdit,
    ToggleConfigEdit,
    AppendConfigEdit(char),
    BackspaceConfigEdit,
    PreviewConfigEdit,
    CancelConfigEdit,
    ConfirmConfigEdit,
    CancelConfigEditConfirmation,
    ConfigEditWriteSucceeded {
        identity: VariableIdentity,
    },
    ConfigEditWriteFailed {
        identity: VariableIdentity,
        message: String,
    },
    ConfigEditRefreshSucceeded {
        identity: VariableIdentity,
    },
    ConfigEditRefreshFailed {
        identity: VariableIdentity,
        message: String,
    },
    VariableDetailFailed {
        identity: VariableIdentity,
        message: String,
    },
    OpenSelectedConfigSource,
    BeginBbmaskEdit,
    ToggleBbmaskEdit,
    AppendBbmask(char),
    BackspaceBbmask,
    PreviewBbmaskEdit,
    CancelBbmaskEdit,
    ConfirmBbmaskWrite,
    CancelBbmaskWrite,
    BeginMetadataSearch,
    AppendMetadataQuery(char),
    BackspaceMetadataQuery,
    FinishMetadataSearch,
    Notify(String),
    ActivateNotification,
    DismissNotification,
    Quit,
    ConfirmQuit,
    CancelQuit,
    WorkspaceLoaded(Workspace),
    RecipesLoaded(Vec<Recipe>),
    LayersLoaded(Vec<Layer>),
    VariableLoaded(VariableDetail),
    RecipeSourcesLoaded {
        recipe: String,
        paths: Vec<PathBuf>,
    },
    HostTelemetryUpdated(HostTelemetry),
    Failure(AppError),
}

fn prepare_build(app: &mut App, target: Option<String>) {
    app.build.status = BuildStatus::LoadingWorkspace;
    app.build.target = target;
    app.build.started = None;
    app.build.completed = 0;
    app.build.total = None;
    app.build.parse_current = None;
    app.build.parse_total = None;
    app.build.warnings = 0;
    app.build.errors = 0;
    app.build.exit_code = None;
    app.dialogs
        .retain(|dialog| !matches!(dialog, Dialog::BuildCompletion));
    app.tasks.clear();
    app.completed_tasks.clear();
    app.task_progress_scroll = 0;
}

fn clamp_task_selection(app: &mut App) {
    app.task_progress_scroll = app
        .task_progress_scroll
        .min(app.visible_task_rows().len().saturating_sub(1));
}

fn archive_unfinished_tasks(app: &mut App, state: TaskState, cancellation: Option<&str>) {
    let finished = SystemTime::now();
    for (_, mut task) in app.tasks.drain() {
        task.state = state;
        task.finished = Some(finished);
        task.cancellation = cancellation.map(str::to_owned);
        app.completed_tasks.push_back(CompletedTask {
            task,
            success: false,
        });
    }
    while app.completed_tasks.len() > MAX_COMPLETED_TASKS {
        app.completed_tasks.pop_front();
    }
    clamp_task_selection(app);
}

fn insert_system_log(app: &mut App, severity: Severity, message: String) {
    let build = app.build.target.clone();
    app.logs.insert(LogEntry {
        id: 0,
        severity,
        message,
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::now(),
        build,
        protected: true,
        diagnostic: None,
    });
    app.error_selection = app
        .error_selection
        .min(app.logs.diagnostics().count().saturating_sub(1));
    if app.logs.follow {
        app.logs.selection = app.logs.filtered().count().saturating_sub(1);
        app.logs.scroll_offset = 0;
    }
}

pub fn format_log_details(entry: &LogEntry) -> String {
    format!(
        "Severity: {:?}\nBuild: {}\nRecipe: {}\nTask: {}\nSource: {}\n\n{}",
        entry.severity,
        entry.build.as_deref().unwrap_or("unavailable"),
        entry.recipe.as_deref().unwrap_or("unavailable"),
        entry.task.as_deref().unwrap_or("unavailable"),
        entry
            .path
            .as_ref()
            .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
        entry.message,
    )
}

fn is_pane_focus(target: FocusTarget) -> bool {
    matches!(
        target,
        FocusTarget::Navigator | FocusTarget::Workspace | FocusTarget::Inspector
    )
}

fn dialog_is_open(app: &App) -> bool {
    !app.dialogs.is_empty()
}

fn open_dialog(app: &mut App, dialog: Dialog) {
    if app.dialogs.is_empty() {
        app.dialogs.push_front(dialog);
    }
}

fn replace_dialog(app: &mut App, dialog: Dialog) {
    if let Some(active) = app.dialogs.front_mut() {
        *active = dialog;
    } else {
        app.dialogs.push_front(dialog);
    }
}

fn close_dialog(app: &mut App) {
    app.dialogs.pop_front();
}

fn enqueue_build_completion(app: &mut App) {
    if !app
        .dialogs
        .iter()
        .any(|dialog| matches!(dialog, Dialog::BuildCompletion))
    {
        app.dialogs.push_back(Dialog::BuildCompletion);
    }
}

fn modal_focus(app: &App) -> Option<FocusTarget> {
    if app.command_palette_open {
        Some(FocusTarget::CommandPalette)
    } else if dialog_is_open(app) {
        Some(FocusTarget::Dialog)
    } else {
        None
    }
}

fn synchronize_focus(app: &mut App) {
    if let Some(target) = modal_focus(app) {
        if app.focus_return.is_none() && is_pane_focus(app.focus) {
            app.focus_return = Some(app.focus);
        }
        app.focus = target;
    } else if let Some(target) = app.focus_return.take() {
        app.focus = target;
    } else if !is_pane_focus(app.focus) {
        app.focus = FocusTarget::Workspace;
    }
}

fn cycle_theme(theme: Theme, backwards: bool) -> Theme {
    const THEMES: [Theme; 8] = [
        Theme::DarkPro,
        Theme::WhiteClassic,
        Theme::MatrixGreen,
        Theme::VscodeDark,
        Theme::VscodeLight,
        Theme::AccessibleDark,
        Theme::SoftLight,
        Theme::HighContrast,
    ];
    let current = THEMES
        .iter()
        .position(|candidate| *candidate == theme)
        .unwrap_or_default();
    let next = if backwards {
        (current + THEMES.len() - 1) % THEMES.len()
    } else {
        (current + 1) % THEMES.len()
    };
    THEMES[next]
}

pub const THEMES: [Theme; 8] = [
    Theme::DarkPro,
    Theme::WhiteClassic,
    Theme::MatrixGreen,
    Theme::VscodeDark,
    Theme::VscodeLight,
    Theme::AccessibleDark,
    Theme::SoftLight,
    Theme::HighContrast,
];

fn command_action(app: &App, id: CommandId) -> Action {
    match id {
        CommandId::BuildImage => Action::OpenBuildOptions,
        CommandId::SelectImage => Action::OpenImagePicker(
            app.workspace
                .recipes
                .iter()
                .map(|recipe| recipe.name.as_str())
                .filter(|name| name.contains("image"))
                .map(str::to_owned)
                .collect(),
        ),
        CommandId::BuildSelectedRecipe => Action::BeginSelectedRecipeBuild,
        CommandId::EditBbmask => Action::BeginBbmaskEdit,
        CommandId::OpenDashboard => Action::Open(Screen::Dashboard),
        CommandId::OpenLayers => Action::Open(Screen::Layers),
        CommandId::OpenRecipes => Action::Open(Screen::Recipes),
        CommandId::OpenImages => Action::Open(Screen::Images),
        CommandId::OpenTasks => Action::Open(Screen::Tasks),
        CommandId::OpenLogs => Action::Open(Screen::Logs),
        CommandId::OpenErrors => Action::Open(Screen::Errors),
        CommandId::OpenConfiguration => Action::Open(Screen::Configuration),
        CommandId::OpenSettings => Action::Open(Screen::Settings),
        CommandId::ChooseTheme => Action::OpenThemePicker,
        CommandId::OpenHelp => Action::Open(Screen::Help),
    }
}

fn select_first_matching_layer_entry(app: &mut App) {
    let query = app.metadata_query.to_ascii_lowercase();
    if let Some(browser) = app.layer_browser.as_mut()
        && let Some(index) = browser.entries.iter().position(|entry| {
            query.is_empty()
                || entry
                    .path
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains(&query)
        })
    {
        browser.selection = index;
    }
}

fn recipe_matches_query(recipe: &Recipe, query: &str) -> bool {
    let query = query.to_ascii_lowercase();
    query.is_empty()
        || [
            Some(recipe.name.as_str()),
            recipe.version.as_deref(),
            recipe.preferred_version.as_deref(),
            recipe.layer.as_deref(),
            recipe.file.as_ref().and_then(|path| path.to_str()),
        ]
        .into_iter()
        .flatten()
        .any(|value| value.to_ascii_lowercase().contains(&query))
}

fn select_first_matching_recipe(app: &mut App) {
    if let Some(index) = app
        .workspace
        .recipes
        .iter()
        .position(|recipe| recipe_matches_query(recipe, &app.metadata_query))
    {
        app.recipe_selection = index;
    }
}

fn begin_recipe_task_for(app: &mut App, recipe_name: &str, task: Option<String>, force: bool) {
    let Some(recipe) = app
        .workspace
        .recipes
        .iter()
        .find(|recipe| recipe.name == recipe_name)
    else {
        app.notification = Some("No recipe is selected for this task.".into());
        return;
    };
    if let Some(task) = task.as_deref() {
        let Some(metadata) = app.recipe_metadata.get(&recipe.name) else {
            app.notification =
                Some("Load selected recipe metadata with Enter before choosing a task.".into());
            return;
        };
        let Some(tasks) = metadata.tasks.as_ref() else {
            app.notification =
                Some("The backend cannot report tasks for the selected recipe.".into());
            return;
        };
        let canonical = format!("do_{task}");
        if !tasks
            .iter()
            .any(|candidate| candidate == task || candidate == &canonical)
        {
            app.notification = Some(format!(
                "Task {task} is not reported for recipe {}.",
                recipe.name
            ));
            return;
        }
    }
    let request = BuildRequest {
        targets: vec![recipe.name.clone()],
        task,
        force,
    };
    if let Err(error) = request.validate() {
        app.notification = Some(error.to_string());
    } else {
        open_dialog(app, Dialog::RecipeTaskConfirmation(request));
    }
}

fn begin_recipe_task(app: &mut App, task: Option<String>, force: bool) {
    let Some(recipe_name) = app
        .workspace
        .recipes
        .get(app.recipe_selection)
        .map(|recipe| recipe.name.clone())
    else {
        app.notification = Some("No recipe is selected for this task.".into());
        return;
    };
    begin_recipe_task_for(app, &recipe_name, task, force);
}

fn selected_recipe_identity(app: &App) -> Result<RecipeIdentity, &'static str> {
    let recipe = app
        .workspace
        .recipes
        .get(app.recipe_selection)
        .ok_or("No recipe is selected for Devtool status.")?;
    let file = recipe
        .file
        .clone()
        .ok_or("The selected recipe has no authoritative provider path.")?;
    if !file.is_absolute() {
        return Err("The selected recipe provider path is not absolute.");
    }
    Ok(RecipeIdentity {
        name: recipe.name.clone(),
        file,
    })
}

fn filtered_config_identities(app: &App) -> Vec<VariableIdentity> {
    let query = app.metadata_query.to_ascii_lowercase();
    let mut identities = app
        .workspace
        .variables
        .iter()
        .filter(|(name, value)| {
            query.is_empty()
                || name.to_ascii_lowercase().contains(&query)
                || value.to_ascii_lowercase().contains(&query)
        })
        .map(|(name, _)| VariableIdentity {
            name: name.clone(),
            recipe: None,
        })
        .collect::<Vec<_>>();
    identities.sort_by(|left, right| left.name.cmp(&right.name));
    identities
}

fn selected_config_identity(app: &App) -> Option<VariableIdentity> {
    filtered_config_identities(app)
        .get(app.config_selection)
        .map(|identity| VariableIdentity {
            name: identity.name.clone(),
            recipe: app.config_scope.clone(),
        })
}

pub fn selected_config_copy_value(app: &App, value: ConfigCopyValue) -> Result<&str, String> {
    let identity = selected_config_identity(app)
        .ok_or_else(|| "No configuration variable is selected.".to_owned())?;
    if app.variable_detail_loading.contains(&identity) {
        return Err(format!(
            "Configuration detail for {} is still loading.",
            identity.name
        ));
    }
    if let Some(error) = app.variable_detail_errors.get(&identity) {
        return Err(format!(
            "Configuration detail for {} is unavailable: {error}",
            identity.name
        ));
    }
    let detail = app.variable_details.get(&identity).ok_or_else(|| {
        format!(
            "Load authoritative detail for {} with Enter before copying.",
            identity.name
        )
    })?;
    match value {
        ConfigCopyValue::Effective => detail
            .effective_value
            .as_deref()
            .ok_or_else(|| format!("The effective value for {} is unavailable.", identity.name)),
        ConfigCopyValue::Unexpanded => detail
            .unexpanded_value
            .as_deref()
            .ok_or_else(|| format!("The unexpanded value for {} is unavailable.", identity.name)),
    }
}

fn selected_config_sources(
    app: &App,
) -> Result<(VariableIdentity, Vec<ConfigSourceChoice>), String> {
    let identity = selected_config_identity(app)
        .ok_or_else(|| "No configuration variable is selected.".to_owned())?;
    if app.variable_detail_loading.contains(&identity) {
        return Err(format!(
            "Configuration detail for {} is still loading.",
            identity.name
        ));
    }
    if let Some(error) = app.variable_detail_errors.get(&identity) {
        return Err(format!(
            "Configuration detail for {} is unavailable: {error}",
            identity.name
        ));
    }
    let detail = app.variable_details.get(&identity).ok_or_else(|| {
        format!(
            "Load authoritative detail for {} with Enter before opening a source.",
            identity.name
        )
    })?;
    let mut seen = HashSet::new();
    let sources = detail
        .operations
        .iter()
        .filter_map(|operation| {
            let path = operation.file.clone()?;
            (!path.as_os_str().is_empty() && seen.insert((path.clone(), operation.line))).then(
                || ConfigSourceChoice {
                    operation: operation.operation.clone(),
                    path,
                    line: operation.line,
                },
            )
        })
        .collect::<Vec<_>>();
    if sources.is_empty() {
        return Err(format!(
            "No file-backed defining operation is available for {}.",
            identity.name
        ));
    }
    Ok((identity, sources))
}

pub fn config_source_disabled_reason(app: &App) -> Option<String> {
    selected_config_sources(app).err()
}

fn comparison_field(global: Option<String>, recipe: Option<String>) -> ConfigComparisonField {
    let outcome = match (&global, &recipe) {
        (Some(global), Some(recipe)) if global == recipe => ConfigComparisonOutcome::Equal,
        (Some(_), Some(_)) => ConfigComparisonOutcome::Different,
        _ => ConfigComparisonOutcome::Unavailable,
    };
    ConfigComparisonField {
        global,
        recipe,
        outcome,
    }
}

pub fn config_comparison(app: &App) -> Result<ConfigComparison, String> {
    let selected = selected_config_identity(app)
        .ok_or_else(|| "No configuration variable is selected.".to_owned())?;
    let recipe = app
        .config_scope
        .clone()
        .ok_or_else(|| "Select a recipe scope with s before comparing.".to_owned())?;
    if !app
        .workspace
        .recipes
        .iter()
        .any(|candidate| candidate.name == recipe)
    {
        return Err(format!("Recipe scope {recipe} is no longer available."));
    }
    let global_identity = VariableIdentity {
        name: selected.name.clone(),
        recipe: None,
    };
    let recipe_identity = VariableIdentity {
        name: selected.name.clone(),
        recipe: Some(recipe.clone()),
    };
    for identity in [&global_identity, &recipe_identity] {
        let scope = identity.recipe.as_deref().unwrap_or("global");
        if app.variable_detail_loading.contains(identity) {
            return Err(format!("Detail for {scope} scope is still loading."));
        }
        if let Some(error) = app.variable_detail_errors.get(identity) {
            return Err(format!("Detail for {scope} scope is unavailable: {error}"));
        }
    }
    let global = app
        .variable_details
        .get(&global_identity)
        .ok_or_else(|| format!("Load global detail for {} before comparing.", selected.name))?;
    let scoped = app.variable_details.get(&recipe_identity).ok_or_else(|| {
        format!(
            "Load {recipe} detail for {} before comparing.",
            selected.name
        )
    })?;
    Ok(ConfigComparison {
        variable: selected.name,
        recipe,
        effective: comparison_field(
            global.effective_value.clone(),
            scoped.effective_value.clone(),
        ),
        unexpanded: comparison_field(
            global.unexpanded_value.clone(),
            scoped.unexpanded_value.clone(),
        ),
    })
}

pub const EDITABLE_CONFIG_VARIABLES: &[&str] = &["DISTRO", "MACHINE"];

fn config_edit_context(app: &App) -> Result<(VariableIdentity, String, PathBuf), String> {
    if app.config_scope.is_some() {
        return Err("Recipe-scoped configuration is read-only; select global scope.".into());
    }
    let identity = selected_config_identity(app)
        .ok_or_else(|| "No configuration variable is selected.".to_owned())?;
    if !EDITABLE_CONFIG_VARIABLES.contains(&identity.name.as_str()) {
        return Err(format!(
            "{} is read-only; editable variables are {}.",
            identity.name,
            EDITABLE_CONFIG_VARIABLES.join(", ")
        ));
    }
    if app.variable_detail_loading.contains(&identity) {
        return Err(format!(
            "Configuration detail for {} is still loading.",
            identity.name
        ));
    }
    if let Some(error) = app.variable_detail_errors.get(&identity) {
        return Err(format!(
            "Configuration detail for {} is unavailable: {error}",
            identity.name
        ));
    }
    let detail = app.variable_details.get(&identity).ok_or_else(|| {
        format!(
            "Load authoritative detail for {} with Enter before editing.",
            identity.name
        )
    })?;
    let value = detail
        .effective_value
        .clone()
        .ok_or_else(|| format!("The effective value for {} is unavailable.", identity.name))?;
    let destination = app
        .workspace
        .build_dir
        .as_ref()
        .map(|build_dir| build_dir.join("conf/local.conf"))
        .ok_or_else(|| {
            "An active build directory is required for configuration editing.".to_owned()
        })?;
    Ok((identity, value, destination))
}

pub fn config_edit_disabled_reason(app: &App) -> Option<String> {
    config_edit_context(app).err()
}

pub fn config_edit_assignment(name: &str, value: &str) -> Result<String, String> {
    if value.chars().any(char::is_control) {
        return Err("Configuration values cannot contain newlines or control characters.".into());
    }
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!("{name} = \"{escaped}\""))
}

pub(crate) fn popup_toml_value(content: &str, key: &str) -> Result<String, String> {
    let mut value = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, raw_value)) = line.split_once('=') else {
            return Err(format!("Expected `{key} = \"value\"`."));
        };
        if name.trim() != key || value.is_some() {
            return Err(format!("Only one `{key} = \"value\"` entry is allowed."));
        }
        let raw_value = raw_value.trim();
        if !(raw_value.starts_with('\"') && raw_value.ends_with('\"')) || raw_value.len() < 2 {
            return Err(format!("`{key}` must be a quoted TOML string."));
        }
        let unescaped = raw_value[1..raw_value.len() - 1]
            .replace("\\\\", "\\")
            .replace("\\\"", "\"");
        value = Some(unescaped);
    }
    value.ok_or_else(|| format!("Missing `{key} = \"value\"` entry."))
}

pub(crate) fn popup_toml_document(key: &str, value: &str, comment: Option<&str>) -> String {
    let mut document = comment.map_or_else(String::new, |comment| format!("# {comment}\n"));
    document.push_str(&format!(
        "{key} = \"{}\"\n",
        value.replace('\\', "\\\\").replace('\"', "\\\"")
    ));
    document
}

pub(crate) fn popup_toml_fields(content: &str) -> Result<HashMap<String, String>, String> {
    let mut fields = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, raw_value)) = line.split_once('=') else {
            return Err("Expected TOML `key = value` entries.".into());
        };
        let key = key.trim();
        if key.is_empty() || fields.contains_key(key) {
            return Err("TOML keys must be nonempty and occur only once.".into());
        }
        let raw_value = raw_value.trim();
        let value =
            if raw_value.starts_with('\"') && raw_value.ends_with('\"') && raw_value.len() >= 2 {
                raw_value[1..raw_value.len() - 1]
                    .replace("\\\\", "\\")
                    .replace("\\\"", "\"")
            } else {
                raw_value.to_owned()
            };
        fields.insert(key.to_owned(), value);
    }
    Ok(fields)
}

pub fn validate_config_edit_request(
    request: &ConfigEditRequest,
    build_dir: &Path,
) -> Result<(), String> {
    if request.identity.recipe.is_some() {
        return Err("Recipe-scoped configuration edits are not allowed.".into());
    }
    if !EDITABLE_CONFIG_VARIABLES.contains(&request.identity.name.as_str()) {
        return Err(format!(
            "{} is read-only; editable variables are {}.",
            request.identity.name,
            EDITABLE_CONFIG_VARIABLES.join(", ")
        ));
    }
    let expected_destination = build_dir.join("conf/local.conf");
    if request.destination != expected_destination {
        return Err(format!(
            "Configuration edit destination must be {}.",
            expected_destination.display()
        ));
    }
    let expected_assignment = config_edit_assignment(&request.identity.name, &request.value)?;
    if request.assignment != expected_assignment {
        return Err("Configuration edit assignment does not match the confirmed value.".into());
    }
    Ok(())
}

fn resolve_config_source(app: &App, path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "Relative configuration source escapes the build directory: {}.",
            path.display()
        ));
    }
    app.workspace
        .build_dir
        .as_ref()
        .map(|build_dir| build_dir.join(path))
        .ok_or_else(|| {
            format!(
                "Cannot resolve relative configuration source {} without an active build directory.",
                path.display()
            )
    })
}

fn set_dependency_graph(app: &mut App, graph: DependencyGraph, limitations: Option<Vec<String>>) {
    let previous = app.dependency_graph_selection.take();
    app.dependency_graph_selection = previous
        .filter(|selected| graph.contains(selected))
        .or_else(|| graph.contains(&graph.root).then(|| graph.root.clone()))
        .or_else(|| graph.nodes.first().map(|node| node.id.clone()));
    app.dependencies = Some(RecipeDependencies {
        recipe: graph.root.recipe_name().to_owned(),
        build: graph
            .edges
            .iter()
            .filter_map(|edge| {
                if edge.from == graph.root && edge.kind == DependencyEdgeKind::Build {
                    match &edge.to {
                        DependencyNodeId::Recipe(recipe) => Some(recipe.clone()),
                        DependencyNodeId::Task { .. } => None,
                    }
                } else {
                    None
                }
            })
            .collect(),
        runtime: graph
            .edges
            .iter()
            .filter_map(|edge| {
                if edge.from == graph.root && edge.kind == DependencyEdgeKind::Runtime {
                    match &edge.to {
                        DependencyNodeId::Recipe(recipe) => Some(recipe.clone()),
                        DependencyNodeId::Task { .. } => None,
                    }
                } else {
                    None
                }
            })
            .collect(),
    });
    app.dependency_selection = 0;
    app.screen = Screen::Dependencies;
    app.dependency_graph = if let Some(limitations) = limitations {
        DependencyGraphState::Partial { graph, limitations }
    } else if graph.edges.is_empty() {
        DependencyGraphState::AvailableEmpty { root: graph.root }
    } else {
        DependencyGraphState::Available(graph)
    };
}

fn signature_comparison_inputs(
    state: &SignatureComparisonState,
) -> (Option<SignatureIdentity>, Option<SignatureIdentity>) {
    match state {
        SignatureComparisonState::NotSelected => (None, None),
        SignatureComparisonState::Ready { left, right } => (left.clone(), right.clone()),
        SignatureComparisonState::Loading { request }
        | SignatureComparisonState::AvailableEmpty { request }
        | SignatureComparisonState::Available { request, .. }
        | SignatureComparisonState::Partial { request, .. }
        | SignatureComparisonState::Failed { request, .. } => {
            (Some(request.left.clone()), Some(request.right.clone()))
        }
    }
}

fn set_signature_dump(
    app: &mut App,
    target: SignatureTarget,
    records: Vec<SignatureRecord>,
    limitations: Option<Vec<String>>,
) {
    let (records, report) = normalize_signature_records(&target, records, MAX_SIGNATURE_RECORDS);
    let identities = records
        .iter()
        .map(|record| record.identity.clone())
        .collect::<BTreeSet<_>>();
    app.signature_selection = app
        .signature_selection
        .take()
        .filter(|selected| identities.contains(selected))
        .or_else(|| records.first().map(|record| record.identity.clone()));

    let (left, right) = signature_comparison_inputs(&app.signature_comparison);
    app.signature_comparison = SignatureComparisonState::Ready {
        left: left.filter(|identity| identities.contains(identity)),
        right: right.filter(|identity| identities.contains(identity)),
    };

    let mut limitations = limitations.unwrap_or_default();
    if report.is_partial() {
        limitations.push(format!(
            "Model bounds rejected {} invalid and truncated {} signature records.",
            report.invalid_records, report.truncated_records
        ));
    }
    app.signature_dump = if records.is_empty() && limitations.is_empty() {
        SignatureDumpState::AvailableEmpty { target }
    } else if limitations.is_empty() {
        SignatureDumpState::Available { target, records }
    } else {
        SignatureDumpState::Partial {
            target,
            records,
            limitations,
        }
    };
}

fn begin_signature_dump(app: &mut App, target: SignatureTarget) -> Option<Effect> {
    if let Err(message) = target.validate() {
        app.notification = Some(message.into());
        return None;
    }
    app.signature_dump = SignatureDumpState::Loading {
        target: target.clone(),
    };
    Some(Effect::GetSignatureDump(target))
}

fn signature_operation_is_loading(app: &App) -> bool {
    matches!(app.signature_dump, SignatureDumpState::Loading { .. })
        || matches!(
            app.signature_comparison,
            SignatureComparisonState::Loading { .. }
        )
}

fn next_image_artifact_generation(app: &mut App) -> u64 {
    app.image_artifact_request_generation = app.image_artifact_request_generation.wrapping_add(1);
    if app.image_artifact_request_generation == 0 {
        app.image_artifact_request_generation = 1;
    }
    app.image_artifact_request_generation
}

fn begin_image_artifact_inventory(app: &mut App) -> Option<Effect> {
    let machine = app
        .workspace
        .variables
        .get("MACHINE")
        .cloned()
        .unwrap_or_default();
    let request = ImageArtifactRequest {
        generation: next_image_artifact_generation(app),
        machine,
    };
    if let Err(message) = request.validate() {
        app.notification = Some(format!("Image artifacts are unavailable: {message}."));
        return None;
    }
    app.image_artifacts = ImageArtifactInventoryState::Loading {
        request: request.clone(),
    };
    Some(Effect::GetImageArtifacts(request))
}

fn image_artifact_operation_is_loading(app: &App) -> bool {
    matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::Loading { .. }
    )
}

fn begin_sdk_artifact_inventory(app: &mut App) -> Option<Effect> {
    let Some(root) = app.workspace.variables.get("SDK_DEPLOY").map(PathBuf::from) else {
        app.notification =
            Some("SDK artifacts are unavailable because SDK_DEPLOY was not reported.".into());
        return None;
    };
    let machine = app
        .workspace
        .variables
        .get("MACHINE")
        .cloned()
        .unwrap_or_default();
    app.sdk_artifact_generation = app.sdk_artifact_generation.wrapping_add(1).max(1);
    let request = SdkArtifactInventoryRequest {
        generation: app.sdk_artifact_generation,
        root,
        machine,
    };
    if let Err(message) = request.validate() {
        app.notification = Some(format!("SDK artifacts are unavailable: {message}."));
        return None;
    }
    app.sdk_artifacts = SdkArtifactInventoryState::Loading {
        request: request.clone(),
    };
    Some(Effect::GetSdkArtifacts(request))
}

fn set_sdk_artifact_selection_to_current_or_first(
    app: &mut App,
    previous: Option<SdkArtifactIdentity>,
) {
    let visible = app
        .filtered_sdk_artifacts()
        .into_iter()
        .map(|artifact| artifact.identity.clone())
        .collect::<Vec<_>>();
    app.sdk_artifact_selection = previous
        .filter(|identity| visible.contains(identity))
        .or_else(|| visible.first().cloned());
}

const MAX_SDK_SESSIONS: usize = 32;
const SDK_BACKGROUND_JOB_NAMESPACE: u64 = 1 << 62;

fn next_sdk_session_id(app: &mut App) -> SdkSessionId {
    app.sdk_session_generation = app.sdk_session_generation.wrapping_add(1).max(1);
    SdkSessionId(app.sdk_session_generation)
}

fn sdk_background_job_id(id: SdkSessionId) -> BackgroundJobId {
    BackgroundJobId(SDK_BACKGROUND_JOB_NAMESPACE | id.0)
}

fn sdk_job_id(app: &App, id: SdkSessionId) -> Option<BackgroundJobId> {
    app.sdk_session(id).map(|session| session.background_job_id)
}

fn mutate_sdk_session(
    app: &mut App,
    id: SdkSessionId,
    mutation: impl FnOnce(&mut SdkSession),
) -> Option<BackgroundJobId> {
    let session = app
        .sdk_sessions
        .iter_mut()
        .find(|session| session.id == id)?;
    let job_id = session.background_job_id;
    mutation(session);
    Some(job_id)
}

fn note_stale_sdk_event(app: &mut App) {
    app.background_jobs.ignored_transitions += 1;
}

fn queue_sdk_session(app: &mut App, operation: SdkOperation) -> Option<Effect> {
    if app.active_sdk_session().is_some() {
        app.notification = Some("A managed SDK tool operation is already active.".into());
        return None;
    }
    while app.sdk_sessions.len() >= MAX_SDK_SESSIONS {
        let Some(index) = app.sdk_sessions.iter().position(|session| {
            app.background_jobs
                .get(session.background_job_id)
                .is_none_or(|job| job.status.is_terminal())
        }) else {
            app.notification = Some("The SDK operation history is full.".into());
            return None;
        };
        app.sdk_sessions.remove(index);
    }
    let id = next_sdk_session_id(app);
    let background_job_id = sdk_background_job_id(id);
    let (title, path) = match &operation {
        SdkOperation::Publish(request) => (
            format!("Publish SDK {}", request.artifact.path.display()),
            Some(request.artifact.path.clone()),
        ),
        SdkOperation::Native(request) => (
            format!("SDK native {}", request.recipe),
            request.extracted_root.clone(),
        ),
    };
    app.background_jobs.queue(BackgroundJobSpec {
        id: background_job_id,
        kind: BackgroundJobKind::Sdk,
        title,
        context: BackgroundJobContext {
            workspace: Some(Screen::Sdk),
            path,
            ..BackgroundJobContext::default()
        },
        cancellation_supported: true,
        queued_at: SystemTime::now(),
    });
    if app.background_jobs.get(background_job_id).is_none() {
        app.notification = Some("The SDK operation could not be queued.".into());
        return None;
    }
    app.sdk_sessions.push_back(SdkSession {
        id,
        background_job_id,
        operation: operation.clone(),
        exit_code: None,
        error_detail: None,
    });
    Some(Effect::StartSdkSession { id, operation })
}

const MAX_TEST_SESSIONS: usize = 32;
const TEST_BACKGROUND_JOB_NAMESPACE: u64 = 3 << 60;

fn next_test_session_id(app: &mut App) -> TestSessionId {
    app.test_session_generation = app.test_session_generation.wrapping_add(1).max(1);
    TestSessionId(app.test_session_generation)
}

fn test_background_job_id(id: TestSessionId) -> BackgroundJobId {
    BackgroundJobId(TEST_BACKGROUND_JOB_NAMESPACE | id.0)
}

fn test_job_id(app: &App, id: TestSessionId) -> Option<BackgroundJobId> {
    app.test_session(id)
        .and_then(|session| session.background_job_id)
}

fn mutate_test_session(
    app: &mut App,
    id: TestSessionId,
    mutation: impl FnOnce(&mut TestSession),
) -> Option<Option<BackgroundJobId>> {
    let session = app
        .test_sessions
        .iter_mut()
        .find(|session| session.id == id)?;
    mutation(session);
    Some(session.background_job_id)
}

fn note_stale_test_event(app: &mut App) {
    app.background_jobs.ignored_transitions += 1;
}

fn test_launch_draft(app: &App, family: TestFamily) -> TestLaunchDraft {
    TestLaunchDraft::new(
        family,
        app.workspace
            .variables
            .get("MACHINE")
            .cloned()
            .unwrap_or_default(),
        app.workspace
            .variables
            .get("DISTRO")
            .cloned()
            .unwrap_or_default(),
        app.build.target.clone().unwrap_or_default(),
    )
}

fn test_preview_is_current(app: &App, preview: &TestLaunchPreview) -> bool {
    match preview {
        TestLaunchPreview::Selftest(request) => {
            TestSelftestRequest::new(
                request.executable.clone(),
                request.family,
                request.selector.clone(),
                request.parallelism,
                request.verbose,
                request.skip_network,
            )
            .as_ref()
                == Ok(request)
                && app.test_capability.executable_for(request.family).as_ref()
                    == Ok(&request.executable)
        }
        TestLaunchPreview::Build {
            family, request, ..
        } => {
            test_launch_draft(app, *family)
                .preview(&app.test_capability)
                .as_ref()
                .is_ok_and(|current| current == preview)
                && request.validate().is_ok()
        }
    }
}

fn queue_test_session(app: &mut App, operation: TestOperation) -> Option<Effect> {
    if app.active_test_session().is_some() {
        app.notification = Some("A managed Testing operation is already active.".into());
        return None;
    }
    while app.test_sessions.len() >= MAX_TEST_SESSIONS {
        let Some(index) = app.test_sessions.iter().position(|session| {
            session.background_job_id.is_some_and(|job_id| {
                app.background_jobs
                    .get(job_id)
                    .is_none_or(|job| job.status.is_terminal())
            })
        }) else {
            app.notification = Some("The Testing session history is full.".into());
            return None;
        };
        app.test_sessions.remove(index);
    }
    let id = next_test_session_id(app);
    let background_job_id =
        matches!(&operation, TestOperation::Selftest(_)).then(|| test_background_job_id(id));
    if let Some(job_id) = background_job_id {
        let family = operation.family();
        app.background_jobs.queue(BackgroundJobSpec {
            id: job_id,
            kind: BackgroundJobKind::Test,
            title: family.label().into(),
            context: BackgroundJobContext {
                workspace: Some(Screen::Testing),
                target: match &operation {
                    TestOperation::Selftest(request) => request.selector.clone(),
                    TestOperation::Build { request, .. } => request.targets.first().cloned(),
                },
                task: family.task().map(str::to_owned),
                image: match &operation {
                    TestOperation::Build { request, .. } => request.targets.first().cloned(),
                    TestOperation::Selftest(_) => None,
                },
                ..BackgroundJobContext::default()
            },
            cancellation_supported: true,
            queued_at: SystemTime::now(),
        });
        if app.background_jobs.get(job_id).is_none() {
            app.notification = Some("The Testing operation could not be queued.".into());
            return None;
        }
    }
    app.test_sessions.push_back(TestSession {
        id,
        background_job_id,
        operation: operation.clone(),
        exit_code: None,
        result_paths: Vec::new(),
        error_detail: None,
        outcome: None,
    });
    match operation {
        TestOperation::Selftest(_) => Some(Effect::StartTestSession { id, operation }),
        TestOperation::Build { request, .. } => Some(Effect::StartTestBuildSession { id, request }),
    }
}

fn next_test_result_generation(app: &mut App) -> u64 {
    app.test_result_generation = app.test_result_generation.wrapping_add(1).max(1);
    app.test_result_generation
}

fn begin_test_result_import(app: &mut App, roots: Vec<PathBuf>) -> Option<Effect> {
    let generation = next_test_result_generation(app);
    let request = match TestResultImportRequest::new(generation, roots) {
        Ok(request) => request,
        Err(message) => {
            app.notification = Some(format!("Test results are unavailable: {message}."));
            return None;
        }
    };
    app.test_results = TestResultInventoryState::Loading {
        request: request.clone(),
    };
    Some(Effect::ImportTestResults(request))
}

fn test_result_request_is_current(app: &App, request: &TestResultImportRequest) -> bool {
    matches!(
        &app.test_results,
        TestResultInventoryState::Loading { request: current } if current == request
    )
}

fn set_test_result_selection_to_current_or_first(
    app: &mut App,
    previous: Option<TestResultIdentity>,
) {
    let visible = app
        .filtered_test_results()
        .into_iter()
        .map(|record| record.identity.clone())
        .collect::<Vec<_>>();
    app.test_result_selection = previous
        .filter(|identity| visible.contains(identity))
        .or_else(|| visible.first().cloned());
    if app.test_case_selection.as_ref().is_some_and(|identity| {
        app.selected_test_result()
            .and_then(|record| record.case(identity))
            .is_none()
    }) {
        app.test_case_selection = None;
        app.test_result_drilled = false;
    }
}

fn test_comparison_inputs_exist(app: &App, request: &TestComparisonRequest) -> bool {
    app.test_results
        .records()
        .iter()
        .any(|record| record.identity == request.baseline)
        && app
            .test_results
            .records()
            .iter()
            .any(|record| record.identity == request.candidate)
}

fn test_comparison_request_is_current(app: &App, request: &TestComparisonRequest) -> bool {
    matches!(
        &app.test_comparison,
        TestComparisonState::Loading { request: current } if current == request
    ) && test_comparison_inputs_exist(app, request)
}

fn set_test_comparison_selection(app: &mut App) {
    let identities = app
        .test_comparison_transitions()
        .iter()
        .map(|transition| transition.identity.clone())
        .collect::<Vec<_>>();
    app.test_comparison_selection = app
        .test_comparison_selection
        .take()
        .filter(|identity| identities.contains(identity))
        .or_else(|| identities.first().cloned());
}

fn test_junit_request_is_current(app: &App, request: &TestJunitExportRequest) -> bool {
    app.test_results
        .records()
        .iter()
        .any(|record| record.identity == request.result)
        && match &app.test_junit_export {
            TestJunitExportState::Running(current) => current == request,
            _ => false,
        }
}

const MAX_QEMU_SESSIONS: usize = 32;
const QEMU_BACKGROUND_JOB_NAMESPACE: u64 = 3 << 62;

fn next_qemu_session_id(app: &mut App) -> QemuSessionId {
    app.qemu_session_generation = app.qemu_session_generation.wrapping_add(1);
    if app.qemu_session_generation == 0 {
        app.qemu_session_generation = 1;
    }
    QemuSessionId(app.qemu_session_generation)
}

fn qemu_background_job_id(id: QemuSessionId) -> BackgroundJobId {
    BackgroundJobId(QEMU_BACKGROUND_JOB_NAMESPACE | id.0)
}

fn qemu_job_id(app: &App, id: QemuSessionId) -> Option<BackgroundJobId> {
    app.qemu_session(id)
        .map(|session| session.background_job_id)
}

fn mutate_qemu_session(
    app: &mut App,
    id: QemuSessionId,
    mutation: impl FnOnce(&mut QemuSession),
) -> Option<BackgroundJobId> {
    let session = app
        .qemu_sessions
        .iter_mut()
        .find(|session| session.id == id)?;
    let job_id = session.background_job_id;
    mutation(session);
    Some(job_id)
}

fn note_stale_qemu_event(app: &mut App) {
    app.background_jobs.ignored_transitions += 1;
}

const MAX_WIC_SESSIONS: usize = 32;
const WIC_BACKGROUND_JOB_NAMESPACE: u64 = 2 << 62;

fn next_wic_session_id(app: &mut App) -> WicSessionId {
    app.wic_session_generation = app.wic_session_generation.wrapping_add(1);
    if app.wic_session_generation == 0 {
        app.wic_session_generation = 1;
    }
    WicSessionId(app.wic_session_generation)
}

fn wic_background_job_id(id: WicSessionId) -> BackgroundJobId {
    BackgroundJobId(WIC_BACKGROUND_JOB_NAMESPACE | id.0)
}

fn wic_job_id(app: &App, id: WicSessionId) -> Option<BackgroundJobId> {
    app.wic_session(id).map(|session| session.background_job_id)
}

fn mutate_wic_session(
    app: &mut App,
    id: WicSessionId,
    mutation: impl FnOnce(&mut WicSession),
) -> Option<BackgroundJobId> {
    let session = app
        .wic_sessions
        .iter_mut()
        .find(|session| session.id == id)?;
    let job_id = session.background_job_id;
    mutation(session);
    Some(job_id)
}

fn note_stale_wic_event(app: &mut App) {
    app.background_jobs.ignored_transitions += 1;
}

fn is_uncompressed_wic_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".wic") || name.ends_with(".direct"))
}

fn reconcile_wic_output_selection(app: &mut App) {
    let rows = app.wic_output_rows();
    let selected_is_present = app
        .wic_output_selection
        .as_ref()
        .is_some_and(|selected| rows.iter().any(|row| &row.identity == selected));
    if !selected_is_present {
        app.wic_output_selection = rows.first().map(|row| row.identity.clone());
    }
}

fn reconcile_wic_device_selection(app: &mut App) {
    let rows = app.wic_device_rows();
    let selected_is_present = app
        .wic_device_selection
        .as_ref()
        .is_some_and(|selected| rows.iter().any(|row| &row.identity == selected));
    if !selected_is_present {
        app.wic_device_selection = rows.first().map(|row| row.identity.clone());
    }
}

fn current_wic_write_preview(
    app: &App,
    request: &WicDeviceInventoryRequest,
    device_identity: &WicDeviceIdentity,
    phrase: &str,
) -> Result<WicWritePreview, String> {
    let (active_request, devices) = match &app.wic_devices {
        WicDeviceInventoryState::Available {
            request, devices, ..
        }
        | WicDeviceInventoryState::Partial {
            request, devices, ..
        } => (request, devices),
        _ => return Err("The Wic device inventory is unavailable.".into()),
    };
    if active_request != request {
        return Err("The Wic device inventory request is stale.".into());
    }
    let current_image = app.selected_wic_write_image()?;
    if current_image != request.image {
        return Err("The selected Wic image identity changed.".into());
    }
    let device = devices
        .iter()
        .find(|device| &device.identity == device_identity)
        .ok_or_else(|| "The selected Wic device identity is stale.".to_owned())?;
    WicWritePreview::new(&app.wic_capability, request.image.clone(), device, phrase)
        .map_err(str::to_owned)
}

fn queue_wic_session(app: &mut App, operation: WicOperation) -> Option<Effect> {
    if app.active_wic_session().is_some() {
        app.notification = Some("A managed Wic operation is already active.".into());
        return None;
    }
    while app.wic_sessions.len() >= MAX_WIC_SESSIONS {
        let Some(index) = app.wic_sessions.iter().position(|session| {
            app.background_jobs
                .get(session.background_job_id)
                .is_none_or(|job| job.status.is_terminal())
        }) else {
            app.notification = Some("The Wic operation history is full.".into());
            return None;
        };
        app.wic_sessions.remove(index);
    }
    let id = next_wic_session_id(app);
    let background_job_id = wic_background_job_id(id);
    let (title, target, path) = match &operation {
        WicOperation::Create(request) => (
            format!("wic create {}", request.image),
            Some(request.image.clone()),
            Some(request.output_directory.clone()),
        ),
        WicOperation::Write(request) => (
            format!("wic write {}", request.device.path.display()),
            None,
            Some(request.device.path.clone()),
        ),
    };
    app.background_jobs.queue(BackgroundJobSpec {
        id: background_job_id,
        kind: BackgroundJobKind::Wic,
        title,
        context: BackgroundJobContext {
            workspace: Some(Screen::Images),
            target,
            path,
            ..BackgroundJobContext::default()
        },
        cancellation_supported: true,
        queued_at: SystemTime::now(),
    });
    if app.background_jobs.get(background_job_id).is_none() {
        app.notification = Some("The Wic operation could not be queued.".into());
        return None;
    }
    app.wic_sessions.push_back(WicSession {
        id,
        background_job_id,
        operation: operation.clone(),
        exit_code: None,
        error_detail: None,
    });
    Some(Effect::StartWicSession { id, operation })
}

fn normalize_image_artifact_limitations(mut limitations: Vec<String>) -> Vec<String> {
    limitations = limitations
        .into_iter()
        .filter(|limitation| !limitation.is_empty() && !limitation.chars().any(char::is_control))
        .map(|limitation| limitation.chars().take(2_048).collect())
        .collect();
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_IMAGE_ARTIFACT_LIMITATIONS);
    limitations
}

fn append_image_artifact_normalization_limitations(
    limitations: &mut Vec<String>,
    report: &ImageArtifactNormalizationReport,
) {
    if report.invalid_records > 0 || report.invalid_fields > 0 {
        limitations.push(format!(
            "Model validation dropped {} image artifact record(s) and {} field value(s).",
            report.invalid_records, report.invalid_fields
        ));
    }
    if report.truncated_records > 0
        || report.truncated_associated_files > 0
        || report.truncated_checksums > 0
    {
        limitations.push(format!(
            "Model bounds truncated {} image artifact(s), {} associated file(s), and {} checksum record(s).",
            report.truncated_records,
            report.truncated_associated_files,
            report.truncated_checksums
        ));
    }
}

fn set_image_artifact_selection_to_current_or_first(
    app: &mut App,
    previous: Option<ImageArtifactIdentity>,
) {
    let visible = app
        .filtered_image_artifacts()
        .into_iter()
        .map(|artifact| artifact.identity.clone())
        .collect::<Vec<_>>();
    app.image_artifact_selection = previous
        .filter(|identity| visible.contains(identity))
        .or_else(|| visible.first().cloned());
}

fn set_image_artifact_inventory(
    app: &mut App,
    request: ImageArtifactRequest,
    inventory: ImageArtifactInventory,
    limitations: Option<Vec<String>>,
) {
    let previous = app.image_artifact_selection.take();
    let (inventory, report) =
        normalize_image_artifact_inventory(&request, inventory, MAX_IMAGE_ARTIFACT_RECORDS);
    let Some(inventory) = inventory else {
        app.image_artifacts = ImageArtifactInventoryState::Failed {
            request,
            message: "backend returned image artifacts for a different or invalid machine".into(),
        };
        app.notification =
            Some("Image artifact inventory failed model identity validation.".into());
        return;
    };
    let mut limitations = limitations.unwrap_or_default();
    append_image_artifact_normalization_limitations(&mut limitations, &report);
    let limitations = normalize_image_artifact_limitations(limitations);
    app.image_artifacts = if inventory.artifacts.is_empty() && limitations.is_empty() {
        ImageArtifactInventoryState::AvailableEmpty { request, inventory }
    } else if limitations.is_empty() {
        ImageArtifactInventoryState::Available { request, inventory }
    } else {
        ImageArtifactInventoryState::Partial {
            request,
            inventory,
            limitations,
        }
    };
    set_image_artifact_selection_to_current_or_first(app, previous);
}

fn next_package_generation(app: &mut App) -> u64 {
    app.package_request_generation = app.package_request_generation.wrapping_add(1);
    if app.package_request_generation == 0 {
        app.package_request_generation = 1;
    }
    app.package_request_generation
}

fn normalize_package_limitations(mut limitations: Vec<String>) -> Vec<String> {
    limitations.retain(|limitation| !limitation.is_empty());
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_PACKAGE_LIMITATIONS);
    limitations
}

fn append_package_normalization_limitations(
    limitations: &mut Vec<String>,
    report: &PackageNormalizationReport,
) {
    if report.invalid_records > 0 || report.invalid_fields > 0 {
        limitations.push(format!(
            "Model validation dropped {} package record(s) and {} field value(s).",
            report.invalid_records, report.invalid_fields
        ));
    }
    if report.truncated_records > 0
        || report.truncated_files > 0
        || report.truncated_dependencies > 0
        || report.truncated_image_memberships > 0
    {
        limitations.push(format!(
            "Model bounds truncated {} package(s), {} file(s), {} dependency value(s), and {} image membership value(s).",
            report.truncated_records,
            report.truncated_files,
            report.truncated_dependencies,
            report.truncated_image_memberships
        ));
    }
}

fn set_package_selection_to_current_or_first(app: &mut App, previous: Option<PackageIdentity>) {
    let visible = app
        .filtered_packages()
        .into_iter()
        .map(|package| package.identity.clone())
        .collect::<Vec<_>>();
    app.package_selection = previous
        .filter(|identity| visible.contains(identity))
        .or_else(|| visible.first().cloned());
}

fn set_package_inventory(
    app: &mut App,
    request: PackageInventoryRequest,
    packages: Vec<PackageSummary>,
    limitations: Option<Vec<String>>,
) {
    let previous = app.package_selection.take();
    let (packages, report) = normalize_package_summaries(packages, MAX_PACKAGE_RECORDS);
    let identities = packages
        .iter()
        .map(|package| package.identity.clone())
        .collect::<BTreeSet<_>>();
    app.package_details
        .retain(|identity, _| identities.contains(identity));
    let mut limitations = limitations.unwrap_or_default();
    append_package_normalization_limitations(&mut limitations, &report);
    let limitations = normalize_package_limitations(limitations);
    app.package_inventory = if packages.is_empty() && limitations.is_empty() {
        PackageInventoryState::AvailableEmpty { request }
    } else if limitations.is_empty() {
        PackageInventoryState::Available { request, packages }
    } else {
        PackageInventoryState::Partial {
            request,
            packages,
            limitations,
        }
    };
    set_package_selection_to_current_or_first(app, previous);
}

fn package_detail_is_empty(detail: &PackageDetail) -> bool {
    matches!(&detail.files, PackageField::Available(files) if files.is_empty())
        && matches!(
            &detail.runtime_dependencies,
            PackageField::Available(dependencies) if dependencies.is_empty()
        )
        && matches!(
            &detail.reverse_dependencies,
            PackageField::Available(dependencies) if dependencies.is_empty()
        )
}

fn begin_package_inventory(app: &mut App) -> Effect {
    let request = PackageInventoryRequest {
        generation: next_package_generation(app),
    };
    app.package_inventory = PackageInventoryState::Loading { request };
    Effect::GetPackageInventory(request)
}

fn package_operation_is_loading(app: &App) -> bool {
    matches!(app.package_inventory, PackageInventoryState::Loading { .. })
        || app
            .package_details
            .values()
            .any(|state| matches!(state, PackageDetailState::Loading { .. }))
}

fn begin_package_detail(app: &mut App, identity: PackageIdentity) -> Effect {
    let request = PackageDetailRequest {
        identity: identity.clone(),
        generation: next_package_generation(app),
    };
    app.package_details.insert(
        identity,
        PackageDetailState::Loading {
            request: request.clone(),
        },
    );
    Effect::GetPackageDetail(request)
}

fn select_package_identity(
    app: &mut App,
    identity: PackageIdentity,
    load_detail: bool,
) -> Option<Effect> {
    if !app
        .package_inventory
        .packages()
        .is_some_and(|packages| packages.iter().any(|package| package.identity == identity))
    {
        app.notification =
            Some("The dependency is not present in the current package inventory.".into());
        return None;
    }
    if let Some(current) = app.package_selection.replace(identity.clone())
        && current != identity
    {
        if app.package_navigation.len() == 64 {
            app.package_navigation.remove(0);
        }
        app.package_navigation.push(current);
    }
    app.package_dependency_selection = 0;
    if !load_detail || app.package_details.contains_key(&identity) {
        None
    } else {
        Some(begin_package_detail(app, identity))
    }
}

pub fn update(app: &mut App, action: Action) -> Option<Effect> {
    if modal_focus(app).is_some()
        && matches!(
            &action,
            Action::Open(_)
                | Action::SelectNavigator { .. }
                | Action::ActivateNavigator
                | Action::CycleFocus { .. }
                | Action::Focus(
                    FocusTarget::Navigator | FocusTarget::Workspace | FocusTarget::Inspector
                )
                | Action::OpenCommandPalette
                | Action::OpenBuildOptions
                | Action::OpenImagePicker(_)
        )
    {
        return None;
    }
    match action {
        Action::ProjectProfileAbsent => {
            app.project_profile = ProjectProfileState::Absent;
        }
        Action::ProjectProfileLoaded(profile) => match profile.validate() {
            Ok(()) => app.project_profile = ProjectProfileState::Loaded(profile),
            Err(error) => app.project_profile = ProjectProfileState::Invalid(error.to_string()),
        },
        Action::ProjectProfileLoadFailed(message) => {
            app.project_profile = ProjectProfileState::Invalid(message);
        }
        Action::PreviewProjectProfileGeneration(profile) => match profile.validate() {
            Ok(()) => app.project_profile = ProjectProfileState::GenerationPreview(profile),
            Err(error) => app.notification = Some(error.to_string()),
        },
        Action::ConfirmProjectProfileGeneration { replace } => {
            let ProjectProfileState::GenerationPreview(profile) = &app.project_profile else {
                return None;
            };
            let profile = profile.clone();
            app.project_profile = ProjectProfileState::Generating(profile.clone());
            return Some(Effect::GenerateProjectProfile { profile, replace });
        }
        Action::ProjectProfileGenerated(profile) => {
            app.project_profile = ProjectProfileState::Loaded(profile);
            app.notification = Some("Project profile generated.".into());
        }
        Action::ProjectProfileGenerationFailed(message) => {
            app.notification = Some(format!("Project profile was not generated: {message}"));
            if let ProjectProfileState::Generating(profile) = &app.project_profile {
                app.project_profile = ProjectProfileState::GenerationPreview(profile.clone());
            }
        }
        Action::SelectProjectProfileItem { delta } => {
            let count =
                project_profile_items(&app.project_profile, &app.workspace, &app.available_images)
                    .len();
            app.project_profile_selection = if delta < 0 {
                app.project_profile_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.project_profile_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::ActivateProjectProfileItem => {
            let items =
                project_profile_items(&app.project_profile, &app.workspace, &app.available_images);
            let item = items.get(app.project_profile_selection)?;
            if !matches!(item.status, ProjectProfileItemStatus::Resolved) {
                app.notification = Some(format!("{} is not currently resolved.", item.label));
                return None;
            }
            let ProjectProfileState::Loaded(profile) = &app.project_profile else {
                app.notification = Some("Project profile is not ready for activation.".into());
                return None;
            };
            match item.kind {
                ProjectProfileItemKind::BuildPreset(index) => {
                    let preset = &profile.build_presets[index];
                    replace_dialog(
                        app,
                        Dialog::RecipeTaskConfirmation(BuildRequest {
                            targets: preset.targets.clone(),
                            task: None,
                            force: false,
                        }),
                    );
                }
                ProjectProfileItemKind::FavoriteRecipe(index) => {
                    let identity = &profile.favorites.recipes[index];
                    app.recipe_selection = app
                        .workspace
                        .recipes
                        .iter()
                        .position(|recipe| &recipe.name == identity)
                        .unwrap_or(0);
                    app.screen = Screen::Recipes;
                }
                ProjectProfileItemKind::FavoriteImage(_) => app.screen = Screen::Images,
                ProjectProfileItemKind::FavoriteLayer(index) => {
                    let identity = &profile.favorites.layers[index];
                    app.layer_selection = app
                        .workspace
                        .layers
                        .iter()
                        .position(|layer| &layer.name == identity)
                        .unwrap_or(0);
                    app.screen = Screen::Layers;
                }
                ProjectProfileItemKind::Workflow(_) => {
                    app.notification = Some(
                        "Workflow selected. Review each typed step; loading never executes it."
                            .into(),
                    );
                }
            }
        }
        Action::Open(s) => {
            app.screen = s;
            app.focus = FocusTarget::Workspace;
            app.focus_return = None;
            if let Some(index) = NAVIGATOR_SCREENS
                .iter()
                .position(|candidate| *candidate == s)
            {
                app.navigator_selection = index;
            }
            if s == Screen::Packages
                && matches!(app.package_inventory, PackageInventoryState::NotLoaded)
            {
                return Some(begin_package_inventory(app));
            }
            if s == Screen::Images
                && matches!(app.image_artifacts, ImageArtifactInventoryState::NotLoaded)
            {
                return begin_image_artifact_inventory(app);
            }
            if s == Screen::Sdk
                && matches!(app.sdk_tool_capability, SdkToolCapability::NotInspected)
            {
                return Some(Effect::InspectSdkTools);
            }
            if s == Screen::Testing
                && matches!(
                    app.test_capability.oe_selftest,
                    TestExecutableCapability::NotInspected
                )
                && matches!(
                    app.test_capability.bitbake_selftest,
                    TestExecutableCapability::NotInspected
                )
            {
                return Some(Effect::InspectTestCapability);
            }
            if s == Screen::Testing
                && matches!(
                    app.result_tool_capability,
                    ResultToolCapability::NotInspected
                )
            {
                return Some(Effect::InspectResultToolCapability);
            }
            if s == Screen::Security
                && matches!(app.security.capability, SecurityCapability::NotInspected)
            {
                return Some(Effect::Security(SecurityEffect::InspectCapability));
            }
            if s == Screen::Qa && matches!(app.qa.capability, QaCapability::NotInspected) {
                return Some(Effect::Qa(QaEffect::InspectCapability {
                    scope: app.qa.scope.clone(),
                }));
            }
            if s == Screen::Maintenance
                && matches!(
                    app.maintenance.capability,
                    MaintenanceCapability::NotInspected
                )
            {
                return update(
                    app,
                    Action::Maintenance(MaintenanceAction::InspectCapability),
                );
            }
        }
        Action::SelectNavigator { delta } => {
            app.navigator_selection = if delta.is_negative() {
                app.navigator_selection.saturating_sub(delta.unsigned_abs())
            } else {
                app.navigator_selection
                    .saturating_add(delta as usize)
                    .min(NAVIGATOR_SCREENS.len().saturating_sub(1))
            };
        }
        Action::SelectPtySession { delta } => {
            let count = app.daemon.pty_sessions.len();
            app.pty_selection = if count == 0 {
                0
            } else if delta.is_negative() {
                app.pty_selection
                    .saturating_sub(delta.unsigned_abs())
                    .min(count.saturating_sub(1))
            } else {
                app.pty_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::ResizeFocusedPane { delta_per_mille } => {
            let focused = app.pane_layout.focused;
            let _ = app.pane_layout.resize(focused, delta_per_mille);
        }
        Action::ActivateNavigator => {
            app.screen = NAVIGATOR_SCREENS[app.navigator_selection];
            app.focus = FocusTarget::Workspace;
            app.focus_return = None;
            if app.screen == Screen::Packages
                && matches!(app.package_inventory, PackageInventoryState::NotLoaded)
            {
                return Some(begin_package_inventory(app));
            }
            if app.screen == Screen::Images
                && matches!(app.image_artifacts, ImageArtifactInventoryState::NotLoaded)
            {
                return begin_image_artifact_inventory(app);
            }
            if app.screen == Screen::Sdk
                && matches!(app.sdk_tool_capability, SdkToolCapability::NotInspected)
            {
                return Some(Effect::InspectSdkTools);
            }
            if app.screen == Screen::Testing
                && matches!(
                    app.test_capability.oe_selftest,
                    TestExecutableCapability::NotInspected
                )
                && matches!(
                    app.test_capability.bitbake_selftest,
                    TestExecutableCapability::NotInspected
                )
            {
                return Some(Effect::InspectTestCapability);
            }
            if app.screen == Screen::Testing
                && matches!(
                    app.result_tool_capability,
                    ResultToolCapability::NotInspected
                )
            {
                return Some(Effect::InspectResultToolCapability);
            }
            if app.screen == Screen::Security
                && matches!(app.security.capability, SecurityCapability::NotInspected)
            {
                return Some(Effect::Security(SecurityEffect::InspectCapability));
            }
            if app.screen == Screen::Qa && matches!(app.qa.capability, QaCapability::NotInspected) {
                return Some(Effect::Qa(QaEffect::InspectCapability {
                    scope: app.qa.scope.clone(),
                }));
            }
            if app.screen == Screen::Maintenance
                && matches!(
                    app.maintenance.capability,
                    MaintenanceCapability::NotInspected
                )
            {
                return update(
                    app,
                    Action::Maintenance(MaintenanceAction::InspectCapability),
                );
            }
        }
        Action::Security(action) => {
            let transition = update_security(&mut app.security, action);
            match transition.dialog {
                SecurityDialogUpdate::None => {}
                SecurityDialogUpdate::Open(dialog) => {
                    if matches!(app.active_dialog(), Some(Dialog::Security(_))) {
                        replace_dialog(app, Dialog::Security(dialog));
                    } else {
                        open_dialog(app, Dialog::Security(dialog));
                    }
                }
                SecurityDialogUpdate::Close => {
                    if matches!(app.active_dialog(), Some(Dialog::Security(_))) {
                        close_dialog(app);
                    }
                }
            }
            if let Some(message) = transition.notification {
                app.notification = Some(message);
            }
            synchronize_focus(app);
            return transition.effect.map(Effect::Security);
        }
        Action::Qa(action) => {
            let transition = update_qa(&mut app.qa, action);
            match transition.dialog {
                QaDialogUpdate::None => {}
                QaDialogUpdate::Open(dialog) => {
                    if matches!(app.active_dialog(), Some(Dialog::Qa(_))) {
                        replace_dialog(app, Dialog::Qa(*dialog));
                    } else {
                        open_dialog(app, Dialog::Qa(*dialog));
                    }
                }
                QaDialogUpdate::Close => {
                    if matches!(app.active_dialog(), Some(Dialog::Qa(_))) {
                        close_dialog(app);
                    }
                }
            }
            if let Some(message) = transition.notification {
                app.notification = Some(message);
            }
            synchronize_focus(app);
            return transition.effect.map(Effect::Qa);
        }
        Action::Maintenance(action) => {
            let transition = update_maintenance(&mut app.maintenance, action);
            match transition.dialog {
                MaintenanceDialogUpdate::None => {}
                MaintenanceDialogUpdate::Open(dialog) => {
                    if matches!(app.active_dialog(), Some(Dialog::Maintenance(_))) {
                        replace_dialog(app, Dialog::Maintenance(dialog));
                    } else {
                        open_dialog(app, Dialog::Maintenance(dialog));
                    }
                }
                MaintenanceDialogUpdate::Close => {
                    if matches!(app.active_dialog(), Some(Dialog::Maintenance(_))) {
                        close_dialog(app);
                    }
                }
            }
            if let Some(message) = transition.notification {
                app.notification = Some(message);
            }
            synchronize_focus(app);
            return transition.effect.map(Effect::Maintenance);
        }
        Action::Focus(target) => app.focus = target,
        Action::OpenCommandPalette => {
            app.command_palette_open = true;
            app.command_palette_selection = 0;
            app.command_palette_query.clear();
        }
        Action::SelectCommandPalette { delta } => {
            let count = app.filtered_command_palette_commands().len();
            app.command_palette_selection = if delta.is_negative() {
                app.command_palette_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.command_palette_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::AppendCommandPaletteQuery(character) if app.command_palette_open => {
            app.command_palette_query.push(character);
            app.command_palette_selection = 0;
        }
        Action::BackspaceCommandPaletteQuery if app.command_palette_open => {
            app.command_palette_query.pop();
            app.command_palette_selection = 0;
        }
        Action::ActivateCommandPalette => {
            if !app.command_palette_open {
                return None;
            }
            let command = app
                .filtered_command_palette_commands()
                .get(app.command_palette_selection)
                .cloned()?;
            if !command.enabled() {
                return None;
            }
            app.command_palette_open = false;
            return update(app, command_action(app, command.id));
        }
        Action::CloseCommandPalette => {
            app.command_palette_open = false;
        }
        Action::SelectSetting { delta } => {
            app.settings_selection = if delta.is_negative() {
                app.settings_selection.saturating_sub(delta.unsigned_abs())
            } else {
                app.settings_selection
                    .saturating_add(delta as usize)
                    .min(SETTINGS.len().saturating_sub(1))
            };
        }
        Action::ChangeSelectedSetting { backwards } => {
            if app.settings_selection >= SETTINGS.len() {
                return None;
            }
            match SETTINGS[app.settings_selection.min(SETTINGS.len() - 1)] {
                Setting::Theme => app.theme = cycle_theme(app.theme, backwards),
                Setting::AnimationSpeed => {
                    app.animation_speed = match app.animation_speed {
                        AnimationSpeed::Slow => AnimationSpeed::Fast,
                        AnimationSpeed::Fast => AnimationSpeed::Slow,
                    }
                }
                Setting::ReducedMotion => app.reduced_motion = !app.reduced_motion,
                Setting::Color if app.color_forced_off => {
                    app.notification =
                        Some("Color is disabled by --no-color for this launch".into());
                    return None;
                }
                Setting::Color => app.color_enabled = !app.color_enabled,
                Setting::LogWrap => {
                    app.logs.wrap = !app.logs.wrap;
                    if app.logs.wrap {
                        app.logs.horizontal_offset = 0;
                    }
                }
                Setting::LogFollow => {
                    app.logs.follow = !app.logs.follow;
                    app.logs.paused_len = (!app.logs.follow).then_some(app.logs.entries.len());
                    if app.logs.follow {
                        app.logs.selection = app.logs.filtered().count().saturating_sub(1);
                        app.logs.scroll_offset = 0;
                    }
                }
            }
            app.settings_dirty = true;
            return Some(Effect::PersistSettings);
        }
        Action::RetrySettingsPersistence if app.settings_dirty => {
            return Some(Effect::PersistSettings);
        }
        Action::RetrySettingsPersistence => {}
        Action::SettingsPersisted => {
            app.settings_dirty = false;
            app.notification = None;
        }
        Action::SettingsPersistenceFailed(message) => {
            app.settings_dirty = true;
            app.notification = Some(format!(
                "Settings changed in memory but could not be saved: {message}"
            ));
        }
        Action::EditActivePopup(command) => {
            let (editor, validation_error) = match app.active_dialog_mut() {
                Some(
                    Dialog::BuildEnvironmentEditor(editor)
                    | Dialog::BuildEnvironmentCloneEditor(editor)
                    | Dialog::BbmaskEdit(editor)
                    | Dialog::SdkPublishTomlEditor(editor)
                    | Dialog::SdkNativeTomlEditor(editor),
                ) => (editor, None),
                Some(Dialog::ConfigEdit { editor, .. }) => (editor, None),
                Some(Dialog::BuildTarget { editor, .. }) => (editor, None),
                Some(Dialog::WicCreateTomlEditor {
                    editor,
                    validation_error,
                })
                | Some(Dialog::TestLaunchTomlEditor {
                    editor,
                    validation_error,
                })
                | Some(Dialog::TestResultImportTomlEditor {
                    editor,
                    validation_error,
                })
                | Some(Dialog::TestComparisonTomlEditor {
                    editor,
                    validation_error,
                }) => (editor, Some(validation_error)),
                Some(Dialog::Security(SecurityDialog::Import {
                    editor,
                    validation_error,
                })) => (editor, Some(validation_error)),
                Some(Dialog::Qa(QaDialog::Import {
                    editor,
                    validation_error,
                })) => (editor, Some(validation_error)),
                Some(Dialog::Maintenance(dialog)) => match dialog.as_mut() {
                    MaintenanceDialog::ReadinessToml {
                        editor,
                        validation_error,
                    }
                    | MaintenanceDialog::CleanupToml {
                        editor,
                        validation_error,
                    }
                    | MaintenanceDialog::PrServiceToml {
                        editor,
                        validation_error,
                        ..
                    }
                    | MaintenanceDialog::LockedCacheToml {
                        editor,
                        validation_error,
                    }
                    | MaintenanceDialog::BuildHistoryToml {
                        editor,
                        validation_error,
                    }
                    | MaintenanceDialog::GitArchiveToml {
                        editor,
                        validation_error,
                    } => (editor, Some(validation_error)),
                    _ => return None,
                },
                _ => return None,
            };
            match command {
                PopupEditorCommand::ToggleInsert => editor.editing = !editor.editing,
                PopupEditorCommand::Insert(character)
                    if editor.editing
                        && !character.is_control()
                        && editor.text.len() + character.len_utf8() <= 16_384 =>
                {
                    editor.insert(&character.to_string());
                    if let Some(error) = validation_error {
                        *error = None;
                    }
                }
                PopupEditorCommand::Insert(_) => {}
                PopupEditorCommand::Backspace if editor.editing => {
                    editor.backspace();
                    if let Some(error) = validation_error {
                        *error = None;
                    }
                }
                PopupEditorCommand::Backspace => {}
                PopupEditorCommand::Left => editor.left(),
                PopupEditorCommand::Right => editor.right(),
                PopupEditorCommand::Up => editor.up(),
                PopupEditorCommand::Down => editor.down(),
                PopupEditorCommand::Home => editor.home(),
                PopupEditorCommand::End => editor.end(),
                PopupEditorCommand::SelectValue => match editor.select_toml_value_at_cursor() {
                    Ok(()) => editor.editing = true,
                    Err(message) => app.notification = Some(message),
                },
                PopupEditorCommand::Copy => {
                    return Some(Effect::CopyToClipboard(editor.copy_selection_or_line()));
                }
                PopupEditorCommand::Paste if editor.editing => {
                    editor.paste();
                    if let Some(error) = validation_error {
                        *error = None;
                    }
                }
                PopupEditorCommand::Paste => {}
            }
        }
        Action::OpenBuildEnvironmentCloneEditor => {
            let mut editor = PopupEditor::new("repository = \"https://git.yoctoproject.org/poky\"\ndestination = \"/home/user/src/poky\"\nrevision = \"\"\nbuild = \"/home/user/src/poky/build-yoctui\"\n".into());
            let _ = editor.select_toml_value("repository");
            open_dialog(app, Dialog::BuildEnvironmentCloneEditor(editor));
        }
        Action::ToggleBuildEnvironmentCloneEditor => {
            if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendBuildEnvironmentCloneEditor(character) => {
            if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceBuildEnvironmentCloneEditor => {
            if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::ReviewBuildEnvironmentClone => {
            if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog().cloned()
            {
                let mut values = HashMap::new();
                for line in editor.text.lines() {
                    if let Some((name, value)) = line.split_once('=') {
                        values.insert(
                            name.trim().to_owned(),
                            value.trim().trim_matches('"').to_owned(),
                        );
                    }
                }
                let plan = BuildEnvironmentClonePlan {
                    request: BuildEnvironmentCloneRequest {
                        repository: values.remove("repository").unwrap_or_default(),
                        destination: PathBuf::from(
                            values.remove("destination").unwrap_or_default(),
                        ),
                        revision: values.remove("revision").filter(|value| !value.is_empty()),
                    },
                    build_dir: PathBuf::from(values.remove("build").unwrap_or_default()),
                };
                match plan.validate() {
                    Ok(()) => replace_dialog(app, Dialog::BuildEnvironmentCloneReview(plan)),
                    Err(error) => app.notification = Some(error.to_string()),
                }
            }
        }
        Action::ConfirmBuildEnvironmentClone => {
            if let Some(Dialog::BuildEnvironmentCloneReview(plan)) = app.active_dialog().cloned() {
                close_dialog(app);
                return Some(Effect::CloneBuildEnvironment(plan));
            }
        }
        Action::CancelBuildEnvironmentClone => {
            if matches!(
                app.active_dialog(),
                Some(
                    Dialog::BuildEnvironmentCloneEditor(_) | Dialog::BuildEnvironmentCloneReview(_)
                )
            ) {
                close_dialog(app);
            }
        }
        Action::OpenBuildEnvironmentEditor => {
            let profile = match &app.build_environment {
                BuildEnvironmentState::Configured(profile)
                | BuildEnvironmentState::Connected(profile)
                | BuildEnvironmentState::Failed { profile, .. }
                | BuildEnvironmentState::Verifying { profile, .. } => Some(profile),
                BuildEnvironmentState::Unconfigured => None,
            };
            let value = |name: &str, path: Option<&Path>| {
                format!(
                    "{name} = \"{}\"",
                    path.map_or_else(String::new, |p| p.display().to_string())
                )
            };
            let content = format!(
                "{}\n{}\n{}\n",
                value("source", profile.map(|p| p.source_dir.as_path())),
                value("build", profile.map(|p| p.build_dir.as_path())),
                value("script", profile.map(|p| p.init_script.as_path()))
            );
            let mut editor = PopupEditor::new(content);
            let _ = editor.select_toml_value("source");
            open_dialog(app, Dialog::BuildEnvironmentEditor(editor));
        }
        Action::ToggleBuildEnvironmentEditor => {
            if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendBuildEnvironmentEditor(character) => {
            if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceBuildEnvironmentEditor => {
            if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::ApplyBuildEnvironmentEditor => {
            if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog().cloned() {
                let mut values = HashMap::new();
                for line in editor.text.lines() {
                    if let Some((name, value)) = line.split_once('=') {
                        let value = value.trim().trim_matches('"').to_owned();
                        values.insert(name.trim().to_owned(), value);
                    }
                }
                let profile = BuildEnvironmentProfile {
                    source_dir: PathBuf::from(values.remove("source").unwrap_or_default()),
                    build_dir: PathBuf::from(values.remove("build").unwrap_or_default()),
                    init_script: PathBuf::from(values.remove("script").unwrap_or_default()),
                };
                close_dialog(app);
                return update(app, Action::ConfigureBuildEnvironment(profile));
            }
        }
        Action::CloseBuildEnvironmentEditor => {
            if matches!(app.active_dialog(), Some(Dialog::BuildEnvironmentEditor(_))) {
                close_dialog(app);
            }
        }
        Action::OpenThemePicker => {
            let selection = THEMES
                .iter()
                .position(|theme| *theme == app.theme)
                .unwrap_or(0);
            open_dialog(
                app,
                Dialog::ThemePicker {
                    selection,
                    original_theme: app.theme,
                    original_color_enabled: app.color_enabled,
                    original_settings_dirty: app.settings_dirty,
                },
            );
        }
        Action::SelectTheme { delta } => {
            if let Some(Dialog::ThemePicker { selection, .. }) = app.active_dialog_mut() {
                *selection = if delta.is_negative() {
                    selection.saturating_sub(delta.unsigned_abs())
                } else {
                    selection
                        .saturating_add(delta as usize)
                        .min(THEMES.len() - 1)
                };
                app.theme = THEMES[*selection];
                if !app.color_forced_off {
                    app.color_enabled = true;
                }
                app.settings_dirty = true;
            }
        }
        Action::ApplySelectedTheme => {
            if let Some(Dialog::ThemePicker { selection, .. }) = app.active_dialog().cloned() {
                app.theme = THEMES[selection.min(THEMES.len() - 1)];
                if !app.color_forced_off {
                    app.color_enabled = true;
                }
                app.settings_dirty = true;
                close_dialog(app);
                return Some(Effect::PersistSettings);
            }
        }
        Action::CloseThemePicker => {
            if let Some(Dialog::ThemePicker {
                original_theme,
                original_color_enabled,
                original_settings_dirty,
                ..
            }) = app.active_dialog().cloned()
            {
                app.theme = original_theme;
                app.color_enabled = original_color_enabled;
                app.settings_dirty = original_settings_dirty;
                close_dialog(app);
            }
        }
        Action::ConfigureBuildEnvironment(profile) => match profile.validate() {
            Ok(()) => {
                app.build_environment = BuildEnvironmentState::Configured(profile);
                app.available_images.clear();
                app.notification = None;
            }
            Err(error) => app.notification = Some(error.to_string()),
        },
        Action::BeginBuildEnvironmentEdit => {
            let profile = match &app.build_environment {
                BuildEnvironmentState::Configured(profile)
                | BuildEnvironmentState::Connected(profile)
                | BuildEnvironmentState::Failed { profile, .. }
                | BuildEnvironmentState::Verifying { profile, .. } => Some(profile),
                BuildEnvironmentState::Unconfigured => None,
            };
            app.build_environment_draft = Some(BuildEnvironmentDraft {
                source: profile.map_or_else(String::new, |p| p.source_dir.display().to_string()),
                build: profile.map_or_else(String::new, |p| p.build_dir.display().to_string()),
                script: profile.map_or_else(String::new, |p| p.init_script.display().to_string()),
                field: BuildEnvironmentField::Source,
                editing: true,
            });
        }
        Action::SelectBuildEnvironmentField { delta } => {
            if let Some(draft) = app.build_environment_draft.as_mut() {
                let index: usize = match draft.field {
                    BuildEnvironmentField::Source => 0,
                    BuildEnvironmentField::Build => 1,
                    BuildEnvironmentField::Script => 2,
                };
                let next = if delta.is_negative() {
                    index.saturating_sub(delta.unsigned_abs())
                } else {
                    index.saturating_add(delta as usize).min(2)
                };
                draft.field = match next {
                    0 => BuildEnvironmentField::Source,
                    1 => BuildEnvironmentField::Build,
                    _ => BuildEnvironmentField::Script,
                };
            }
        }
        Action::AppendBuildEnvironmentField(character) => {
            if let Some(draft) = app.build_environment_draft.as_mut() {
                let value = match draft.field {
                    BuildEnvironmentField::Source => &mut draft.source,
                    BuildEnvironmentField::Build => &mut draft.build,
                    BuildEnvironmentField::Script => &mut draft.script,
                };
                value.push(character);
            }
        }
        Action::BackspaceBuildEnvironmentField => {
            if let Some(draft) = app.build_environment_draft.as_mut() {
                let value = match draft.field {
                    BuildEnvironmentField::Source => &mut draft.source,
                    BuildEnvironmentField::Build => &mut draft.build,
                    BuildEnvironmentField::Script => &mut draft.script,
                };
                value.pop();
            }
        }
        Action::FinishBuildEnvironmentEdit => {
            if let Some(draft) = app.build_environment_draft.as_mut() {
                draft.editing = false;
            }
        }
        Action::CancelBuildEnvironmentEdit => app.build_environment_draft = None,
        Action::ApplyBuildEnvironmentProfile => {
            if let Some(draft) = app.build_environment_draft.take() {
                let profile = BuildEnvironmentProfile {
                    source_dir: PathBuf::from(draft.source),
                    build_dir: PathBuf::from(draft.build),
                    init_script: PathBuf::from(draft.script),
                };
                let _ = update(app, Action::ConfigureBuildEnvironment(profile));
            }
        }
        Action::BeginBuildEnvironmentVerification => {
            let profile = match &app.build_environment {
                BuildEnvironmentState::Configured(profile)
                | BuildEnvironmentState::Failed { profile, .. } => profile.clone(),
                BuildEnvironmentState::Verifying { .. } => return None,
                BuildEnvironmentState::Unconfigured | BuildEnvironmentState::Connected(_) => {
                    app.notification =
                        Some("Select a build environment before verification.".into());
                    return None;
                }
            };
            app.build_environment_generation = app.build_environment_generation.wrapping_add(1);
            let generation = app.build_environment_generation;
            app.build_environment = BuildEnvironmentState::Verifying {
                profile: profile.clone(),
                generation,
            };
            return Some(Effect::VerifyBuildEnvironment {
                profile,
                generation,
            });
        }
        Action::BuildEnvironmentVerified { generation } => {
            if let BuildEnvironmentState::Verifying {
                profile,
                generation: pending,
            } = &app.build_environment
                && *pending == generation
            {
                app.build_environment = BuildEnvironmentState::Connected(profile.clone());
                app.notification = None;
            }
        }
        Action::BuildEnvironmentVerificationFailed {
            generation,
            message,
        } => {
            if let BuildEnvironmentState::Verifying {
                profile,
                generation: pending,
            } = &app.build_environment
                && *pending == generation
            {
                app.build_environment = BuildEnvironmentState::Failed {
                    profile: profile.clone(),
                    message: message.clone(),
                };
                app.notification = Some(format!("BitBake connection failed: {message}"));
            }
        }
        Action::CycleFocus { backwards } => {
            if matches!(app.focus, FocusTarget::Dialog | FocusTarget::CommandPalette) {
                return None;
            }
            const TARGETS: [FocusTarget; 3] = [
                FocusTarget::Navigator,
                FocusTarget::Workspace,
                FocusTarget::Inspector,
            ];
            let current = TARGETS
                .iter()
                .position(|target| *target == app.focus)
                .unwrap_or(1);
            let next = if backwards {
                (current + TARGETS.len() - 1) % TARGETS.len()
            } else {
                (current + 1) % TARGETS.len()
            };
            app.focus = TARGETS[next];
        }
        Action::OpenBuildOptions => {
            if app.build_environment.connected() {
                open_dialog(app, Dialog::BuildOptions);
            } else {
                app.notification = Some("Configure and verify a BitBake environment first".into());
            }
        }
        Action::CloseBuildOptions => {
            if matches!(app.active_dialog(), Some(Dialog::BuildOptions)) {
                close_dialog(app);
            }
        }
        Action::OpenImagePicker(mut images) => {
            images.sort();
            images.dedup();
            let selection = app
                .build
                .target
                .as_ref()
                .and_then(|target| images.iter().position(|image| image == target))
                .unwrap_or(0);
            if images.is_empty() {
                app.notification =
                    Some("No image recipes were discovered in the active layers.".into());
            } else {
                open_dialog(app, Dialog::ImagePicker(ImagePicker { images, selection }));
            }
        }
        Action::SelectImage { delta } => {
            if let Some(Dialog::ImagePicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.images.len().saturating_sub(1))
                };
            }
        }
        Action::ConfirmImagePicker => {
            if let Some(Dialog::ImagePicker(picker)) = app.active_dialog() {
                let image = picker.images.get(picker.selection).cloned();
                if let Some(image) = image {
                    app.build.target = Some(image);
                    close_dialog(app);
                }
            }
        }
        Action::CancelImagePicker => {
            if matches!(app.active_dialog(), Some(Dialog::ImagePicker(_))) {
                close_dialog(app);
            }
        }
        Action::BeginCurrentImageBuild => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::RecipeEditor(editor)) if editor.dirty
            ) {
                app.notification = Some("Save the edited file with Ctrl+S before building.".into());
            } else if let Some(target) = app.build.target.clone() {
                replace_dialog(
                    app,
                    Dialog::RecipeTaskConfirmation(BuildRequest {
                        targets: vec![target],
                        task: None,
                        force: false,
                    }),
                );
            } else {
                app.notification = Some("Select an image first with i.".into());
            }
        }
        Action::BeginImageArtifactInventory | Action::RefreshImageArtifactInventory => {
            if image_artifact_operation_is_loading(app) {
                app.notification = Some("An image artifact operation is already running.".into());
                return None;
            }
            app.qemu_capability = QemuCapability::NotInspected;
            return begin_image_artifact_inventory(app);
        }
        Action::CancelImageArtifactOperation => {
            if image_artifact_operation_is_loading(app) {
                return Some(Effect::CancelImageArtifactOperation);
            }
            app.notification = Some("No image artifact operation is running.".into());
        }
        Action::ImageArtifactInventoryLoaded { request, inventory } => {
            if !matches!(
                &app.image_artifacts,
                ImageArtifactInventoryState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            set_image_artifact_inventory(app, request, inventory, None);
        }
        Action::ImageArtifactInventoryPartial {
            request,
            inventory,
            limitations,
        } => {
            if !matches!(
                &app.image_artifacts,
                ImageArtifactInventoryState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            set_image_artifact_inventory(app, request, inventory, Some(limitations));
        }
        Action::ImageArtifactInventoryFailed { request, message } => {
            if !matches!(
                &app.image_artifacts,
                ImageArtifactInventoryState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            app.image_artifacts = ImageArtifactInventoryState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!(
                "Image artifact inventory is unavailable: {message}"
            ));
        }
        Action::SelectImageArtifact { delta } => {
            let visible = app
                .filtered_image_artifacts()
                .into_iter()
                .map(|artifact| artifact.identity.clone())
                .collect::<Vec<_>>();
            if visible.is_empty() {
                app.image_artifact_selection = None;
                return None;
            }
            let current = app
                .image_artifact_selection
                .as_ref()
                .and_then(|identity| visible.iter().position(|candidate| candidate == identity))
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(visible.len().saturating_sub(1))
            };
            app.image_artifact_selection = Some(visible[next].clone());
        }
        Action::BeginImageArtifactSearch => app.image_artifact_searching = true,
        Action::AppendImageArtifactQuery(character) => {
            if app.image_artifact_searching
                && !character.is_control()
                && app.image_artifact_query.len() < 256
            {
                app.image_artifact_query.push(character);
                set_image_artifact_selection_to_current_or_first(
                    app,
                    app.image_artifact_selection.clone(),
                );
            }
        }
        Action::BackspaceImageArtifactQuery => {
            if app.image_artifact_searching {
                app.image_artifact_query.pop();
                set_image_artifact_selection_to_current_or_first(
                    app,
                    app.image_artifact_selection.clone(),
                );
            }
        }
        Action::FinishImageArtifactSearch => app.image_artifact_searching = false,
        Action::BeginSelectedImageArtifactBuild => {
            if let Some(target) = app
                .selected_image_artifact()
                .map(|artifact| artifact.identity.image.clone())
            {
                app.build.target = Some(target.clone());
                open_dialog(
                    app,
                    Dialog::RecipeTaskConfirmation(BuildRequest {
                        targets: vec![target],
                        task: None,
                        force: false,
                    }),
                );
            } else if let Some(target) = app.build.target.clone() {
                open_dialog(
                    app,
                    Dialog::RecipeTaskConfirmation(BuildRequest {
                        targets: vec![target],
                        task: None,
                        force: false,
                    }),
                );
            } else {
                app.notification = Some("Select an image first with i.".into());
            }
        }
        Action::OpenSelectedImageArtifact => {
            if let Some(path) = app
                .selected_image_artifact()
                .map(|artifact| artifact.identity.path.clone())
            {
                return Some(Effect::OpenInEditor(path));
            }
            app.notification = Some("No deployed image artifact is selected.".into());
        }
        Action::OpenSelectedImageArtifactAssociation(association) => {
            if let Some(path) = app
                .selected_image_artifact()
                .and_then(|artifact| artifact.associated_paths(association))
                .and_then(|paths| paths.first())
                .cloned()
            {
                return Some(Effect::OpenInEditor(path));
            }
            let label = match association {
                ImageArtifactAssociation::Manifest => "manifest",
                ImageArtifactAssociation::License => "license",
                ImageArtifactAssociation::Spdx => "SPDX/SBOM",
                ImageArtifactAssociation::Wic => "Wic",
            };
            app.notification = Some(format!(
                "The selected image artifact has no authoritative {label} path."
            ));
        }
        Action::BeginSdkBuild(action) => {
            let Some(image) = app.build.target.clone() else {
                app.notification = Some("Select an SDK image target with i first.".into());
                return None;
            };
            let machine = app
                .workspace
                .variables
                .get("MACHINE")
                .cloned()
                .unwrap_or_default();
            let distro = app
                .workspace
                .variables
                .get("DISTRO")
                .cloned()
                .unwrap_or_default();
            match SdkBuildPreview::new(machine, distro, image, action) {
                Ok(preview) => open_dialog(app, Dialog::SdkBuildConfirmation(preview)),
                Err(message) => app.notification = Some(message.into()),
            }
        }
        Action::ConfirmSdkBuild => {
            let Some(Dialog::SdkBuildConfirmation(preview)) = app.active_dialog().cloned() else {
                return None;
            };
            let current = SdkBuildPreview::new(
                app.workspace
                    .variables
                    .get("MACHINE")
                    .cloned()
                    .unwrap_or_default(),
                app.workspace
                    .variables
                    .get("DISTRO")
                    .cloned()
                    .unwrap_or_default(),
                app.build.target.clone().unwrap_or_default(),
                preview.action,
            );
            if current.as_ref() != Ok(&preview) {
                app.notification = Some("The SDK build preview is stale; review it again.".into());
                return None;
            }
            close_dialog(app);
            return Some(Effect::Start(preview.request));
        }
        Action::CancelSdkBuild => {
            if matches!(app.active_dialog(), Some(Dialog::SdkBuildConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::BeginSdkArtifactInventory | Action::RefreshSdkArtifactInventory => {
            if matches!(app.sdk_artifacts, SdkArtifactInventoryState::Loading { .. }) {
                app.notification = Some("An SDK artifact scan is already running.".into());
                return None;
            }
            return begin_sdk_artifact_inventory(app);
        }
        Action::SdkArtifactInventoryLoaded {
            request,
            artifacts,
            limitations,
        } => {
            if !matches!(
                &app.sdk_artifacts,
                SdkArtifactInventoryState::Loading { request: pending } if pending == &request
            ) {
                note_stale_sdk_event(app);
                return None;
            }
            let previous = app.sdk_artifact_selection.take();
            match normalize_sdk_artifacts(&request, artifacts) {
                Ok(artifacts) => {
                    let limitations = normalize_sdk_limitations(limitations);
                    app.sdk_artifacts = if artifacts.is_empty() && limitations.is_empty() {
                        SdkArtifactInventoryState::AvailableEmpty { request }
                    } else if limitations.is_empty() {
                        SdkArtifactInventoryState::Available { request, artifacts }
                    } else {
                        SdkArtifactInventoryState::Partial {
                            request,
                            artifacts,
                            limitations,
                        }
                    };
                    set_sdk_artifact_selection_to_current_or_first(app, previous);
                }
                Err(message) => {
                    app.sdk_artifacts = SdkArtifactInventoryState::Failed {
                        request,
                        message: message.into(),
                    };
                    app.notification =
                        Some("SDK artifact inventory failed model validation.".into());
                }
            }
        }
        Action::SdkArtifactInventoryFailed { request, message } => {
            if !matches!(
                &app.sdk_artifacts,
                SdkArtifactInventoryState::Loading { request: pending } if pending == &request
            ) {
                note_stale_sdk_event(app);
                return None;
            }
            app.sdk_artifacts = SdkArtifactInventoryState::Failed { request, message };
            app.sdk_artifact_selection = None;
        }
        Action::SelectSdkArtifact { delta } => {
            let visible = app
                .filtered_sdk_artifacts()
                .into_iter()
                .map(|artifact| artifact.identity.clone())
                .collect::<Vec<_>>();
            if visible.is_empty() {
                app.sdk_artifact_selection = None;
                return None;
            }
            let current = app
                .sdk_artifact_selection
                .as_ref()
                .and_then(|identity| visible.iter().position(|candidate| candidate == identity))
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(visible.len().saturating_sub(1))
            };
            app.sdk_artifact_selection = Some(visible[next].clone());
        }
        Action::BeginSdkArtifactSearch => app.sdk_artifact_searching = true,
        Action::AppendSdkArtifactQuery(character) => {
            if app.sdk_artifact_searching
                && !character.is_control()
                && app.sdk_artifact_query.len() < 256
            {
                app.sdk_artifact_query.push(character);
                set_sdk_artifact_selection_to_current_or_first(
                    app,
                    app.sdk_artifact_selection.clone(),
                );
            }
        }
        Action::BackspaceSdkArtifactQuery => {
            if app.sdk_artifact_searching {
                app.sdk_artifact_query.pop();
                set_sdk_artifact_selection_to_current_or_first(
                    app,
                    app.sdk_artifact_selection.clone(),
                );
            }
        }
        Action::FinishSdkArtifactSearch => app.sdk_artifact_searching = false,
        Action::OpenSelectedSdkArtifact => {
            if let Some(path) = app
                .selected_sdk_artifact()
                .map(|artifact| artifact.identity.path.clone())
            {
                return Some(Effect::OpenInEditor(path));
            }
            app.notification = Some("No SDK artifact is selected.".into());
        }
        Action::SdkToolCapabilityLoaded(capability) => app.sdk_tool_capability = capability,
        Action::BeginSelectedSdkPublish => {
            let Some(artifact) = app.selected_sdk_artifact() else {
                app.notification = Some("Select an SDK artifact to publish.".into());
                return None;
            };
            if artifact.kind != SdkArtifactKind::Installer {
                app.notification = Some("Only an SDK installer can be published.".into());
                return None;
            }
            if let Err(message) = app.sdk_tool_capability.publish_executable() {
                app.notification = Some(message.into());
                return None;
            }
            let mut editor = PopupEditor::new(popup_toml_document("destination", "", None));
            let _ = editor.select_toml_value("destination");
            open_dialog(app, Dialog::SdkPublishTomlEditor(editor));
        }
        Action::ToggleSdkPublishTomlEditor => {
            if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendSdkPublishTomlEditor(character) => {
            if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
                && editor.text.len() < 4_096
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceSdkPublishTomlEditor => {
            if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::AppendSdkPublishDestination(character) => {
            if let Some(Dialog::SdkPublish(draft)) = app.active_dialog_mut()
                && !character.is_control()
                && draft.destination.len() < 4_096
            {
                draft.destination.push(character);
            }
        }
        Action::BackspaceSdkPublishDestination => {
            if let Some(Dialog::SdkPublish(draft)) = app.active_dialog_mut() {
                draft.destination.pop();
            }
        }
        Action::PreviewSdkPublish => {
            if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog().cloned() {
                let destination = match popup_toml_value(&editor.text, "destination") {
                    Ok(value) => value,
                    Err(message) => {
                        app.notification = Some(message);
                        return None;
                    }
                };
                let Some(artifact) = app.selected_sdk_artifact() else {
                    app.notification = Some("The selected SDK artifact is stale.".into());
                    return None;
                };
                let preview = app
                    .sdk_tool_capability
                    .publish_executable()
                    .map_err(str::to_owned)
                    .and_then(|executable| {
                        SdkPublishPreview::new(
                            executable,
                            artifact.identity.clone(),
                            PathBuf::from(destination),
                        )
                        .map_err(str::to_owned)
                    });
                match preview {
                    Ok(preview) => replace_dialog(app, Dialog::SdkPublishConfirmation(preview)),
                    Err(message) => app.notification = Some(message),
                }
                return None;
            }
            let Some(Dialog::SdkPublish(draft)) = app.active_dialog().cloned() else {
                return None;
            };
            let Some(artifact) = app.selected_sdk_artifact() else {
                app.notification = Some("The selected SDK artifact is stale.".into());
                return None;
            };
            let preview = app
                .sdk_tool_capability
                .publish_executable()
                .map_err(str::to_owned)
                .and_then(|executable| {
                    SdkPublishPreview::new(
                        executable,
                        artifact.identity.clone(),
                        PathBuf::from(draft.destination),
                    )
                    .map_err(str::to_owned)
                });
            match preview {
                Ok(preview) => replace_dialog(app, Dialog::SdkPublishConfirmation(preview)),
                Err(message) => app.notification = Some(message),
            }
        }
        Action::CancelSdkPublish => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::SdkPublish(_) | Dialog::SdkPublishTomlEditor(_))
            ) {
                close_dialog(app);
            }
        }
        Action::CancelSdkPublishPreview => {
            if matches!(app.active_dialog(), Some(Dialog::SdkPublishConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmSdkPublish => {
            let Some(Dialog::SdkPublishConfirmation(preview)) = app.active_dialog().cloned() else {
                return None;
            };
            let valid = app.selected_sdk_artifact().is_some_and(|artifact| {
                artifact.kind == SdkArtifactKind::Installer
                    && artifact.identity == preview.request.artifact
            }) && app.sdk_tool_capability.publish_executable().as_ref()
                == Ok(&preview.request.executable);
            if !valid {
                app.notification = Some("The SDK publication preview is stale.".into());
                return None;
            }
            close_dialog(app);
            return queue_sdk_session(app, SdkOperation::Publish(preview.request));
        }
        Action::BeginSdkNative => {
            let mut editor = PopupEditor::new("mode = \"find-sysroot\"\nworkspace = \"\"\nrecipe = \"\"\ntool = \"\"\narguments = \"\"\n".into());
            let _ = editor.select_toml_value("mode");
            open_dialog(app, Dialog::SdkNativeTomlEditor(editor));
        }
        Action::ToggleSdkNativeTomlEditor => {
            if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendSdkNativeTomlEditor(character) => {
            if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
                && editor.text.len() < 8_192
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceSdkNativeTomlEditor => {
            if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::UpdateSdkNativeDraft(draft) => {
            if matches!(app.active_dialog(), Some(Dialog::SdkNative(_))) {
                replace_dialog(app, Dialog::SdkNative(SdkNativeDialog::new(draft)));
            }
        }
        Action::SelectSdkNativeField { delta } => {
            if let Some(Dialog::SdkNative(dialog)) = app.active_dialog_mut()
                && !dialog.editing
            {
                dialog.selected_field = dialog.selected_field.shifted(delta);
                dialog.validation_error = None;
            }
        }
        Action::ActivateSdkNativeField => {
            if let Some(Dialog::SdkNative(dialog)) = app.active_dialog_mut() {
                if dialog.selected_field == SdkNativeField::Mode {
                    dialog.cycle_mode();
                } else if dialog.selected_field == SdkNativeField::Tool
                    && dialog.draft.mode == SdkNativeMode::FindSysroot
                {
                    dialog.validation_error =
                        Some("Tool is not applicable in find-native-sysroot mode.".into());
                } else {
                    dialog.editing = true;
                    dialog.validation_error = None;
                }
            }
        }
        Action::CycleSdkNativeMode => {
            if let Some(Dialog::SdkNative(dialog)) = app.active_dialog_mut()
                && !dialog.editing
                && dialog.selected_field == SdkNativeField::Mode
            {
                dialog.cycle_mode();
            }
        }
        Action::AppendSdkNativeField(character) => {
            if let Some(Dialog::SdkNative(dialog)) = app.active_dialog_mut()
                && dialog.editing
                && !character.is_control()
            {
                let arguments = dialog.selected_field == SdkNativeField::Arguments;
                if let Some((text, bound)) = dialog.selected_text_mut()
                    && text.len() + character.len_utf8() <= bound
                {
                    text.push(character);
                    if arguments {
                        dialog.synchronize_arguments();
                    }
                    dialog.validation_error = None;
                }
            }
        }
        Action::BackspaceSdkNativeField => {
            if let Some(Dialog::SdkNative(dialog)) = app.active_dialog_mut()
                && dialog.editing
            {
                let arguments = dialog.selected_field == SdkNativeField::Arguments;
                if let Some((text, _)) = dialog.selected_text_mut() {
                    text.pop();
                    if arguments {
                        dialog.synchronize_arguments();
                    }
                    dialog.validation_error = None;
                }
            }
        }
        Action::FinishSdkNativeFieldEdit => {
            if let Some(Dialog::SdkNative(dialog)) = app.active_dialog_mut() {
                dialog.synchronize_arguments();
                dialog.editing = false;
            }
        }
        Action::PreviewSdkNative => {
            if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog().cloned() {
                let request = (|| {
                    let fields = popup_toml_fields(&editor.text)?;
                    let mode = match fields.get("mode").map(String::as_str) {
                        Some("find-sysroot") => SdkNativeMode::FindSysroot,
                        Some("run-native") => SdkNativeMode::RunNative,
                        _ => return Err("`mode` must be find-sysroot or run-native.".to_owned()),
                    };
                    let value = |key: &str| {
                        fields
                            .get(key)
                            .cloned()
                            .ok_or_else(|| format!("Missing `{key}`."))
                    };
                    let workspace = value("workspace")?;
                    let recipe = value("recipe")?;
                    let tool = value("tool")?;
                    let arguments = value("arguments")?
                        .split_ascii_whitespace()
                        .map(str::to_owned)
                        .collect();
                    let executable = app
                        .sdk_tool_capability
                        .executable_for(mode)
                        .map_err(str::to_owned)?;
                    Ok(SdkNativeRequest {
                        executable,
                        mode,
                        extracted_root: (!workspace.is_empty()).then(|| PathBuf::from(workspace)),
                        recipe,
                        tool: (mode == SdkNativeMode::RunNative).then_some(tool),
                        arguments,
                    })
                })();
                match request
                    .and_then(|request| SdkNativePreview::new(request).map_err(str::to_owned))
                {
                    Ok(preview) => replace_dialog(app, Dialog::SdkNativeConfirmation(preview)),
                    Err(message) => app.notification = Some(message),
                }
                return None;
            }
            let Some(Dialog::SdkNative(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            let draft = dialog.draft;
            let executable = app.sdk_tool_capability.executable_for(draft.mode);
            let request = executable.map(|executable| SdkNativeRequest {
                executable,
                mode: draft.mode,
                extracted_root: (!draft.extracted_root.is_empty())
                    .then(|| PathBuf::from(draft.extracted_root)),
                recipe: draft.recipe,
                tool: (draft.mode == SdkNativeMode::RunNative).then_some(draft.tool),
                arguments: draft.arguments,
            });
            match request.and_then(SdkNativePreview::new) {
                Ok(preview) => replace_dialog(app, Dialog::SdkNativeConfirmation(preview)),
                Err(message) => app.notification = Some(message.into()),
            }
        }
        Action::CancelSdkNative => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::SdkNative(_) | Dialog::SdkNativeTomlEditor(_))
            ) {
                close_dialog(app);
            }
        }
        Action::CancelSdkNativePreview => {
            if matches!(app.active_dialog(), Some(Dialog::SdkNativeConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmSdkNative => {
            let Some(Dialog::SdkNativeConfirmation(preview)) = app.active_dialog().cloned() else {
                return None;
            };
            if SdkNativePreview::new(preview.request.clone()).as_ref() != Ok(&preview)
                || app
                    .sdk_tool_capability
                    .executable_for(preview.request.mode)
                    .as_ref()
                    != Ok(&preview.request.executable)
            {
                app.notification = Some("The SDK native-tool preview is stale.".into());
                return None;
            }
            close_dialog(app);
            return queue_sdk_session(app, SdkOperation::Native(preview.request));
        }
        Action::SdkSessionStarting { id, started_at } => {
            let Some(job_id) = sdk_job_id(app, id) else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Queued], |job| {
                    job.status = BackgroundJobStatus::Starting;
                    job.started_at = Some(started_at);
                });
        }
        Action::SdkSessionRunning { id } => {
            let Some(job_id) = sdk_job_id(app, id) else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Starting], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
        }
        Action::AppendSdkSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        } => {
            let Some(job_id) = sdk_job_id(app, id) else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs.append_output(
                job_id,
                BackgroundJobOutputEntry {
                    severity: if stream == SdkOutputStream::Stderr {
                        Severity::Warning
                    } else {
                        Severity::Info
                    },
                    message: line,
                    source: if stream == SdkOutputStream::Stderr {
                        BackgroundJobOutputSource::Stderr
                    } else {
                        BackgroundJobOutputSource::Stdout
                    },
                    truncated,
                    timestamp,
                },
            );
        }
        Action::CompleteSdkSession {
            id,
            exit_code,
            artifacts,
            finished_at,
        } => {
            let Some(job_id) =
                mutate_sdk_session(app, id, |session| session.exit_code = Some(exit_code))
            else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Succeeded;
                    job.finished_at = Some(finished_at);
                    job.result = Some(BackgroundJobResult {
                        summary: "SDK operation completed".into(),
                        artifacts,
                    });
                },
            );
        }
        Action::FailSdkSession {
            id,
            message,
            exit_code,
            finished_at,
        } => {
            let Some(job_id) = mutate_sdk_session(app, id, |session| {
                session.exit_code = exit_code;
                session.error_detail = Some(message.clone());
            }) else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Queued,
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Failed;
                    job.finished_at = Some(finished_at);
                    job.error = Some(BackgroundJobError {
                        summary: "SDK operation failed".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::LoseSdkSession {
            id,
            message,
            finished_at,
        } => {
            let Some(job_id) = mutate_sdk_session(app, id, |session| {
                session.error_detail = Some(message.clone())
            }) else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Queued,
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Lost;
                    job.finished_at = Some(finished_at);
                    job.error = Some(BackgroundJobError {
                        summary: "SDK operation lost".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::BeginActiveSdkSessionCancellation => {
            if let Some(id) = app.active_sdk_session().map(|session| session.id) {
                open_dialog(app, Dialog::SdkCancellationConfirmation(id));
            } else if matches!(app.sdk_artifacts, SdkArtifactInventoryState::Loading { .. }) {
                return Some(Effect::CancelSdkArtifactOperation);
            } else {
                app.notification = Some("No managed SDK operation is active.".into());
            }
        }
        Action::ConfirmSdkSessionCancellation => {
            let Some(Dialog::SdkCancellationConfirmation(id)) = app.active_dialog().cloned() else {
                return None;
            };
            let Some(job_id) = sdk_job_id(app, id) else {
                note_stale_sdk_event(app);
                close_dialog(app);
                return None;
            };
            let before = app.background_jobs.get(job_id).map(|job| job.status);
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Queued,
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                ],
                |job| job.status = BackgroundJobStatus::Cancelling,
            );
            close_dialog(app);
            if before != app.background_jobs.get(job_id).map(|job| job.status) {
                return Some(Effect::CancelSdkSession(id));
            }
        }
        Action::CancelSdkSessionCancellation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::SdkCancellationConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::RejectSdkSessionCancellation { id, message } => {
            let Some(job_id) = sdk_job_id(app, id) else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Running;
                    job.error = Some(BackgroundJobError {
                        summary: "SDK cancellation was rejected".into(),
                        detail: Some(message.clone()),
                    });
                });
            app.notification = Some(message);
        }
        Action::CancelSdkSession {
            id,
            exit_code,
            finished_at,
        } => {
            let Some(job_id) = mutate_sdk_session(app, id, |session| session.exit_code = exit_code)
            else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Cancelled;
                    job.finished_at = Some(finished_at);
                    job.result = Some(BackgroundJobResult {
                        summary: "SDK operation cancelled".into(),
                        artifacts: Vec::new(),
                    });
                });
        }
        Action::InspectTestCapability => {
            app.test_capability = TestCapability::default();
            return Some(Effect::InspectTestCapability);
        }
        Action::TestCapabilityLoaded(capability) => {
            app.test_capability = capability;
            if app.screen == Screen::Testing
                && matches!(
                    app.result_tool_capability,
                    ResultToolCapability::NotInspected
                )
            {
                return Some(Effect::InspectResultToolCapability);
            }
        }
        Action::SelectTestFamily { delta } => {
            app.test_family_selection = app.test_family_selection.shifted(delta);
        }
        Action::BeginSelectedTestLaunch => {
            let draft = test_launch_draft(app, app.test_family_selection);
            let mut editor = PopupEditor::new(format!(
                "# family, machine, distro, and image are authoritative\nfamily = \"{}\"\nmachine = \"{}\"\ndistro = \"{}\"\nimage = \"{}\"\nscope = \"all\"\nselector = \"\"\nparallelism = 1\nverbose = false\nskip_network = false\n",
                draft.family.label(),
                draft.machine,
                draft.distro,
                draft.image
            ));
            let _ = editor.select_toml_value("scope");
            open_dialog(
                app,
                Dialog::TestLaunchTomlEditor {
                    editor,
                    validation_error: None,
                },
            );
        }
        Action::ToggleTestLaunchTomlEditor => {
            if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendTestLaunchTomlEditor(character) => {
            if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
                && editor.text.len() + character.len_utf8() <= 16_384
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceTestLaunchTomlEditor => {
            if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::UpdateTestLaunchDraft(draft) => {
            if matches!(app.active_dialog(), Some(Dialog::TestLaunch(_))) {
                replace_dialog(app, Dialog::TestLaunch(TestLaunchDialog::new(draft)));
            }
        }
        Action::SelectTestLaunchField { delta } => {
            if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog_mut()
                && !dialog.editing
            {
                dialog.select(delta);
            }
        }
        Action::ActivateTestLaunchField => {
            if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog_mut() {
                dialog.activate();
            }
        }
        Action::AppendTestLaunchField(character) => {
            if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog_mut() {
                dialog.append(character);
            }
        }
        Action::BackspaceTestLaunchField => {
            if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog_mut() {
                dialog.backspace();
            }
        }
        Action::FinishTestLaunchFieldEdit => {
            if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog_mut() {
                dialog.finish_edit();
            }
        }
        Action::PreviewTestLaunch => {
            if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog().cloned()
            {
                let preview = (|| {
                    let fields = popup_toml_fields(&editor.text)?;
                    let get = |key: &str| {
                        fields
                            .get(key)
                            .cloned()
                            .ok_or_else(|| format!("Missing `{key}`."))
                    };
                    let family = match get("family")?.as_str() {
                        "OE selftest" => TestFamily::OeSelftest,
                        "BitBake selftest" => TestFamily::BitbakeSelftest,
                        "Image runtime" => TestFamily::TestImage,
                        "Standard SDK" => TestFamily::TestSdk,
                        "Extensible SDK" => TestFamily::TestSdkExt,
                        "Package tests" => TestFamily::Ptest,
                        _ => return Err("Unknown test family.".to_owned()),
                    };
                    let scope = match get("scope")?.as_str() {
                        "all" => TestSelectorScope::All,
                        "selected" => TestSelectorScope::Selected,
                        _ => return Err("`scope` must be all or selected.".to_owned()),
                    };
                    let parallelism = get("parallelism")?
                        .parse()
                        .map_err(|_| "`parallelism` must be a number.".to_owned())?;
                    let boolean = |key: &str| match get(key)?.as_str() {
                        "true" => Ok(true),
                        "false" => Ok(false),
                        _ => Err(format!("`{key}` must be true or false.")),
                    };
                    let draft = TestLaunchDraft {
                        family,
                        machine: get("machine")?,
                        distro: get("distro")?,
                        image: get("image")?,
                        scope,
                        selector: get("selector")?,
                        parallelism,
                        verbose: boolean("verbose")?,
                        skip_network: boolean("skip_network")?,
                    };
                    let authoritative = test_launch_draft(app, app.test_family_selection);
                    if draft.family != authoritative.family
                        || draft.machine != authoritative.machine
                        || draft.distro != authoritative.distro
                        || draft.image != authoritative.image
                    {
                        return Err(
                            "`family`, `machine`, `distro`, and `image` must match the current Testing context."
                                .to_owned(),
                        );
                    }
                    draft.preview(&app.test_capability).map_err(str::to_owned)
                })();
                match preview {
                    Ok(preview) => replace_dialog(app, Dialog::TestLaunchConfirmation(preview)),
                    Err(message) => {
                        if let Some(Dialog::TestLaunchTomlEditor {
                            validation_error, ..
                        }) = app.active_dialog_mut()
                        {
                            *validation_error = Some(message);
                        }
                    }
                }
                return None;
            }
            let Some(Dialog::TestLaunch(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            match dialog.draft.preview(&app.test_capability) {
                Ok(preview) => {
                    replace_dialog(app, Dialog::TestLaunchConfirmation(preview));
                }
                Err(message) => {
                    if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog_mut() {
                        dialog.validation_error = Some(message.into());
                    }
                }
            }
        }
        Action::CancelTestLaunch => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestLaunch(_) | Dialog::TestLaunchTomlEditor { .. })
            ) {
                close_dialog(app);
            }
        }
        Action::CancelTestLaunchPreview => {
            if matches!(app.active_dialog(), Some(Dialog::TestLaunchConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmTestLaunch => {
            let Some(Dialog::TestLaunchConfirmation(preview)) = app.active_dialog().cloned() else {
                return None;
            };
            if !test_preview_is_current(app, &preview) {
                app.notification = Some("The Testing launch preview is stale.".into());
                return None;
            }
            close_dialog(app);
            return queue_test_session(app, preview.operation());
        }
        Action::AttachTestBuildSession {
            id,
            background_job_id,
        } => {
            let valid = app.test_session(id).is_some_and(|session| {
                session.background_job_id.is_none()
                    && matches!(session.operation, TestOperation::Build { .. })
            }) && app
                .background_jobs
                .get(background_job_id)
                .is_some_and(|job| job.kind == BackgroundJobKind::Test);
            if !valid {
                note_stale_test_event(app);
                return None;
            }
            let _ = mutate_test_session(app, id, |session| {
                session.background_job_id = Some(background_job_id)
            });
        }
        Action::TestSessionStarting { id, started_at } => {
            let Some(job_id) = test_job_id(app, id) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Queued], |job| {
                    job.status = BackgroundJobStatus::Starting;
                    job.started_at = Some(started_at);
                });
        }
        Action::TestSessionRunning { id } => {
            let Some(job_id) = test_job_id(app, id) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Starting], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
        }
        Action::AppendTestSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        } => {
            let Some(job_id) = test_job_id(app, id) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs.append_output(
                job_id,
                BackgroundJobOutputEntry {
                    severity: if stream == TestOutputStream::Stderr {
                        Severity::Warning
                    } else {
                        Severity::Info
                    },
                    message: line,
                    source: if stream == TestOutputStream::Stderr {
                        BackgroundJobOutputSource::Stderr
                    } else {
                        BackgroundJobOutputSource::Stdout
                    },
                    truncated,
                    timestamp,
                },
            );
        }
        Action::CompleteTestSession {
            id,
            exit_code,
            result_paths,
            finished_at,
        } => {
            if !test_result_paths_are_valid(&result_paths) {
                note_stale_test_event(app);
                app.notification = Some("Testing returned invalid structured result paths.".into());
                return None;
            }
            let Some(Some(job_id)) = mutate_test_session(app, id, |session| {
                session.exit_code = Some(exit_code);
                session.result_paths.clone_from(&result_paths);
                session.outcome = Some(TestSessionOutcome::Succeeded);
            }) else {
                note_stale_test_event(app);
                return None;
            };
            let import_roots = result_paths.clone();
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Succeeded;
                    job.finished_at = Some(finished_at);
                    job.result = Some(BackgroundJobResult {
                        summary: "Testing operation completed".into(),
                        artifacts: result_paths,
                    });
                },
            );
            if !import_roots.is_empty()
                && app
                    .background_jobs
                    .get(job_id)
                    .is_some_and(|job| job.status == BackgroundJobStatus::Succeeded)
            {
                return begin_test_result_import(app, import_roots);
            }
        }
        Action::FailTestSession {
            id,
            message,
            exit_code,
            finished_at,
        } => {
            let Some(Some(job_id)) = mutate_test_session(app, id, |session| {
                session.exit_code = exit_code;
                session.error_detail = Some(message.clone());
                session.outcome = Some(TestSessionOutcome::Failed);
            }) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Queued,
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Failed;
                    job.finished_at = Some(finished_at);
                    job.error = Some(BackgroundJobError {
                        summary: "Testing operation failed".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::TimeoutTestSession {
            id,
            forced,
            exit_code,
            finished_at,
        } => {
            let detail = if forced {
                "Testing operation timed out and required forced termination"
            } else {
                "Testing operation timed out after graceful termination"
            };
            let Some(Some(job_id)) = mutate_test_session(app, id, |session| {
                session.exit_code = exit_code;
                session.error_detail = Some(detail.into());
                session.outcome = Some(TestSessionOutcome::TimedOut);
            }) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Queued,
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Failed;
                    job.finished_at = Some(finished_at);
                    job.error = Some(BackgroundJobError {
                        summary: "Testing operation timed out".into(),
                        detail: Some(detail.into()),
                    });
                },
            );
        }
        Action::LoseTestSession {
            id,
            message,
            finished_at,
        } => {
            let Some(Some(job_id)) = mutate_test_session(app, id, |session| {
                session.error_detail = Some(message.clone());
                session.outcome = Some(TestSessionOutcome::Lost);
            }) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Queued,
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Lost;
                    job.finished_at = Some(finished_at);
                    job.error = Some(BackgroundJobError {
                        summary: "Testing operation lost".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::BeginActiveTestSessionCancellation => {
            if let Some(id) = app.active_test_session().map(|session| session.id) {
                open_dialog(app, Dialog::TestCancellationConfirmation(id));
            } else {
                app.notification = Some("No managed Testing operation is active.".into());
            }
        }
        Action::ConfirmTestSessionCancellation => {
            let Some(Dialog::TestCancellationConfirmation(id)) = app.active_dialog().cloned()
            else {
                return None;
            };
            let Some(job_id) = test_job_id(app, id) else {
                note_stale_test_event(app);
                close_dialog(app);
                return None;
            };
            let before = app.background_jobs.get(job_id).map(|job| job.status);
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Queued,
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                ],
                |job| job.status = BackgroundJobStatus::Cancelling,
            );
            close_dialog(app);
            if before != app.background_jobs.get(job_id).map(|job| job.status) {
                return Some(Effect::CancelTestSession(id));
            }
        }
        Action::CancelTestSessionCancellation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestCancellationConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::RejectTestSessionCancellation { id, message } => {
            let Some(job_id) = test_job_id(app, id) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Running;
                    job.error = Some(BackgroundJobError {
                        summary: "Testing cancellation was rejected".into(),
                        detail: Some(message.clone()),
                    });
                });
            app.notification = Some(message);
        }
        Action::CancelTestSession {
            id,
            exit_code,
            finished_at,
        } => {
            let Some(Some(job_id)) = mutate_test_session(app, id, |session| {
                session.exit_code = exit_code;
                session.outcome = Some(TestSessionOutcome::Cancelled);
            }) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Cancelled;
                    job.finished_at = Some(finished_at);
                    job.result = Some(BackgroundJobResult {
                        summary: "Testing operation cancelled".into(),
                        artifacts: Vec::new(),
                    });
                });
        }
        Action::InspectResultToolCapability => {
            app.result_tool_capability = ResultToolCapability::NotInspected;
            return Some(Effect::InspectResultToolCapability);
        }
        Action::ResultToolCapabilityLoaded(capability) => {
            app.result_tool_capability = capability;
        }
        Action::CycleTestView => {
            app.test_view = app.test_view.next();
        }
        Action::BeginTestResultImport => {
            let mut editor = PopupEditor::new(popup_toml_document("root", "", None));
            let _ = editor.select_toml_value("root");
            open_dialog(
                app,
                Dialog::TestResultImportTomlEditor {
                    editor,
                    validation_error: None,
                },
            );
        }
        Action::ToggleTestResultImportTomlEditor => {
            if let Some(Dialog::TestResultImportTomlEditor { editor, .. }) = app.active_dialog_mut()
            {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendTestResultImportTomlEditor(character) => {
            if let Some(Dialog::TestResultImportTomlEditor {
                editor,
                validation_error,
            }) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
                && editor.text.len() + character.len_utf8() <= MAX_TEST_TEXT_BYTES
            {
                editor.insert(&character.to_string());
                *validation_error = None;
            }
        }
        Action::BackspaceTestResultImportTomlEditor => {
            if let Some(Dialog::TestResultImportTomlEditor {
                editor,
                validation_error,
            }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
                *validation_error = None;
            }
        }
        Action::AppendTestResultImport(character) => {
            if let Some(Dialog::TestResultImport(dialog)) = app.active_dialog_mut() {
                dialog.append(character);
            }
        }
        Action::BackspaceTestResultImport => {
            if let Some(Dialog::TestResultImport(dialog)) = app.active_dialog_mut() {
                dialog.backspace();
            }
        }
        Action::ConfirmTestResultImport => {
            if let Some(Dialog::TestResultImportTomlEditor { editor, .. }) =
                app.active_dialog().cloned()
            {
                let root = popup_toml_value(&editor.text, "root")
                    .map(PathBuf::from)
                    .and_then(|root| {
                        absolute_normal_path(&root).then_some(root).ok_or_else(|| {
                            "result import path must be normalized and absolute".to_owned()
                        })
                    });
                match root {
                    Ok(root) => {
                        close_dialog(app);
                        return begin_test_result_import(app, vec![root]);
                    }
                    Err(message) => {
                        if let Some(Dialog::TestResultImportTomlEditor {
                            validation_error, ..
                        }) = app.active_dialog_mut()
                        {
                            *validation_error = Some(message);
                        }
                    }
                }
                return None;
            }
            let Some(Dialog::TestResultImport(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            match dialog.root() {
                Ok(root) => {
                    close_dialog(app);
                    return begin_test_result_import(app, vec![root]);
                }
                Err(message) => {
                    if let Some(Dialog::TestResultImport(dialog)) = app.active_dialog_mut() {
                        dialog.validation_error = Some(message.into());
                    }
                }
            }
        }
        Action::CancelTestResultImport => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestResultImport(_) | Dialog::TestResultImportTomlEditor { .. })
            ) {
                close_dialog(app);
            }
        }
        Action::RefreshTestResults => {
            let Some(roots) = app
                .test_results
                .request()
                .map(|request| request.roots.clone())
            else {
                app.notification = Some("No validated test-result roots are retained.".into());
                return None;
            };
            return begin_test_result_import(app, roots);
        }
        Action::TestResultsLoaded {
            request,
            records,
            limitations,
        } => {
            if !test_result_request_is_current(app, &request)
                || records.iter().any(|record| !record.is_valid())
            {
                note_stale_test_event(app);
                return None;
            }
            let previous = app.test_result_selection.clone();
            let (records, limitations) = normalize_test_results(records, limitations);
            app.test_results = if records.is_empty() && limitations.is_empty() {
                TestResultInventoryState::AvailableEmpty { request }
            } else if limitations.is_empty() {
                TestResultInventoryState::Available { request, records }
            } else {
                TestResultInventoryState::Partial {
                    request,
                    records,
                    limitations,
                }
            };
            set_test_result_selection_to_current_or_first(app, previous);
            if app
                .test_comparison
                .request()
                .is_some_and(|request| !test_comparison_inputs_exist(app, request))
            {
                app.test_comparison = TestComparisonState::NotSelected;
                app.test_comparison_selection = None;
            }
        }
        Action::TestResultsFailed { request, message } => {
            if !test_result_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_results = TestResultInventoryState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Test result import failed: {message}"));
        }
        Action::TestResultsCancelled { request } => {
            if !test_result_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_results = TestResultInventoryState::Cancelled { request };
        }
        Action::TestResultsTimedOut { request } => {
            if !test_result_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_results = TestResultInventoryState::TimedOut { request };
        }
        Action::TestResultsLost { request, message } => {
            if !test_result_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_results = TestResultInventoryState::Lost {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Test result worker was lost: {message}"));
        }
        Action::SelectTestResult { delta } => {
            let visible = app
                .filtered_test_results()
                .into_iter()
                .map(|record| record.identity.clone())
                .collect::<Vec<_>>();
            if visible.is_empty() {
                app.test_result_selection = None;
                return None;
            }
            let current = app
                .test_result_selection
                .as_ref()
                .and_then(|identity| visible.iter().position(|candidate| candidate == identity))
                .unwrap_or_default();
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(visible.len() - 1)
            };
            app.test_result_selection = visible.get(next).cloned();
            app.test_result_drilled = false;
            app.test_case_selection = None;
        }
        Action::BeginTestResultSearch => app.test_result_searching = true,
        Action::AppendTestResultQuery(character) => {
            if app.test_result_searching
                && !character.is_control()
                && app.test_result_query.len() + character.len_utf8() <= MAX_TEST_TEXT_BYTES
            {
                app.test_result_query.push(character);
                let previous = app.test_result_selection.clone();
                set_test_result_selection_to_current_or_first(app, previous);
            }
        }
        Action::BackspaceTestResultQuery => {
            if app.test_result_searching {
                app.test_result_query.pop();
                let previous = app.test_result_selection.clone();
                set_test_result_selection_to_current_or_first(app, previous);
            }
        }
        Action::FinishTestResultSearch => app.test_result_searching = false,
        Action::OpenSelectedTestResult => {
            let Some(record) = app.selected_test_result() else {
                app.notification = Some("No exact test result is selected.".into());
                return None;
            };
            return Some(Effect::OpenInEditor(record.identity.path.clone()));
        }
        Action::DrillIntoSelectedTestResult => {
            let first = app.selected_test_result().and_then(|record| {
                record
                    .suites
                    .iter()
                    .flat_map(|suite| &suite.cases)
                    .next()
                    .map(|case| case.identity.clone())
            });
            if first.is_some() {
                app.test_result_drilled = true;
                app.test_case_selection = first;
            } else {
                app.notification = Some("The selected test result contains no cases.".into());
            }
        }
        Action::LeaveTestResultCases => {
            app.test_result_drilled = false;
            app.test_case_selection = None;
        }
        Action::SelectTestCase { delta } => {
            let identities = app.selected_test_result().map_or_else(Vec::new, |record| {
                record
                    .suites
                    .iter()
                    .flat_map(|suite| &suite.cases)
                    .map(|case| case.identity.clone())
                    .collect::<Vec<_>>()
            });
            if identities.is_empty() {
                app.test_case_selection = None;
                return None;
            }
            let current = app
                .test_case_selection
                .as_ref()
                .and_then(|identity| {
                    identities
                        .iter()
                        .position(|candidate| candidate == identity)
                })
                .unwrap_or_default();
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(identities.len() - 1)
            };
            app.test_case_selection = identities.get(next).cloned();
        }
        Action::OpenSelectedTestCaseLog => {
            let Some(path) = app
                .selected_test_case()
                .and_then(|case| case.log_path.clone())
            else {
                app.notification = Some("The selected test case has no exact log path.".into());
                return None;
            };
            return Some(Effect::OpenInEditor(path));
        }
        Action::BeginTestComparison => {
            let records = app.test_results.records();
            if records.len() < 2 {
                app.notification =
                    Some("At least two exact test results are required for comparison.".into());
                return None;
            }
            let picker = TestComparisonPicker::new(app.test_result_selection.clone(), records);
            let mut editor = PopupEditor::new(format!(
                "baseline = \"{}\"\ncandidate = \"{}\"\n",
                picker
                    .baseline
                    .as_ref()
                    .map_or_else(String::new, |value| value.path.display().to_string()),
                picker
                    .candidate
                    .as_ref()
                    .map_or_else(String::new, |value| value.path.display().to_string())
            ));
            let _ = editor.select_toml_value("baseline");
            open_dialog(
                app,
                Dialog::TestComparisonTomlEditor {
                    editor,
                    validation_error: None,
                },
            );
        }
        Action::ToggleTestComparisonTomlEditor => {
            if let Some(Dialog::TestComparisonTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendTestComparisonTomlEditor(character) => {
            if let Some(Dialog::TestComparisonTomlEditor {
                editor,
                validation_error,
            }) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
                && editor.text.len() + character.len_utf8() <= MAX_TEST_TEXT_BYTES
            {
                editor.insert(&character.to_string());
                *validation_error = None;
            }
        }
        Action::BackspaceTestComparisonTomlEditor => {
            if let Some(Dialog::TestComparisonTomlEditor {
                editor,
                validation_error,
            }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
                *validation_error = None;
            }
        }
        Action::SelectTestComparisonChoice { delta } => {
            let records = app.test_results.records().to_vec();
            if let Some(Dialog::TestComparison(dialog)) = app.active_dialog_mut() {
                dialog.select(&records, delta);
            }
        }
        Action::CycleTestComparisonField => {
            if let Some(Dialog::TestComparison(dialog)) = app.active_dialog_mut() {
                dialog.cycle_field();
            }
        }
        Action::ActivateTestComparisonChoice => {
            if let Some(Dialog::TestComparison(dialog)) = app.active_dialog_mut() {
                dialog.activate();
            }
        }
        Action::PreviewTestComparison => {
            if let Some(Dialog::TestComparisonTomlEditor { editor, .. }) =
                app.active_dialog().cloned()
            {
                let preview = (|| {
                    let fields = popup_toml_fields(&editor.text)?;
                    let lookup = |key: &str| {
                        let path = fields.get(key).ok_or_else(|| format!("Missing `{key}`."))?;
                        app.test_results
                            .records()
                            .iter()
                            .find(|record| record.identity.path == Path::new(path))
                            .map(|record| record.identity.clone())
                            .ok_or_else(|| format!("`{key}` is not an available test result."))
                    };
                    app.test_comparison_generation =
                        app.test_comparison_generation.wrapping_add(1).max(1);
                    let request = TestComparisonRequest::new(
                        app.test_comparison_generation,
                        lookup("baseline")?,
                        lookup("candidate")?,
                    )
                    .map_err(str::to_owned)?;
                    let executable = app
                        .result_tool_capability
                        .executable()
                        .map_err(str::to_owned)?;
                    TestComparisonPreview::new(executable, request).map_err(str::to_owned)
                })();
                match preview {
                    Ok(preview) => replace_dialog(app, Dialog::TestComparisonConfirmation(preview)),
                    Err(message) => {
                        if let Some(Dialog::TestComparisonTomlEditor {
                            validation_error, ..
                        }) = app.active_dialog_mut()
                        {
                            *validation_error = Some(message);
                        }
                    }
                }
                return None;
            }
            let Some(Dialog::TestComparison(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            app.test_comparison_generation = app.test_comparison_generation.wrapping_add(1).max(1);
            let preview = dialog
                .preview(app.test_comparison_generation)
                .and_then(|request| {
                    app.result_tool_capability
                        .executable()
                        .and_then(|executable| TestComparisonPreview::new(executable, request))
                });
            match preview {
                Ok(preview) => {
                    replace_dialog(app, Dialog::TestComparisonConfirmation(preview));
                }
                Err(message) => {
                    if let Some(Dialog::TestComparison(dialog)) = app.active_dialog_mut() {
                        dialog.validation_error = Some(message.into());
                    }
                }
            }
        }
        Action::CancelTestComparison => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestComparison(_) | Dialog::TestComparisonTomlEditor { .. })
            ) {
                close_dialog(app);
            }
        }
        Action::CancelTestComparisonPreview => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestComparisonConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::ConfirmTestComparison => {
            let Some(Dialog::TestComparisonConfirmation(preview)) = app.active_dialog().cloned()
            else {
                return None;
            };
            let request = preview.request;
            let valid = app
                .test_results
                .records()
                .iter()
                .any(|record| record.identity == request.baseline)
                && app
                    .test_results
                    .records()
                    .iter()
                    .any(|record| record.identity == request.candidate)
                && app
                    .result_tool_capability
                    .executable()
                    .is_ok_and(|executable| preview.argv.first() == Some(&executable));
            if !valid {
                app.notification = Some("The test comparison preview is stale.".into());
                return None;
            }
            close_dialog(app);
            app.test_comparison = TestComparisonState::Loading {
                request: request.clone(),
            };
            return Some(Effect::CompareTestResults(request));
        }
        Action::TestComparisonLoaded {
            request,
            comparison,
            limitations,
        } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            let baseline = app
                .test_results
                .records()
                .iter()
                .find(|record| record.identity == request.baseline);
            let candidate = app
                .test_results
                .records()
                .iter()
                .find(|record| record.identity == request.candidate);
            let Some(expected) = baseline.zip(candidate).and_then(|(baseline, candidate)| {
                TestComparison::between(baseline, candidate).ok()
            }) else {
                note_stale_test_event(app);
                return None;
            };
            if comparison != expected {
                note_stale_test_event(app);
                app.notification =
                    Some("Testing rejected an inconsistent comparison result.".into());
                return None;
            }
            let limitations = normalize_limitations(limitations);
            app.test_comparison = if limitations.is_empty() {
                TestComparisonState::Available {
                    request,
                    comparison,
                }
            } else {
                TestComparisonState::Partial {
                    request,
                    comparison,
                    limitations,
                }
            };
            set_test_comparison_selection(app);
        }
        Action::TestComparisonFailed { request, message } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_comparison = TestComparisonState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Test comparison failed: {message}"));
        }
        Action::TestComparisonCancelled { request } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_comparison = TestComparisonState::Cancelled { request };
        }
        Action::TestComparisonTimedOut { request } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_comparison = TestComparisonState::TimedOut { request };
        }
        Action::TestComparisonLost { request, message } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_comparison = TestComparisonState::Lost { request, message };
        }
        Action::SelectTestComparisonTransition { delta } => {
            let identities = app
                .test_comparison_transitions()
                .iter()
                .map(|transition| transition.identity.clone())
                .collect::<Vec<_>>();
            if identities.is_empty() {
                app.test_comparison_selection = None;
                return None;
            }
            let current = app
                .test_comparison_selection
                .as_ref()
                .and_then(|identity| {
                    identities
                        .iter()
                        .position(|candidate| candidate == identity)
                })
                .unwrap_or_default();
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(identities.len() - 1)
            };
            app.test_comparison_selection = identities.get(next).cloned();
        }
        Action::OpenSelectedTestTransitionLog => {
            let Some(path) = app.selected_test_transition().and_then(|transition| {
                transition
                    .candidate_log
                    .clone()
                    .or_else(|| transition.baseline_log.clone())
            }) else {
                app.notification =
                    Some("The selected comparison transition has no exact log path.".into());
                return None;
            };
            return Some(Effect::OpenInEditor(path));
        }
        Action::BeginTestJunitExport => {
            let Some(identity) = app
                .selected_test_result()
                .map(|record| record.identity.clone())
            else {
                app.notification = Some("No exact test result is selected for export.".into());
                return None;
            };
            if app.result_tool_capability.executable().is_err() {
                app.notification = Some("resulttool is unavailable for JUnit export.".into());
                return None;
            }
            let mut editor = PopupEditor::new(popup_toml_document("destination", "", None));
            let _ = editor.select_toml_value("destination");
            open_dialog(
                app,
                Dialog::TestJunitTomlEditor {
                    result: identity,
                    editor,
                    validation_error: None,
                },
            );
        }
        Action::ToggleTestJunitTomlEditor => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendTestJunitTomlEditor(character) => {
            if let Some(Dialog::TestJunitTomlEditor {
                editor,
                validation_error,
                ..
            }) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
                && editor.text.len() < MAX_TEST_TEXT_BYTES
            {
                editor.insert(&character.to_string());
                *validation_error = None;
            }
        }
        Action::BackspaceTestJunitTomlEditor => {
            if let Some(Dialog::TestJunitTomlEditor {
                editor,
                validation_error,
                ..
            }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
                *validation_error = None;
            }
        }
        Action::MoveTestJunitTomlEditorLeft => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.left();
            }
        }
        Action::MoveTestJunitTomlEditorRight => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.right();
            }
        }
        Action::MoveTestJunitTomlEditorUp => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.up();
            }
        }
        Action::MoveTestJunitTomlEditorDown => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.down();
            }
        }
        Action::MoveTestJunitTomlEditorHome => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.home();
            }
        }
        Action::MoveTestJunitTomlEditorEnd => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.end();
            }
        }
        Action::SelectTestJunitDestination => {
            if let Some(Dialog::TestJunitTomlEditor {
                editor,
                validation_error,
                ..
            }) = app.active_dialog_mut()
            {
                *validation_error = editor.select_toml_value("destination").err();
                editor.editing = validation_error.is_none();
            }
        }
        Action::CopyTestJunitTomlEditor => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                return Some(Effect::CopyToClipboard(editor.copy_selection_or_line()));
            }
        }
        Action::PasteTestJunitTomlEditor => {
            if let Some(Dialog::TestJunitTomlEditor {
                editor,
                validation_error,
                ..
            }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.paste();
                *validation_error = None;
            }
        }
        Action::AppendTestJunitDestination(character) => {
            if let Some(Dialog::TestJunitExport(dialog)) = app.active_dialog_mut() {
                dialog.append(character);
            }
        }
        Action::BackspaceTestJunitDestination => {
            if let Some(Dialog::TestJunitExport(dialog)) = app.active_dialog_mut() {
                dialog.backspace();
            }
        }
        Action::PreviewTestJunitExport => {
            if let Some(Dialog::TestJunitTomlEditor { result, editor, .. }) =
                app.active_dialog().cloned()
            {
                let destination = popup_toml_value(&editor.text, "destination")
                    .map(PathBuf::from)
                    .and_then(|path| {
                        (absolute_normal_path(&path)
                            && path.extension().and_then(|value| value.to_str()) == Some("xml"))
                        .then_some(path)
                        .ok_or_else(|| {
                            "JUnit destination must be a normalized absolute .xml path".to_owned()
                        })
                    });
                match destination {
                    Ok(destination) => {
                        app.test_junit_export = TestJunitExportState::Inspecting {
                            result: result.clone(),
                            destination: destination.clone(),
                        };
                        return Some(Effect::InspectTestJunitDestination {
                            result,
                            destination,
                        });
                    }
                    Err(message) => {
                        if let Some(Dialog::TestJunitTomlEditor {
                            validation_error, ..
                        }) = app.active_dialog_mut()
                        {
                            *validation_error = Some(message);
                        }
                    }
                }
                return None;
            }
            let Some(Dialog::TestJunitExport(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            match dialog.lexical_destination() {
                Ok(destination) => {
                    app.test_junit_export = TestJunitExportState::Inspecting {
                        result: dialog.result.clone(),
                        destination: destination.clone(),
                    };
                    return Some(Effect::InspectTestJunitDestination {
                        result: dialog.result,
                        destination,
                    });
                }
                Err(message) => {
                    if let Some(Dialog::TestJunitExport(dialog)) = app.active_dialog_mut() {
                        dialog.validation_error = Some(message.into());
                    }
                }
            }
        }
        Action::CancelTestJunitExport => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestJunitExport(_) | Dialog::TestJunitTomlEditor { .. })
            ) {
                close_dialog(app);
                app.test_junit_export = TestJunitExportState::NotStarted;
            }
        }
        Action::TestJunitDestinationInspected { result, inspection } => {
            let current = matches!(
                &app.test_junit_export,
                TestJunitExportState::Inspecting {
                    result: current_result,
                    destination,
                } if current_result == &result && destination == &inspection.requested
            ) && app
                .selected_test_result()
                .is_some_and(|record| record.identity == result)
                && matches!(app.active_dialog(),
                    Some(Dialog::TestJunitExport(dialog)) if dialog.result == result)
                || matches!(app.active_dialog(),
                        Some(Dialog::TestJunitTomlEditor { result: dialog_result, .. }) if *dialog_result == result);
            if !current {
                note_stale_test_event(app);
                return None;
            }
            app.test_junit_generation = app.test_junit_generation.wrapping_add(1).max(1);
            let preview =
                TestJunitExportRequest::new(app.test_junit_generation, result, &inspection)
                    .and_then(|request| {
                        app.result_tool_capability
                            .executable()
                            .and_then(|executable| TestJunitExportPreview::new(executable, request))
                    });
            match preview {
                Ok(preview) => {
                    app.test_junit_export = TestJunitExportState::Ready(preview.clone());
                    replace_dialog(app, Dialog::TestJunitExportConfirmation(preview));
                }
                Err(message) => {
                    app.test_junit_export = TestJunitExportState::NotStarted;
                    match app.active_dialog_mut() {
                        Some(Dialog::TestJunitExport(dialog)) => {
                            dialog.validation_error = Some(message.into())
                        }
                        Some(Dialog::TestJunitTomlEditor {
                            validation_error, ..
                        }) => *validation_error = Some(message.into()),
                        _ => {}
                    }
                }
            }
        }
        Action::CancelTestJunitExportPreview => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestJunitExportConfirmation(_))
            ) {
                close_dialog(app);
                app.test_junit_export = TestJunitExportState::NotStarted;
            }
        }
        Action::ConfirmTestJunitExport => {
            let Some(Dialog::TestJunitExportConfirmation(preview)) = app.active_dialog().cloned()
            else {
                return None;
            };
            let valid = matches!(
                &app.test_junit_export,
                TestJunitExportState::Ready(current) if current == &preview
            ) && app
                .test_results
                .records()
                .iter()
                .any(|record| record.identity == preview.request.result)
                && app
                    .result_tool_capability
                    .executable()
                    .is_ok_and(|executable| preview.argv.first() == Some(&executable));
            if !valid {
                app.notification = Some("The JUnit export preview is stale.".into());
                return None;
            }
            close_dialog(app);
            let request = preview.request;
            app.test_junit_export = TestJunitExportState::Running(request.clone());
            return Some(Effect::ExportTestJunit(request));
        }
        Action::TestJunitExportSucceeded { request } => {
            if !test_junit_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_junit_export = TestJunitExportState::Succeeded(request);
        }
        Action::TestJunitExportFailed { request, message } => {
            if !test_junit_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_junit_export = TestJunitExportState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("JUnit export failed: {message}"));
        }
        Action::TestJunitExportCancelled { request } => {
            if !test_junit_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_junit_export = TestJunitExportState::Cancelled(request);
        }
        Action::TestJunitExportTimedOut { request } => {
            if !test_junit_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_junit_export = TestJunitExportState::TimedOut(request);
        }
        Action::TestJunitExportLost { request, message } => {
            if !test_junit_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_junit_export = TestJunitExportState::Lost { request, message };
        }
        Action::InspectQemuCapability => {
            app.qemu_capability = QemuCapability::NotInspected;
            return Some(Effect::InspectQemuCapability);
        }
        Action::QemuCapabilityLoaded(capability) => {
            app.qemu_capability = capability;
        }
        Action::BeginSelectedQemuLaunch => {
            if let Some(reason) = app.qemu_launch_unavailable_reason() {
                app.notification = Some(reason);
                return None;
            }
            let artifact = app.selected_image_artifact().cloned()?;
            let draft = QemuLaunchDraft::for_artifact(artifact.identity, artifact.kind);
            open_dialog(app, Dialog::QemuLaunch(QemuLaunchDialog::new(draft)));
        }
        Action::UpdateQemuLaunchDraft(draft) => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut() {
                dialog.draft = draft;
                dialog.validation_error = None;
            } else {
                app.notification = Some("No runqemu launch draft is active.".into());
            }
        }
        Action::SelectQemuLaunchField { delta } => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut()
                && !dialog.editing
            {
                dialog.selected_field = dialog.selected_field.shifted(delta);
            }
        }
        Action::ActivateQemuLaunchField => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut() {
                if dialog.selected_field.is_read_only() {
                    app.notification = Some("Image and machine identity are read-only.".into());
                } else if dialog.selected_field.is_text() {
                    dialog.editing = true;
                    dialog.validation_error = None;
                } else if dialog.cycle_choice(false) {
                    dialog.validation_error = None;
                }
            }
        }
        Action::CycleQemuLaunchChoice { backwards } => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut()
                && !dialog.editing
                && dialog.cycle_choice(backwards)
            {
                dialog.validation_error = None;
            }
        }
        Action::AppendQemuLaunchField(character) => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut()
                && dialog.editing
                && !character.is_control()
                && let Some((input, maximum)) = dialog.selected_text_mut()
                && input.len() + character.len_utf8() <= maximum
            {
                input.push(character);
                dialog.validation_error = None;
            }
        }
        Action::BackspaceQemuLaunchField => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut()
                && dialog.editing
                && let Some((input, _)) = dialog.selected_text_mut()
            {
                input.pop();
                dialog.validation_error = None;
            }
        }
        Action::FinishQemuLaunchFieldEdit => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut() {
                dialog.editing = false;
            }
        }
        Action::PreviewQemuLaunch => {
            let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog().cloned() else {
                app.notification = Some("No runqemu launch draft is active.".into());
                return None;
            };
            match dialog.draft.preview(&app.qemu_capability) {
                Ok(preview) => replace_dialog(app, Dialog::QemuLaunchConfirmation(preview)),
                Err(message) => {
                    if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut() {
                        dialog.validation_error = Some(message.into());
                    }
                    app.notification = Some(message.into());
                }
            }
        }
        Action::CancelQemuLaunch => {
            if matches!(app.active_dialog(), Some(Dialog::QemuLaunch(_))) {
                close_dialog(app);
            }
        }
        Action::CancelQemuLaunchPreview => {
            if matches!(app.active_dialog(), Some(Dialog::QemuLaunchConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmQemuLaunch => {
            let Some(Dialog::QemuLaunchConfirmation(preview)) = app.active_dialog().cloned() else {
                app.notification = Some("No runqemu launch is awaiting confirmation.".into());
                return None;
            };
            if app.active_qemu_session().is_some() {
                app.notification = Some("A managed runqemu session is already active.".into());
                return None;
            }
            if preview.request.validate().is_err()
                || app
                    .qemu_capability
                    .executable_for(&preview.request.image)
                    .is_err()
            {
                app.notification =
                    Some("The runqemu launch preview is no longer valid; review it again.".into());
                return None;
            }
            while app.qemu_sessions.len() >= MAX_QEMU_SESSIONS {
                let Some(index) = app.qemu_sessions.iter().position(|session| {
                    app.background_jobs
                        .get(session.background_job_id)
                        .is_none_or(|job| job.status.is_terminal())
                }) else {
                    app.notification = Some("The runqemu session history is full.".into());
                    return None;
                };
                app.qemu_sessions.remove(index);
            }
            let id = next_qemu_session_id(app);
            let background_job_id = qemu_background_job_id(id);
            let request = preview.request;
            app.background_jobs.queue(BackgroundJobSpec {
                id: background_job_id,
                kind: BackgroundJobKind::Qemu,
                title: format!("runqemu {}", request.image.image),
                context: BackgroundJobContext {
                    workspace: Some(Screen::Images),
                    target: Some(request.image.image.clone()),
                    image: Some(request.image.image.clone()),
                    path: Some(request.image.path.clone()),
                    ..BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at: SystemTime::now(),
            });
            if app.background_jobs.get(background_job_id).is_none() {
                app.notification = Some("The runqemu session could not be queued.".into());
                return None;
            }
            app.qemu_sessions.push_back(QemuSession {
                id,
                background_job_id,
                request: request.clone(),
                exit_code: None,
                error_detail: None,
            });
            close_dialog(app);
            return Some(Effect::StartQemuSession { id, request });
        }
        Action::QemuSessionStarting { id, started_at } => {
            let Some(job_id) = qemu_job_id(app, id) else {
                note_stale_qemu_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Queued], |job| {
                    job.status = BackgroundJobStatus::Starting;
                    job.started_at = Some(started_at);
                });
        }
        Action::QemuSessionRunning { id } => {
            let Some(job_id) = qemu_job_id(app, id) else {
                note_stale_qemu_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Starting], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
        }
        Action::AppendQemuSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        } => {
            let Some(job_id) = qemu_job_id(app, id) else {
                note_stale_qemu_event(app);
                return None;
            };
            app.background_jobs.append_output(
                job_id,
                BackgroundJobOutputEntry {
                    severity: if stream == QemuOutputStream::Stderr {
                        Severity::Warning
                    } else {
                        Severity::Info
                    },
                    message: line,
                    source: match stream {
                        QemuOutputStream::Stdout => BackgroundJobOutputSource::Stdout,
                        QemuOutputStream::Stderr => BackgroundJobOutputSource::Stderr,
                    },
                    truncated,
                    timestamp,
                },
            );
        }
        Action::CompleteQemuSession {
            id,
            exit_code,
            finished_at,
        } => {
            if !matches!(
                qemu_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Running,
                    ..
                })
            ) {
                note_stale_qemu_event(app);
                return None;
            }
            let Some(job_id) =
                mutate_qemu_session(app, id, |session| session.exit_code = Some(exit_code))
            else {
                note_stale_qemu_event(app);
                return None;
            };
            if exit_code == 0 {
                app.background_jobs
                    .update_if(job_id, &[BackgroundJobStatus::Running], |job| {
                        job.status = BackgroundJobStatus::Succeeded;
                        job.finished_at = Some(finished_at);
                        job.result = Some(BackgroundJobResult {
                            summary: "runqemu exited successfully".into(),
                            artifacts: Vec::new(),
                        });
                    });
            } else {
                let detail = format!("exit code {exit_code}");
                let _ = mutate_qemu_session(app, id, |session| {
                    session.error_detail = Some(detail.clone())
                });
                app.background_jobs
                    .update_if(job_id, &[BackgroundJobStatus::Running], |job| {
                        job.status = BackgroundJobStatus::Failed;
                        job.finished_at = Some(finished_at);
                        job.error = Some(BackgroundJobError {
                            summary: "runqemu failed".into(),
                            detail: Some(detail),
                        });
                    });
            }
        }
        Action::FailQemuSession {
            id,
            message,
            exit_code,
            finished_at,
        } => {
            if !matches!(
                qemu_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Queued
                        | BackgroundJobStatus::Starting
                        | BackgroundJobStatus::Running
                        | BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_qemu_event(app);
                return None;
            }
            let Some(job_id) = mutate_qemu_session(app, id, |session| {
                session.exit_code = exit_code;
                session.error_detail = Some(message.clone());
            }) else {
                note_stale_qemu_event(app);
                return None;
            };
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Queued,
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Failed;
                    job.finished_at = Some(finished_at);
                    job.error = Some(BackgroundJobError {
                        summary: "runqemu failed".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::LoseQemuSession {
            id,
            message,
            finished_at,
        } => {
            if !matches!(
                qemu_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Starting
                        | BackgroundJobStatus::Running
                        | BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_qemu_event(app);
                return None;
            }
            let Some(job_id) = mutate_qemu_session(app, id, |session| {
                session.error_detail = Some(message.clone())
            }) else {
                note_stale_qemu_event(app);
                return None;
            };
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Lost;
                    job.finished_at = Some(finished_at);
                    job.error = Some(BackgroundJobError {
                        summary: "runqemu process was lost".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::BeginQemuSessionCancellation { id } => {
            let Some(session) = app.qemu_session(id) else {
                app.notification = Some("The runqemu session no longer exists.".into());
                return None;
            };
            let cancellable = app
                .background_jobs
                .get(session.background_job_id)
                .is_some_and(|job| {
                    job.cancellation_supported
                        && matches!(
                            job.status,
                            BackgroundJobStatus::Queued
                                | BackgroundJobStatus::Starting
                                | BackgroundJobStatus::Running
                        )
                });
            if cancellable {
                open_dialog(app, Dialog::QemuCancellationConfirmation(id));
            } else {
                app.notification = Some("The runqemu session cannot be cancelled.".into());
            }
        }
        Action::BeginActiveQemuSessionCancellation => {
            let Some(id) = app.active_qemu_session().map(|session| session.id) else {
                app.notification = Some("No managed runqemu session is active.".into());
                return None;
            };
            let _ = update(app, Action::BeginQemuSessionCancellation { id });
        }
        Action::ConfirmQemuSessionCancellation => {
            let Some(Dialog::QemuCancellationConfirmation(id)) = app.active_dialog().cloned()
            else {
                app.notification = Some("No runqemu cancellation is awaiting confirmation.".into());
                return None;
            };
            let Some(job_id) = qemu_job_id(app, id) else {
                note_stale_qemu_event(app);
                return None;
            };
            let before = app.background_jobs.get(job_id).map(|job| job.status);
            app.background_jobs.request_cancellation(job_id);
            close_dialog(app);
            if before
                == Some(
                    app.background_jobs
                        .get(job_id)
                        .map_or(BackgroundJobStatus::Lost, |job| job.status),
                )
            {
                app.notification = Some("The runqemu cancellation request was rejected.".into());
                return None;
            }
            return Some(Effect::CancelQemuSession(id));
        }
        Action::CancelQemuSessionCancellation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::QemuCancellationConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::RejectQemuSessionCancellation { id, message } => {
            let Some(job_id) = qemu_job_id(app, id) else {
                note_stale_qemu_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
            app.notification = Some(format!("runqemu cancellation failed: {message}"));
        }
        Action::CancelQemuSession {
            id,
            exit_code,
            finished_at,
        } => {
            if !matches!(
                qemu_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_qemu_event(app);
                return None;
            }
            let Some(job_id) =
                mutate_qemu_session(app, id, |session| session.exit_code = exit_code)
            else {
                note_stale_qemu_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Cancelled;
                    job.finished_at = Some(finished_at);
                });
        }
        Action::InspectWicCapability => {
            app.wic_capability = WicCapability::NotInspected;
            return Some(Effect::InspectWicCapability);
        }
        Action::WicCapabilityLoaded(capability) => {
            app.wic_capability = normalize_wic_capability(capability);
        }
        Action::BeginSelectedWicCreate => {
            if let Some(reason) = app.wic_create_unavailable_reason() {
                app.notification = Some(reason);
                return None;
            }
            let artifact = app
                .selected_image_artifact()
                .cloned()
                .expect("checked above");
            let WicCapability::Available { kickstarts, .. } = &app.wic_capability else {
                unreachable!("checked above")
            };
            let kickstart = app
                .workspace
                .variables
                .get("WKS_FILE")
                .and_then(|configured| {
                    let configured_path = Path::new(configured);
                    kickstarts.iter().find(|kickstart| {
                        kickstart.identity.name == *configured
                            || kickstart.identity.path.as_deref() == Some(configured_path)
                    })
                })
                .unwrap_or(&kickstarts[0])
                .identity
                .clone();
            let output_directory = artifact
                .identity
                .path
                .parent()
                .unwrap_or(Path::new("/"))
                .display()
                .to_string();
            let mut editor = PopupEditor::new(format!(
                "# machine is authoritative and read-only\nmachine = \"{}\"\nimage = \"{}\"\nkickstart = \"{}\"\noutput_directory = \"{}\"\ngenerate_bmap = true\ncompression = \"none\"\n",
                artifact.identity.machine,
                artifact.identity.image,
                kickstart.name,
                output_directory,
            ));
            let _ = editor.select_toml_value("output_directory");
            open_dialog(
                app,
                Dialog::WicCreateTomlEditor {
                    editor,
                    validation_error: None,
                },
            );
        }
        Action::ToggleWicCreateTomlEditor => {
            if let Some(Dialog::WicCreateTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendWicCreateTomlEditor(character) => {
            if let Some(Dialog::WicCreateTomlEditor {
                editor,
                validation_error,
            }) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
            {
                editor.insert(&character.to_string());
                *validation_error = None;
            }
        }
        Action::BackspaceWicCreateTomlEditor => {
            if let Some(Dialog::WicCreateTomlEditor {
                editor,
                validation_error,
            }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
                *validation_error = None;
            }
        }
        Action::SelectWicCreateField { delta } => {
            if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut()
                && !dialog.editing
            {
                dialog.selected_field = dialog.selected_field.shifted(delta);
            }
        }
        Action::ActivateWicCreateField => {
            let capability = app.wic_capability.clone();
            if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut() {
                if dialog.selected_field.is_read_only() {
                    app.notification = Some("Wic machine identity is read-only.".into());
                } else if dialog.selected_field == WicCreateField::OutputDirectory {
                    dialog.editing = true;
                    dialog.validation_error = None;
                } else if dialog.cycle_choice(&capability, false) {
                    dialog.validation_error = None;
                }
            }
        }
        Action::CycleWicCreateChoice { backwards } => {
            let capability = app.wic_capability.clone();
            if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut()
                && !dialog.editing
                && dialog.cycle_choice(&capability, backwards)
            {
                dialog.validation_error = None;
            }
        }
        Action::AppendWicCreateField(character) => {
            if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut()
                && dialog.editing
                && !character.is_control()
                && let Some((input, maximum)) = dialog.selected_text_mut()
                && input.len() + character.len_utf8() <= maximum
            {
                input.push(character);
                dialog.validation_error = None;
            }
        }
        Action::BackspaceWicCreateField => {
            if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut()
                && dialog.editing
                && let Some((input, _)) = dialog.selected_text_mut()
            {
                input.pop();
                dialog.validation_error = None;
            }
        }
        Action::FinishWicCreateFieldEdit => {
            if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut() {
                dialog.editing = false;
            }
        }
        Action::PreviewWicCreate => {
            if let Some(Dialog::WicCreateTomlEditor { editor, .. }) = app.active_dialog().cloned() {
                let result = (|| {
                    let fields = popup_toml_fields(&editor.text)?;
                    let machine = fields.get("machine").cloned().ok_or("Missing `machine`.")?;
                    let image = fields.get("image").cloned().ok_or("Missing `image`.")?;
                    let kickstart_name = fields
                        .get("kickstart")
                        .cloned()
                        .ok_or("Missing `kickstart`.")?;
                    let output_directory = fields
                        .get("output_directory")
                        .cloned()
                        .ok_or("Missing `output_directory`.")?;
                    let generate_bmap = match fields.get("generate_bmap").map(String::as_str) {
                        Some("true") => true,
                        Some("false") => false,
                        _ => return Err("`generate_bmap` must be true or false.".to_owned()),
                    };
                    let compression = match fields.get("compression").map(String::as_str) {
                        Some("none") => WicCompression::None,
                        Some("gzip") => WicCompression::Gzip,
                        Some("bzip2") => WicCompression::Bzip2,
                        Some("xz") => WicCompression::Xz,
                        _ => {
                            return Err(
                                "`compression` must be none, gzip, bzip2, or xz.".to_owned()
                            );
                        }
                    };
                    let expected_machine = app
                        .selected_image_artifact()
                        .map(|artifact| artifact.identity.machine.as_str())
                        .ok_or_else(|| "The selected image artifact is unavailable.".to_owned())?;
                    if machine != expected_machine {
                        return Err(
                            "`machine` is authoritative and cannot differ from the selected image."
                                .to_owned(),
                        );
                    }
                    let WicCapability::Available { kickstarts, .. } = &app.wic_capability else {
                        return Err("Wic capability is not available.".to_owned());
                    };
                    let kickstart = kickstarts
                        .iter()
                        .find(|candidate| candidate.identity.name == kickstart_name)
                        .map(|candidate| candidate.identity.clone())
                        .ok_or_else(|| "The selected kickstart is unavailable.".to_owned())?;
                    WicCreateDraft {
                        machine,
                        image,
                        kickstart,
                        output_directory,
                        generate_bmap,
                        compression,
                    }
                    .preview(&app.wic_capability)
                    .map_err(str::to_owned)
                })();
                match result {
                    Ok(preview) => replace_dialog(app, Dialog::WicCreateConfirmation(preview)),
                    Err(message) => {
                        if let Some(Dialog::WicCreateTomlEditor {
                            validation_error, ..
                        }) = app.active_dialog_mut()
                        {
                            *validation_error = Some(message.clone());
                        }
                        app.notification = Some(message);
                    }
                }
                return None;
            }
            let Some(Dialog::WicCreate(dialog)) = app.active_dialog().cloned() else {
                app.notification = Some("No Wic creation draft is active.".into());
                return None;
            };
            match dialog.draft.preview(&app.wic_capability) {
                Ok(preview) => replace_dialog(app, Dialog::WicCreateConfirmation(preview)),
                Err(message) => {
                    if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut() {
                        dialog.validation_error = Some(message.into());
                    }
                    app.notification = Some(message.into());
                }
            }
        }
        Action::CancelWicCreate => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::WicCreate(_) | Dialog::WicCreateTomlEditor { .. })
            ) {
                close_dialog(app);
            }
        }
        Action::CancelWicCreatePreview => {
            if matches!(app.active_dialog(), Some(Dialog::WicCreateConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmWicCreate => {
            let Some(Dialog::WicCreateConfirmation(preview)) = app.active_dialog().cloned() else {
                app.notification = Some("No Wic creation is awaiting confirmation.".into());
                return None;
            };
            close_dialog(app);
            return update(app, Action::StartConfirmedWicCreate(preview));
        }
        Action::SelectWicOutput { delta } => {
            let rows = app.wic_output_rows();
            if rows.is_empty() {
                app.wic_output_selection = None;
                return None;
            }
            let current = app
                .wic_output_selection
                .as_ref()
                .and_then(|selected| rows.iter().position(|row| &row.identity == selected))
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current.saturating_add(delta as usize).min(rows.len() - 1)
            };
            app.wic_output_selection = Some(rows[next].identity.clone());
        }
        Action::OpenSelectedWicOutput => {
            let Some(output) = app.selected_wic_output() else {
                app.notification = Some("Select a generated Wic output first.".into());
                return None;
            };
            return Some(Effect::OpenInEditor(output.identity.path.clone()));
        }
        Action::BeginActiveWicSessionCancellation => {
            let Some((id, incomplete_device_warning)) = app.active_wic_session().map(|session| {
                (
                    session.id,
                    matches!(session.operation, WicOperation::Write(_)),
                )
            }) else {
                app.notification = Some("No managed Wic operation is active.".into());
                return None;
            };
            open_dialog(
                app,
                Dialog::WicCancellationConfirmation {
                    id,
                    incomplete_device_warning,
                },
            );
        }
        Action::BeginActiveImageRuntimeCancellation => {
            if app.active_wic_session().is_some() {
                return update(app, Action::BeginActiveWicSessionCancellation);
            }
            return update(app, Action::BeginActiveQemuSessionCancellation);
        }
        Action::CancelWicSessionCancellation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::WicCancellationConfirmation { .. })
            ) {
                close_dialog(app);
            }
        }
        Action::BeginWicOutputInventory(request) => {
            if let Err(message) = request.validate() {
                app.notification = Some(format!("Wic outputs are unavailable: {message}."));
                return None;
            }
            app.wic_output_generation = app.wic_output_generation.max(request.generation);
            app.wic_outputs = WicOutputInventoryState::Loading {
                request: request.clone(),
            };
            return Some(Effect::GetWicOutputs(request));
        }
        Action::WicOutputInventoryLoaded {
            request,
            outputs,
            limitations,
        } => {
            if !matches!(
                &app.wic_outputs,
                WicOutputInventoryState::Loading { request: active }
                    if active == &request
            ) {
                note_stale_wic_event(app);
                return None;
            }
            match normalize_wic_outputs(&request.output_directory, outputs) {
                Ok(outputs) => {
                    let limitations = normalize_wic_limitations(limitations);
                    app.wic_outputs = if limitations.is_empty() {
                        WicOutputInventoryState::Available { request, outputs }
                    } else {
                        WicOutputInventoryState::Partial {
                            request,
                            outputs,
                            limitations,
                        }
                    };
                }
                Err(message) => {
                    app.wic_outputs = WicOutputInventoryState::Failed {
                        request,
                        message: message.into(),
                    };
                }
            }
            reconcile_wic_output_selection(app);
        }
        Action::WicOutputInventoryFailed { request, message } => {
            if !matches!(
                &app.wic_outputs,
                WicOutputInventoryState::Loading { request: active }
                    if active == &request
            ) {
                note_stale_wic_event(app);
                return None;
            }
            app.wic_outputs = WicOutputInventoryState::Failed { request, message };
        }
        Action::BeginSelectedWicDeviceWrite => {
            if let Some(reason) = app.wic_device_write_unavailable_reason() {
                app.notification = Some(reason);
                return None;
            }
            let image = app
                .selected_wic_write_image()
                .expect("availability checked above");
            app.wic_device_generation = app.wic_device_generation.wrapping_add(1).max(1);
            let request = WicDeviceInventoryRequest {
                generation: app.wic_device_generation,
                image,
            };
            if let Err(message) = request.validate() {
                app.notification = Some(format!("Wic devices are unavailable: {message}."));
                return None;
            }
            let preserve_selection = matches!(
                &app.wic_devices,
                WicDeviceInventoryState::Loading { request: active }
                    | WicDeviceInventoryState::Available {
                        request: active,
                        ..
                    }
                    | WicDeviceInventoryState::Partial {
                        request: active,
                        ..
                    }
                    | WicDeviceInventoryState::Failed {
                        request: active,
                        ..
                    } if active.image == request.image
            );
            if !preserve_selection {
                app.wic_device_selection = None;
            }
            app.wic_devices = WicDeviceInventoryState::Loading {
                request: request.clone(),
            };
            open_dialog(
                app,
                Dialog::WicDevicePicker(WicDevicePickerDialog {
                    request: request.clone(),
                }),
            );
            synchronize_focus(app);
            return Some(Effect::GetWicDevices(request));
        }
        Action::BeginWicDeviceInventory(request) => {
            if let Err(message) = request.validate() {
                app.notification = Some(format!("Wic devices are unavailable: {message}."));
                return None;
            }
            app.wic_device_generation = app.wic_device_generation.max(request.generation);
            app.wic_devices = WicDeviceInventoryState::Loading {
                request: request.clone(),
            };
            return Some(Effect::GetWicDevices(request));
        }
        Action::WicDeviceInventoryLoaded {
            request,
            devices,
            limitations,
        } => {
            if !matches!(
                &app.wic_devices,
                WicDeviceInventoryState::Loading { request: active }
                    if active == &request
            ) {
                note_stale_wic_event(app);
                return None;
            }
            let devices = normalize_wic_devices(devices);
            let limitations = normalize_wic_limitations(limitations);
            app.wic_devices = if limitations.is_empty() {
                WicDeviceInventoryState::Available { request, devices }
            } else {
                WicDeviceInventoryState::Partial {
                    request,
                    devices,
                    limitations,
                }
            };
            reconcile_wic_device_selection(app);
        }
        Action::WicDeviceInventoryFailed { request, message } => {
            if !matches!(
                &app.wic_devices,
                WicDeviceInventoryState::Loading { request: active }
                    if active == &request
            ) {
                note_stale_wic_event(app);
                return None;
            }
            app.wic_devices = WicDeviceInventoryState::Failed { request, message };
            app.wic_device_selection = None;
        }
        Action::SelectWicDevice { delta } => {
            let Some(Dialog::WicDevicePicker(dialog)) = app.active_dialog() else {
                return None;
            };
            let request_matches = matches!(
                &app.wic_devices,
                WicDeviceInventoryState::Available { request, .. }
                    | WicDeviceInventoryState::Partial { request, .. }
                    if request == &dialog.request
            );
            if !request_matches {
                return None;
            }
            let rows = app.wic_device_rows();
            if rows.is_empty() {
                app.wic_device_selection = None;
                return None;
            }
            let current = app
                .wic_device_selection
                .as_ref()
                .and_then(|selected| rows.iter().position(|row| &row.identity == selected))
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current.saturating_add(delta as usize).min(rows.len() - 1)
            };
            app.wic_device_selection = Some(rows[next].identity.clone());
        }
        Action::ConfirmWicDeviceSelection => {
            let Some(Dialog::WicDevicePicker(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            let Some(device) = app.selected_wic_device() else {
                app.notification = Some("No eligible Wic device is available to select.".into());
                return None;
            };
            let request_matches = matches!(
                &app.wic_devices,
                WicDeviceInventoryState::Available { request, .. }
                    | WicDeviceInventoryState::Partial { request, .. }
                    if request == &dialog.request
            );
            if !request_matches {
                app.notification = Some("The Wic device picker is stale.".into());
                return None;
            }
            replace_dialog(
                app,
                Dialog::WicWritePhrase(WicWritePhraseDialog {
                    request: dialog.request,
                    device: device.identity.clone(),
                    input: String::new(),
                    validation_error: None,
                }),
            );
        }
        Action::CancelWicDevicePicker => {
            if matches!(app.active_dialog(), Some(Dialog::WicDevicePicker(_))) {
                close_dialog(app);
            }
        }
        Action::AppendWicWritePhrase(character) => {
            if let Some(Dialog::WicWritePhrase(dialog)) = app.active_dialog_mut() {
                dialog.append(character);
            }
        }
        Action::BackspaceWicWritePhrase => {
            if let Some(Dialog::WicWritePhrase(dialog)) = app.active_dialog_mut() {
                dialog.backspace();
            }
        }
        Action::PreviewWicDeviceWrite => {
            let Some(Dialog::WicWritePhrase(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            match current_wic_write_preview(app, &dialog.request, &dialog.device, &dialog.input) {
                Ok(preview) => replace_dialog(app, Dialog::WicWriteConfirmation(preview)),
                Err(message) => {
                    if let Some(Dialog::WicWritePhrase(active)) = app.active_dialog_mut() {
                        active.validation_error = Some(message.clone());
                    }
                    app.notification = Some(message);
                }
            }
        }
        Action::CancelWicWritePhrase => {
            if matches!(app.active_dialog(), Some(Dialog::WicWritePhrase(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmWicDeviceWrite => {
            let Some(Dialog::WicWriteConfirmation(preview)) = app.active_dialog().cloned() else {
                return None;
            };
            let request = WicDeviceInventoryRequest {
                generation: match &app.wic_devices {
                    WicDeviceInventoryState::Available { request, .. }
                    | WicDeviceInventoryState::Partial { request, .. } => request.generation,
                    _ => {
                        app.notification = Some("The Wic device inventory is unavailable.".into());
                        return None;
                    }
                },
                image: preview.request.image.clone(),
            };
            let phrase = format!("WRITE {}", preview.request.device.path.display());
            let current =
                current_wic_write_preview(app, &request, &preview.request.device, &phrase);
            if current.as_ref() != Ok(&preview) {
                app.notification =
                    Some("The Wic device write preview is stale or no longer valid.".into());
                return None;
            }
            close_dialog(app);
            let effect = queue_wic_session(app, WicOperation::Write(preview.request));
            synchronize_focus(app);
            return effect;
        }
        Action::CancelWicWritePreview => {
            if matches!(app.active_dialog(), Some(Dialog::WicWriteConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::StartConfirmedWicCreate(preview) => {
            let Some(output_directory) = preview.request.output_directory.to_str() else {
                app.notification = Some("Wic output paths must be valid UTF-8.".into());
                return None;
            };
            let draft = WicCreateDraft {
                machine: preview.request.machine.clone(),
                image: preview.request.image.clone(),
                kickstart: preview.request.kickstart.clone(),
                output_directory: output_directory.into(),
                generate_bmap: preview.request.generate_bmap,
                compression: preview.request.compression,
            };
            if draft.preview(&app.wic_capability).as_ref() != Ok(&preview) {
                app.notification =
                    Some("The Wic creation preview is stale or no longer valid.".into());
                return None;
            }
            return queue_wic_session(app, WicOperation::Create(preview.request));
        }
        Action::WicSessionStarting { id, started_at } => {
            let Some(job_id) = wic_job_id(app, id) else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Queued], |job| {
                    job.status = BackgroundJobStatus::Starting;
                    job.started_at = Some(started_at);
                });
        }
        Action::WicSessionRunning { id } => {
            let Some(job_id) = wic_job_id(app, id) else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Starting], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
        }
        Action::AppendWicSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        } => {
            let Some(job_id) = wic_job_id(app, id) else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs.append_output(
                job_id,
                BackgroundJobOutputEntry {
                    severity: if stream == WicOutputStream::Stderr {
                        Severity::Warning
                    } else {
                        Severity::Info
                    },
                    message: line,
                    source: if stream == WicOutputStream::Stderr {
                        BackgroundJobOutputSource::Stderr
                    } else {
                        BackgroundJobOutputSource::Stdout
                    },
                    truncated,
                    timestamp,
                },
            );
        }
        Action::CompleteWicSession {
            id,
            exit_code,
            outputs,
            limitations,
            finished_at,
        } => {
            let Some(session) = app.wic_session(id).cloned() else {
                note_stale_wic_event(app);
                return None;
            };
            let Some(job) = app.background_jobs.get(session.background_job_id) else {
                note_stale_wic_event(app);
                return None;
            };
            if job.status != BackgroundJobStatus::Running {
                note_stale_wic_event(app);
                return None;
            }
            let job_id = mutate_wic_session(app, id, |session| session.exit_code = Some(exit_code))
                .expect("session checked above");
            if exit_code == 0 {
                let artifacts = outputs
                    .iter()
                    .map(|output| output.identity.path.clone())
                    .collect();
                app.background_jobs
                    .update_if(job_id, &[BackgroundJobStatus::Running], |job| {
                        job.status = BackgroundJobStatus::Succeeded;
                        job.finished_at = Some(finished_at);
                        job.result = Some(BackgroundJobResult {
                            summary: "Wic operation completed".into(),
                            artifacts,
                        });
                    });
                if let WicOperation::Create(request) = session.operation {
                    let generation = app.wic_output_generation.wrapping_add(1).max(1);
                    app.wic_output_generation = generation;
                    let outputs = normalize_wic_outputs(&request.output_directory, outputs)
                        .unwrap_or_default();
                    let limitations = normalize_wic_limitations(limitations);
                    app.wic_outputs = if limitations.is_empty() {
                        WicOutputInventoryState::Available {
                            request: WicOutputInventoryRequest {
                                generation,
                                output_directory: request.output_directory,
                            },
                            outputs,
                        }
                    } else {
                        WicOutputInventoryState::Partial {
                            request: WicOutputInventoryRequest {
                                generation,
                                output_directory: request.output_directory,
                            },
                            outputs,
                            limitations,
                        }
                    };
                    reconcile_wic_output_selection(app);
                }
            } else {
                let message = format!("exit code {exit_code}");
                let _ = mutate_wic_session(app, id, |session| {
                    session.error_detail = Some(message.clone())
                });
                app.background_jobs
                    .update_if(job_id, &[BackgroundJobStatus::Running], |job| {
                        job.status = BackgroundJobStatus::Failed;
                        job.finished_at = Some(finished_at);
                        job.error = Some(BackgroundJobError {
                            summary: "Wic operation failed".into(),
                            detail: Some(message),
                        });
                    });
            }
        }
        Action::FailWicSession {
            id,
            message,
            exit_code,
            finished_at,
        } => {
            if !matches!(
                wic_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Queued
                        | BackgroundJobStatus::Starting
                        | BackgroundJobStatus::Running
                        | BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_wic_event(app);
                return None;
            }
            let Some(job_id) = mutate_wic_session(app, id, |session| {
                session.exit_code = exit_code;
                session.error_detail = Some(message.clone());
            }) else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Queued,
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Failed;
                    job.finished_at = Some(finished_at);
                    job.error = Some(BackgroundJobError {
                        summary: "Wic operation failed".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::LoseWicSession {
            id,
            message,
            finished_at,
        } => {
            if !matches!(
                wic_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Starting
                        | BackgroundJobStatus::Running
                        | BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_wic_event(app);
                return None;
            }
            let Some(job_id) = mutate_wic_session(app, id, |session| {
                session.error_detail = Some(message.clone())
            }) else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Lost;
                    job.finished_at = Some(finished_at);
                    job.error = Some(BackgroundJobError {
                        summary: "Wic process was lost".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::ConfirmWicSessionCancellation {
            id,
            acknowledge_incomplete_device,
        } => {
            let Some(session) = app.wic_session(id) else {
                note_stale_wic_event(app);
                return None;
            };
            if matches!(session.operation, WicOperation::Write(_)) && !acknowledge_incomplete_device
            {
                app.notification = Some(
                    "A device write cancellation requires the incomplete-device warning.".into(),
                );
                return None;
            }
            let job_id = session.background_job_id;
            let before = app.background_jobs.get(job_id).map(|job| job.status);
            app.background_jobs.request_cancellation(job_id);
            if before == app.background_jobs.get(job_id).map(|job| job.status) {
                app.notification = Some("The Wic cancellation request was rejected.".into());
                return None;
            }
            if matches!(
                app.active_dialog(),
                Some(Dialog::WicCancellationConfirmation { id: candidate, .. })
                    if *candidate == id
            ) {
                close_dialog(app);
            }
            return Some(Effect::CancelWicSession(id));
        }
        Action::RejectWicSessionCancellation { id, message } => {
            let Some(job_id) = wic_job_id(app, id) else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
            app.notification = Some(format!("Wic cancellation failed: {message}"));
        }
        Action::CancelWicSession {
            id,
            exit_code,
            finished_at,
        } => {
            if !matches!(
                wic_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_wic_event(app);
                return None;
            }
            let is_device_write = matches!(
                app.wic_session(id).map(|session| &session.operation),
                Some(WicOperation::Write(_))
            );
            let Some(job_id) = mutate_wic_session(app, id, |session| session.exit_code = exit_code)
            else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Cancelled;
                    job.finished_at = Some(finished_at);
                    if is_device_write {
                        job.error = Some(BackgroundJobError {
                            summary: "Wic device write cancelled".into(),
                            detail: Some("The target device may be incomplete.".into()),
                        });
                    }
                });
        }
        Action::BeginBuildTargetEdit => {
            let mut editor = PopupEditor::new(popup_toml_document(
                "target",
                app.build.target.as_deref().unwrap_or_default(),
                None,
            ));
            let _ = editor.select_toml_value("target");
            replace_dialog(app, Dialog::BuildTarget { editor, task: None });
        }
        Action::BeginBuildTargetTask(task) => {
            let mut editor = PopupEditor::new(popup_toml_document(
                "target",
                app.build.target.as_deref().unwrap_or_default(),
                None,
            ));
            let _ = editor.select_toml_value("target");
            replace_dialog(app, Dialog::BuildTarget { editor, task });
        }
        Action::ToggleBuildTargetEdit => {
            if let Some(Dialog::BuildTarget { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendBuildTarget(character) => {
            if let Some(Dialog::BuildTarget { editor, .. }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceBuildTarget => {
            if let Some(Dialog::BuildTarget { editor, .. }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::CancelBuildTargetEdit => {
            if matches!(app.active_dialog(), Some(Dialog::BuildTarget { .. })) {
                close_dialog(app);
            }
        }
        Action::ConfirmBuildTarget => {
            if let Some(Dialog::BuildTarget { editor, task }) = app.active_dialog() {
                let input = match popup_toml_value(&editor.text, "target") {
                    Ok(value) => value,
                    Err(reason) => {
                        app.notification = Some(reason);
                        return None;
                    }
                };
                let request = BuildRequest {
                    targets: vec![input],
                    task: task.clone(),
                    force: false,
                };
                if let Err(error) = request.validate() {
                    app.notification = Some(error.to_string());
                } else {
                    replace_dialog(app, Dialog::RecipeTaskConfirmation(request));
                }
            }
        }
        Action::Start(r) => {
            if !app.build_environment.connected() {
                app.notification = Some("Configure and verify a BitBake environment first".into());
            } else if let Err(e) = r.validate() {
                app.notification = Some(e.to_string())
            } else {
                prepare_build(app, r.targets.first().cloned());
                return Some(Effect::Start(r));
            }
        }
        Action::QueueBackgroundJob(spec) => app.background_jobs.queue(spec),
        Action::StartBackgroundJob { id, started_at } => {
            app.background_jobs
                .update_if(id, &[BackgroundJobStatus::Queued], |job| {
                    job.status = BackgroundJobStatus::Starting;
                    job.started_at = Some(started_at);
                })
        }
        Action::RunBackgroundJob { id } => {
            app.background_jobs
                .update_if(id, &[BackgroundJobStatus::Starting], |job| {
                    job.status = BackgroundJobStatus::Running
                })
        }
        Action::UpdateBackgroundJobProgress { id, progress } => {
            if progress.is_valid() {
                app.background_jobs
                    .update_if(id, &[BackgroundJobStatus::Running], |job| {
                        job.progress = progress
                    });
            } else {
                app.background_jobs.ignored_transitions += 1;
            }
        }
        Action::AppendBackgroundJobOutput { id, entry } => {
            app.background_jobs.append_output(id, entry);
        }
        Action::RequestBackgroundJobCancellation { id } => {
            app.background_jobs.request_cancellation(id);
        }
        Action::RejectBackgroundJobCancellation { id } => {
            app.background_jobs
                .update_if(id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Running
                })
        }
        Action::SucceedBackgroundJob {
            id,
            result,
            finished_at,
        } => app
            .background_jobs
            .update_if(id, &[BackgroundJobStatus::Running], |job| {
                job.status = BackgroundJobStatus::Succeeded;
                job.finished_at = Some(finished_at);
                job.result = Some(result);
            }),
        Action::FailBackgroundJob {
            id,
            error,
            finished_at,
        } => app.background_jobs.update_if(
            id,
            &[
                BackgroundJobStatus::Starting,
                BackgroundJobStatus::Running,
                BackgroundJobStatus::Cancelling,
            ],
            |job| {
                job.status = BackgroundJobStatus::Failed;
                job.finished_at = Some(finished_at);
                job.error = Some(error);
            },
        ),
        Action::CancelBackgroundJob { id, finished_at } => {
            app.background_jobs
                .update_if(id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Cancelled;
                    job.finished_at = Some(finished_at);
                })
        }
        Action::LoseBackgroundJob {
            id,
            error,
            finished_at,
        } => app.background_jobs.update_if(
            id,
            &[
                BackgroundJobStatus::Starting,
                BackgroundJobStatus::Running,
                BackgroundJobStatus::Cancelling,
            ],
            |job| {
                job.status = BackgroundJobStatus::Lost;
                job.finished_at = Some(finished_at);
                job.error = Some(error);
            },
        ),
        Action::BuildRequested { target } => prepare_build(app, target),
        Action::BuildStarted => {
            app.build.status = BuildStatus::Running;
            app.build.started = Some(SystemTime::now());
            app.build.parse_current = None;
            app.build.parse_total = None;
        }
        Action::ParseProgress { current, total } => {
            app.build.status = BuildStatus::Parsing;
            app.build.parse_current = current;
            app.build.parse_total = total;
        }
        Action::TaskStarted(t) => {
            let mut task = t;
            if let Some(stats) = task.stats {
                app.build.completed = app.build.completed.max(stats.completed);
                app.build.total = (stats.total > 0).then_some(stats.total);
            }
            task.state = TaskState::Active;
            task.started.get_or_insert_with(SystemTime::now);
            app.tasks.insert(task.id.clone(), task);
            clamp_task_selection(app);
        }
        Action::TaskQueued(t) => {
            let mut task = t;
            if let Some(stats) = task.stats {
                app.build.completed = app.build.completed.max(stats.completed);
                app.build.total = (stats.total > 0).then_some(stats.total);
            }
            task.state = TaskState::Waiting;
            app.tasks.insert(task.id.clone(), task);
            clamp_task_selection(app);
        }
        Action::TaskProgress { id, progress } => {
            if let Some(t) = app.tasks.get_mut(&id) {
                t.progress = progress.map(|value| value.min(100))
            }
        }
        Action::TaskCompleted { id, success } => {
            if let Some(mut task) = app.tasks.remove(&id) {
                task.progress = Some(100);
                task.state = if success {
                    TaskState::Completed
                } else {
                    TaskState::Failed
                };
                task.finished = Some(SystemTime::now());
                app.completed_tasks
                    .push_back(CompletedTask { task, success });
                if app.completed_tasks.len() > MAX_COMPLETED_TASKS {
                    app.completed_tasks.pop_front();
                }
                app.build.completed += 1;
            }
            clamp_task_selection(app);
        }
        Action::ScrollBuildTasks { delta } => {
            let task_count = app.visible_task_rows().len();
            app.task_progress_scroll = if delta.is_negative() {
                app.task_progress_scroll
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.task_progress_scroll
                    .saturating_add(delta as usize)
                    .min(task_count.saturating_sub(1))
            };
        }
        Action::CycleTaskStateFilter => {
            app.task_filters.state = app.task_filters.state.next();
            clamp_task_selection(app);
        }
        Action::CycleTaskFilterField => {
            app.task_filter_field = app.task_filter_field.next();
        }
        Action::BeginTaskFilterEdit => {
            app.task_filter_editing = true;
        }
        Action::AppendTaskFilter(character) => {
            if app.task_filter_editing {
                match app.task_filter_field {
                    TaskFilterField::Recipe => app.task_filters.recipe.push(character),
                    TaskFilterField::Task => app.task_filters.task.push(character),
                    TaskFilterField::Worker => app.task_filters.worker.push(character),
                }
                clamp_task_selection(app);
            }
        }
        Action::BackspaceTaskFilter => {
            if app.task_filter_editing {
                match app.task_filter_field {
                    TaskFilterField::Recipe => app.task_filters.recipe.pop(),
                    TaskFilterField::Task => app.task_filters.task.pop(),
                    TaskFilterField::Worker => app.task_filters.worker.pop(),
                };
                clamp_task_selection(app);
            }
        }
        Action::FinishTaskFilterEdit => {
            app.task_filter_editing = false;
        }
        Action::CycleTaskDurationFilter => {
            app.task_filters.minimum_duration = match app.task_filters.minimum_duration {
                None => Some(Duration::from_secs(1)),
                Some(duration) if duration == Duration::from_secs(1) => {
                    Some(Duration::from_secs(10))
                }
                Some(duration) if duration == Duration::from_secs(10) => {
                    Some(Duration::from_secs(60))
                }
                Some(_) => None,
            };
            clamp_task_selection(app);
        }
        Action::Log(l) => {
            let mut entry = l;
            match entry.severity {
                Severity::Warning => app.build.warnings += 1,
                Severity::Error => app.build.errors += 1,
                _ => {}
            }
            entry.build = entry.build.or_else(|| app.build.target.clone());
            entry.protected |= matches!(entry.severity, Severity::Warning | Severity::Error);
            app.logs.insert(entry);
            app.error_selection = app
                .error_selection
                .min(app.logs.diagnostics().count().saturating_sub(1));
            if app.logs.follow {
                app.logs.selection = app.logs.filtered().count().saturating_sub(1);
                app.logs.scroll_offset = 0;
            }
        }
        Action::BuildCompleted { success, exit_code } => {
            archive_unfinished_tasks(app, TaskState::Lost, Some("build ended"));
            app.build.status = if success {
                BuildStatus::Completed
            } else {
                BuildStatus::Failed
            };
            if !success {
                app.build.errors = app.build.errors.max(1);
            }
            if success && let Some(total) = app.build.total {
                app.build.completed = total;
            }
            insert_system_log(
                app,
                if success {
                    Severity::Info
                } else {
                    Severity::Error
                },
                format!(
                    "Build {} with exit code {}",
                    if success { "completed" } else { "failed" },
                    exit_code.map_or_else(|| "unknown".into(), |code| code.to_string())
                ),
            );
            app.build.exit_code = exit_code;
            app.build_history.push_back(BuildRecord {
                target: app.build.target.clone(),
                success,
                exit_code,
                elapsed: app.elapsed(),
                completed_tasks: app.build.completed,
                warnings: app.build.warnings,
                errors: app.build.errors,
            });
            if app.build_history.len() > MAX_BUILD_HISTORY {
                app.build_history.pop_front();
            }
            app.build_history_selection = 0;
            clamp_task_selection(app);
            enqueue_build_completion(app);
            app.notification = Some(if success {
                if app.build.warnings > 0 {
                    format!(
                        "Build completed with {} warning(s). Open Errors to investigate.",
                        app.build.warnings
                    )
                } else {
                    "Build completed successfully with no errors.".into()
                }
            } else {
                format!(
                    "Build failed with {} error(s). Press Enter to open Errors.",
                    app.build.errors
                )
            });
        }
        Action::BuildCancelled { exit_code } => {
            archive_unfinished_tasks(app, TaskState::Cancelled, Some("cancelled"));
            app.build.status = BuildStatus::Cancelled;
            app.build.exit_code = exit_code;
            insert_system_log(
                app,
                Severity::Warning,
                format!(
                    "Build cancelled with exit code {}",
                    exit_code.map_or_else(|| "unknown".into(), |code| code.to_string())
                ),
            );
            app.build_history.push_back(BuildRecord {
                target: app.build.target.clone(),
                success: false,
                exit_code,
                elapsed: app.elapsed(),
                completed_tasks: app.build.completed,
                warnings: app.build.warnings,
                errors: app.build.errors,
            });
            if app.build_history.len() > MAX_BUILD_HISTORY {
                app.build_history.pop_front();
            }
            app.build_history_selection = 0;
            clamp_task_selection(app);
            enqueue_build_completion(app);
            app.notification =
                Some("Build was cancelled; this is distinct from a build failure.".into());
        }
        Action::BuildCancellationRejected(message) => {
            if app.build.status == BuildStatus::Cancelling {
                app.build.status = BuildStatus::Running;
            }
            for task in app.tasks.values_mut() {
                task.cancellation = None;
            }
            insert_system_log(
                app,
                Severity::Warning,
                format!("Build cancellation was rejected: {message}"),
            );
            app.notification = Some(format!(
                "Could not cancel the active build: {message}. The build may still be running."
            ));
        }
        Action::DismissBuildCompletion => {
            if matches!(app.active_dialog(), Some(Dialog::BuildCompletion)) {
                close_dialog(app);
            }
        }
        Action::OpenBuildCompletionErrors => {
            if matches!(app.active_dialog(), Some(Dialog::BuildCompletion)) {
                close_dialog(app);
            }
            app.screen = Screen::Errors;
            app.error_selection = app.logs.diagnostics().count().saturating_sub(1);
            app.notification = None;
        }
        Action::SelectBuildHistory { delta } => {
            app.build_history_selection = if delta.is_negative() {
                app.build_history_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.build_history_selection
                    .saturating_add(delta as usize)
                    .min(app.build_history.len().saturating_sub(1))
            };
        }
        Action::Cancel => {
            if matches!(
                app.build.status,
                BuildStatus::Running | BuildStatus::Parsing
            ) {
                app.build.status = BuildStatus::Cancelling;
                for task in app.tasks.values_mut() {
                    task.cancellation = Some("cancellation requested".into());
                }
                insert_system_log(
                    app,
                    Severity::Warning,
                    "Build cancellation requested".into(),
                );
                return Some(Effect::Cancel);
            }
        }
        Action::ToggleLogFollow => {
            app.logs.follow = !app.logs.follow;
            app.logs.paused_len = (!app.logs.follow).then_some(app.logs.entries.len());
            if app.logs.follow {
                app.logs.selection = app.logs.filtered().count().saturating_sub(1);
                app.logs.scroll_offset = 0;
            }
        }
        Action::ToggleLogWrap => {
            app.logs.wrap = !app.logs.wrap;
            if app.logs.wrap {
                app.logs.horizontal_offset = 0;
            }
        }
        Action::CycleLogSeverity => {
            app.logs.filter = match app.logs.filter {
                None => Some(Severity::Info),
                Some(Severity::Info) => Some(Severity::Warning),
                Some(Severity::Warning) => Some(Severity::Error),
                Some(Severity::Error) | Some(Severity::Trace) => None,
            };
            app.logs.jump_target = None;
            app.logs.clamp_selection();
        }
        Action::ScrollLogs { delta } => {
            app.logs.follow = false;
            app.logs.paused_len = Some(app.logs.entries.len());
            let count = app.logs.filtered().count();
            app.logs.selection = if delta.is_negative() {
                app.logs
                    .selection
                    .saturating_add(delta.unsigned_abs())
                    .min(count.saturating_sub(1))
            } else {
                app.logs.selection.saturating_sub(delta as usize)
            };
            app.logs.scroll_offset = count.saturating_sub(app.logs.selection.saturating_add(1));
        }
        Action::BeginLogSearch => {
            app.logs.jump_target = None;
            app.logs.searching = true;
            app.logs.follow = false;
            app.logs.paused_len = Some(app.logs.entries.len());
        }
        Action::AppendLogQuery(character) if app.logs.searching => {
            app.logs.query.push(character);
            app.logs.clamp_selection();
        }
        Action::BackspaceLogQuery if app.logs.searching => {
            app.logs.query.pop();
            app.logs.clamp_selection();
        }
        Action::FinishLogSearch => app.logs.searching = false,
        Action::NextLogMatch if !app.logs.query.is_empty() => {
            let count = app.logs.filtered().count();
            app.logs.follow = false;
            app.logs.paused_len = Some(app.logs.entries.len());
            app.logs.selection = app
                .logs
                .selection
                .saturating_add(1)
                .min(count.saturating_sub(1));
            app.logs.scroll_offset = count.saturating_sub(app.logs.selection.saturating_add(1));
        }
        Action::PreviousLogMatch if !app.logs.query.is_empty() => {
            app.logs.follow = false;
            app.logs.paused_len = Some(app.logs.entries.len());
            app.logs.selection = app.logs.selection.saturating_sub(1);
            let count = app.logs.filtered().count();
            app.logs.scroll_offset = count.saturating_sub(app.logs.selection.saturating_add(1));
        }
        Action::ScrollLogsHorizontally { delta } => {
            if app.logs.wrap {
                return None;
            }
            app.logs.horizontal_offset = if delta.is_negative() {
                app.logs
                    .horizontal_offset
                    .saturating_sub(delta.unsigned_abs())
            } else {
                let maximum = app
                    .logs
                    .filtered()
                    .map(|entry| entry.message.chars().count())
                    .max()
                    .unwrap_or(0)
                    .saturating_sub(1);
                app.logs
                    .horizontal_offset
                    .saturating_add(delta as usize)
                    .min(maximum)
            };
        }
        Action::CycleLogRecipeFilter => {
            app.logs.jump_target = None;
            let mut values = app
                .logs
                .entries
                .iter()
                .filter_map(|entry| entry.recipe.clone())
                .collect::<Vec<_>>();
            values.sort();
            values.dedup();
            app.logs.recipe_filter = next_filter(&values, app.logs.recipe_filter.take());
            app.logs.clamp_selection();
        }
        Action::CycleLogTaskFilter => {
            app.logs.jump_target = None;
            let mut values = app
                .logs
                .entries
                .iter()
                .filter_map(|entry| entry.task.clone())
                .collect::<Vec<_>>();
            values.sort();
            values.dedup();
            app.logs.task_filter = next_filter(&values, app.logs.task_filter.take());
            app.logs.clamp_selection();
        }
        Action::CycleLogBuildFilter => {
            app.logs.jump_target = None;
            let mut values = app
                .logs
                .entries
                .iter()
                .filter_map(|entry| entry.build.clone())
                .collect::<Vec<_>>();
            values.sort();
            values.dedup();
            app.logs.build_filter = next_filter(&values, app.logs.build_filter.take());
            app.logs.clamp_selection();
        }
        Action::OpenSelectedLogSource => {
            if let Some(path) = app.logs.selected().and_then(|entry| entry.path.clone()) {
                return Some(Effect::OpenInEditor(path));
            }
            app.notification = Some("The selected log entry has no source path.".into());
        }
        Action::CopySelectedLog => {
            if let Some(entry) = app.logs.selected() {
                return Some(Effect::CopyToClipboard(format_log_details(entry)));
            }
            app.notification = Some("No log entry is selected to copy.".into());
        }
        Action::SelectError { delta } => {
            let count = app.logs.diagnostics().count();
            app.error_selection = if delta.is_negative() {
                app.error_selection.saturating_sub(delta.unsigned_abs())
            } else {
                app.error_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::JumpToSelectedError => {
            let id = {
                app.logs
                    .diagnostics()
                    .nth(app.error_selection)
                    .map(|entry| entry.id)
            };
            if let Some(id) = id {
                app.logs.jump_target = Some(id);
                app.logs.follow = false;
                app.logs.paused_len = Some(app.logs.entries.len());
                let selection = app
                    .logs
                    .filtered()
                    .position(|entry| entry.id == id)
                    .unwrap_or(0);
                let count = app.logs.filtered().count();
                app.logs.selection = selection;
                app.logs.scroll_offset = count.saturating_sub(selection.saturating_add(1));
                app.screen = Screen::Logs;
            }
        }
        Action::OpenSelectedErrorSource => {
            let selected = app.logs.diagnostics().nth(app.error_selection);
            if let Some(path) = selected.and_then(|entry| entry.path.clone()) {
                return Some(Effect::OpenInEditor(path));
            }
            app.notification = Some("The selected diagnostic has no source log path.".into());
        }
        Action::SelectRecipe { delta } => {
            let matches = app
                .workspace
                .recipes
                .iter()
                .enumerate()
                .filter(|(_, recipe)| recipe_matches_query(recipe, &app.metadata_query))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let position = matches
                .iter()
                .position(|index| *index == app.recipe_selection)
                .unwrap_or(0);
            let position = if delta.is_negative() {
                position.saturating_sub(delta.unsigned_abs())
            } else {
                position
                    .saturating_add(delta as usize)
                    .min(matches.len().saturating_sub(1))
            };
            app.recipe_selection = matches.get(position).copied().unwrap_or(0);
        }
        Action::BeginSelectedRecipeBuild => {
            begin_recipe_task(app, None, false);
        }
        Action::BeginSelectedRecipeClean => {
            begin_recipe_task(app, Some("clean".into()), false);
        }
        Action::BeginSelectedRecipeMenuConfig => {
            begin_recipe_task(app, Some("menuconfig".into()), false);
        }
        Action::BeginSelectedRecipeCleanState => {
            begin_recipe_task(app, Some("cleansstate".into()), false);
        }
        Action::BeginSelectedRecipeDevshell => {
            begin_recipe_task(app, Some("devshell".into()), false);
        }
        Action::BeginSelectedRecipeDiffconfig => {
            begin_recipe_task(app, Some("diffconfig".into()), false);
        }
        Action::BeginSelectedRecipeDiffsigs => {
            begin_recipe_task(app, Some("diffsigs".into()), false);
        }
        Action::BeginSelectedRecipeSignatures => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.replace("Devtool status", "signatures"));
                    return None;
                }
            };
            let Some(tasks) = app
                .recipe_metadata
                .get(&identity.name)
                .and_then(|metadata| metadata.tasks.as_ref())
            else {
                app.notification = Some(
                    "Load authoritative recipe tasks with Enter before inspecting signatures."
                        .into(),
                );
                return None;
            };
            let mut tasks = tasks
                .iter()
                .filter(|task| {
                    SignatureTarget {
                        recipe: identity.name.clone(),
                        task: (*task).clone(),
                    }
                    .validate()
                    .is_ok()
                })
                .cloned()
                .collect::<Vec<_>>();
            tasks.sort();
            tasks.dedup();
            if tasks.is_empty() {
                app.notification =
                    Some("BitBake reported no valid signature tasks for this recipe.".into());
                return None;
            }
            open_dialog(
                app,
                Dialog::SignatureTaskPicker(SignatureTaskPicker {
                    recipe: identity,
                    tasks,
                    selection: 0,
                }),
            );
        }
        Action::BeginSelectedRecipeCveCheck => {
            begin_recipe_task(app, Some("cve_check".into()), false);
        }
        Action::BeginSelectedRecipeSpdx => {
            begin_recipe_task(app, Some("create_spdx".into()), false);
        }
        Action::BeginSelectedRecipeTask { task, force } => {
            begin_recipe_task(app, task, force);
        }
        Action::BeginSelectedRecipeForceTask => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected for forced task execution.".into());
                return None;
            };
            let Some(tasks) = app
                .recipe_metadata
                .get(&recipe.name)
                .and_then(|metadata| metadata.tasks.as_ref())
            else {
                app.notification = Some(
                    "Load authoritative recipe tasks with Enter before forcing a task.".into(),
                );
                return None;
            };
            let mut tasks = tasks
                .iter()
                .map(|task| task.strip_prefix("do_").unwrap_or(task).to_owned())
                .filter(|task| {
                    !task.is_empty()
                        && task.chars().all(|character| {
                            character.is_ascii_alphanumeric()
                                || matches!(character, '-' | '_' | '.' | '+')
                        })
                })
                .collect::<Vec<_>>();
            tasks.sort();
            tasks.dedup();
            if tasks.is_empty() {
                app.notification =
                    Some("BitBake reported no forceable tasks for this recipe.".into());
            } else {
                open_dialog(
                    app,
                    Dialog::RecipeTaskPicker(RecipeTaskPicker {
                        recipe: recipe.name.clone(),
                        tasks,
                        selection: 0,
                        force: true,
                    }),
                );
            }
        }
        Action::SelectRecipeTask { delta } => {
            if let Some(Dialog::RecipeTaskPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.tasks.len().saturating_sub(1))
                };
            }
        }
        Action::PreviewSelectedRecipeTask => {
            if let Some(Dialog::RecipeTaskPicker(picker)) = app.active_dialog()
                && let Some(task) = picker.tasks.get(picker.selection)
            {
                let request = BuildRequest {
                    targets: vec![picker.recipe.clone()],
                    task: Some(task.clone()),
                    force: picker.force,
                };
                replace_dialog(app, Dialog::RecipeTaskConfirmation(request));
            }
        }
        Action::CancelRecipeTaskPicker => {
            if matches!(app.active_dialog(), Some(Dialog::RecipeTaskPicker(_))) {
                close_dialog(app);
            }
        }
        Action::SelectSignatureTask { delta } => {
            if let Some(Dialog::SignatureTaskPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.tasks.len().saturating_sub(1))
                };
            }
        }
        Action::ConfirmSignatureTask => {
            let Some(Dialog::SignatureTaskPicker(picker)) = app.active_dialog().cloned() else {
                return None;
            };
            let Some(task) = picker.tasks.get(picker.selection).cloned() else {
                app.notification = Some("No authoritative signature task is selected.".into());
                return None;
            };
            let target = SignatureTarget {
                recipe: picker.recipe.name.clone(),
                task,
            };
            if let Err(message) = target.validate() {
                app.notification = Some(message.into());
                return None;
            }
            close_dialog(app);
            app.screen = Screen::Signatures;
            app.focus = FocusTarget::Workspace;
            app.focus_return = None;
            app.signature_recipe = Some(picker.recipe);
            app.signature_selection = None;
            app.signature_comparison = SignatureComparisonState::NotSelected;
            return begin_signature_dump(app, target);
        }
        Action::CancelSignatureTaskPicker => {
            if matches!(app.active_dialog(), Some(Dialog::SignatureTaskPicker(_))) {
                close_dialog(app);
            }
        }
        Action::OpenSelectedRecipeProvider => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected to open.".into());
                return None;
            };
            if let Some(path) = recipe.file.clone() {
                return Some(Effect::OpenInEditor(path));
            }
            app.notification = Some(format!(
                "BitBake did not report an authoritative provider path for {}.",
                recipe.name
            ));
        }
        Action::BeginSelectedRecipeTaskLog => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected for task-log inspection.".into());
                return None;
            };
            let recipe_name = recipe.name.clone();
            let mut logs = app
                .tasks
                .values()
                .chain(app.completed_tasks.iter().map(|completed| &completed.task))
                .filter(|task| task.recipe == recipe_name)
                .filter_map(|task| {
                    task.log_path.clone().map(|path| RecipeTaskLogChoice {
                        task: task.task.clone(),
                        state: task.state,
                        path,
                    })
                })
                .collect::<Vec<_>>();
            logs.sort_by(|left, right| {
                left.task
                    .cmp(&right.task)
                    .then_with(|| left.path.cmp(&right.path))
            });
            logs.dedup_by(|left, right| left.path == right.path);
            match logs.len() {
                0 => {
                    app.notification = Some(format!(
                        "No retained task log path is available for {recipe_name}; BitBake may not have reported one or it may have been evicted."
                    ));
                }
                1 => return Some(Effect::OpenInEditor(logs.remove(0).path)),
                _ => open_dialog(
                    app,
                    Dialog::RecipeTaskLogPicker(RecipeTaskLogPicker {
                        recipe: recipe_name,
                        logs,
                        selection: 0,
                    }),
                ),
            }
        }
        Action::SelectRecipeTaskLog { delta } => {
            if let Some(Dialog::RecipeTaskLogPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.logs.len().saturating_sub(1))
                };
            }
        }
        Action::OpenSelectedRecipeTaskLog => {
            if let Some(Dialog::RecipeTaskLogPicker(picker)) = app.active_dialog()
                && let Some(path) = picker
                    .logs
                    .get(picker.selection)
                    .map(|choice| choice.path.clone())
            {
                close_dialog(app);
                return Some(Effect::OpenInEditor(path));
            }
        }
        Action::CancelRecipeTaskLogPicker => {
            if matches!(app.active_dialog(), Some(Dialog::RecipeTaskLogPicker(_))) {
                close_dialog(app);
            }
        }
        Action::BeginSelectedRecipePatchReview => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected for patch review.".into());
                return None;
            };
            let recipe_name = recipe.name.clone();
            let Some(patches) = app
                .recipe_metadata
                .get(&recipe_name)
                .and_then(|metadata| metadata.patches.as_ref())
            else {
                app.notification = Some(format!(
                    "Load authoritative metadata for {recipe_name} with Enter before reviewing patches."
                ));
                return None;
            };
            let mut local_patches = patches
                .iter()
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .collect::<Vec<_>>();
            local_patches.sort();
            local_patches.dedup();
            if local_patches.is_empty() {
                app.notification = Some(if patches.is_empty() {
                    format!("BitBake reported no patches for {recipe_name}.")
                } else {
                    format!(
                        "The patches for {recipe_name} are remote or unresolved; no authoritative local path is available."
                    )
                });
            } else if local_patches.len() == 1 {
                return Some(Effect::OpenInEditor(local_patches.remove(0)));
            } else {
                open_dialog(
                    app,
                    Dialog::RecipePatchPicker(RecipePatchPicker {
                        recipe: recipe_name,
                        patches: local_patches,
                        selection: 0,
                    }),
                );
            }
        }
        Action::SelectRecipePatch { delta } => {
            if let Some(Dialog::RecipePatchPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.patches.len().saturating_sub(1))
                };
            }
        }
        Action::OpenSelectedRecipePatch => {
            if let Some(Dialog::RecipePatchPicker(picker)) = app.active_dialog()
                && let Some(path) = picker.patches.get(picker.selection).cloned()
            {
                close_dialog(app);
                return Some(Effect::OpenInEditor(path));
            }
        }
        Action::CancelRecipePatchPicker => {
            if matches!(app.active_dialog(), Some(Dialog::RecipePatchPicker(_))) {
                close_dialog(app);
            }
        }
        Action::BeginSelectedRecipeDevtoolModify => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification =
                    Some("Refresh authoritative Devtool status with t before modifying.".into());
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::ModifyOrEdit) {
                app.notification = Some(reason);
                return None;
            }
            if let DevtoolWorkspace::Present { source_path, .. } = &status.workspace {
                return Some(Effect::OpenWorkspaceEditor {
                    label: identity.name,
                    root: source_path.clone(),
                });
            }
            open_dialog(app, Dialog::DevtoolModifyConfirmation(identity));
        }
        Action::BeginSelectedRecipeDevtoolStatus => match selected_recipe_identity(app) {
            Ok(identity) => {
                app.devtool_status_loading.insert(identity.clone());
                return Some(Effect::InspectDevtoolStatus(identity));
            }
            Err(message) => app.notification = Some(message.into()),
        },
        Action::DevtoolStatusLoaded(status) => {
            app.devtool_status_loading.remove(&status.identity);
            app.devtool_statuses.insert(status.identity.clone(), status);
        }
        Action::BeginSelectedRecipeDevtoolReset => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification =
                    Some("Refresh authoritative Devtool status with t before reset.".into());
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::Reset) {
                app.notification = Some(reason);
                return None;
            }
            let source_path = match &status.workspace {
                DevtoolWorkspace::Present { source_path, .. }
                | DevtoolWorkspace::MissingDirectory { source_path } => source_path.clone(),
                DevtoolWorkspace::NotMember => return None,
            };
            if !source_path.is_absolute() {
                app.notification =
                    Some("The authoritative Devtool reset source path is not absolute.".into());
                return None;
            }
            open_dialog(
                app,
                Dialog::DevtoolResetConfirmation(DevtoolResetPlan {
                    identity,
                    source_path,
                }),
            );
        }
        Action::BeginSelectedRecipeDevtoolUpdateRecipe => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification = Some(
                    "Refresh authoritative Devtool status with t before update-recipe.".into(),
                );
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::UpdateRecipe) {
                app.notification = Some(reason);
                return None;
            }
            open_dialog(app, Dialog::DevtoolUpdateConfirmation(identity));
        }
        Action::BeginSelectedRecipeDevtoolFinish => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification =
                    Some("Refresh authoritative Devtool status with t before finish.".into());
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::Finish) {
                app.notification = Some(reason);
                return None;
            }
            let layers = app
                .workspace
                .layers
                .iter()
                .filter(|layer| layer.path.is_absolute())
                .cloned()
                .collect::<Vec<_>>();
            if layers.is_empty() {
                app.notification =
                    Some("No configured layer has an absolute finish destination.".into());
                return None;
            }
            let provider_layer = app
                .workspace
                .recipes
                .get(app.recipe_selection)
                .and_then(|recipe| recipe.layer.as_deref());
            let selection = provider_layer
                .and_then(|name| layers.iter().position(|layer| layer.name == name))
                .unwrap_or(0);
            open_dialog(
                app,
                Dialog::DevtoolFinishPicker(DevtoolFinishPicker {
                    identity,
                    layers,
                    selection,
                }),
            );
        }
        Action::BeginSelectedRecipeDevtoolDeploy => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification = Some(
                    "Refresh authoritative Devtool status with t before deploy-target.".into(),
                );
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::Deploy) {
                app.notification = Some(reason);
                return None;
            }
            open_dialog(
                app,
                Dialog::DevtoolDeploy(DevtoolDeployDraft {
                    identity,
                    target: String::new(),
                }),
            );
        }
        Action::BeginSelectedRecipeDependencies => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                return update(
                    app,
                    Action::BeginDependencyGraph {
                        root: DependencyNodeId::recipe(recipe.name.clone()),
                    },
                );
            }
            app.notification = Some("No recipe is selected for dependency inspection.".into());
        }
        Action::BeginDependencyGraph { root } => {
            app.dependency_graph = DependencyGraphState::Loading { root: root.clone() };
            app.dependency_graph_selection = Some(root.clone());
            return Some(Effect::GetDependencies(root.recipe_name().to_owned()));
        }
        Action::DependencyGraphLoaded(graph) => {
            set_dependency_graph(app, graph, None);
        }
        Action::DependencyGraphPartial { graph, limitations } => {
            set_dependency_graph(app, graph, Some(limitations));
        }
        Action::DependencyGraphFailed { root, message } => {
            app.dependency_graph = DependencyGraphState::Failed {
                root: root.clone(),
                message: message.clone(),
            };
            app.dependency_graph_selection = Some(root);
            app.notification = Some(format!("Dependency graph is unavailable: {message}"));
        }
        Action::SelectDependencyGraphNode { delta } => {
            let graph = app.dependency_graph.graph()?;
            if graph.nodes.is_empty() {
                app.dependency_graph_selection = None;
                return None;
            }
            let current = app
                .dependency_graph_selection
                .as_ref()
                .and_then(|selected| graph.nodes.iter().position(|node| &node.id == selected))
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(graph.nodes.len().saturating_sub(1))
            };
            app.dependency_graph_selection = Some(graph.nodes[next].id.clone());
        }
        Action::RefreshDependencyGraph => {
            let Some(root) = app.dependency_graph.root().cloned() else {
                app.notification = Some("No dependency graph root is available to refresh.".into());
                return None;
            };
            return update(app, Action::BeginDependencyGraph { root });
        }
        Action::OpenSelectedDependencyRecipe => {
            let identity = match &app.dependency_graph {
                DependencyGraphState::Available(graph)
                | DependencyGraphState::Partial { graph, .. } => app
                    .dependency_graph_selection
                    .as_ref()
                    .filter(|selected| graph.contains(selected))
                    .cloned(),
                DependencyGraphState::AvailableEmpty { root } => Some(root.clone()),
                DependencyGraphState::NotLoaded
                | DependencyGraphState::Loading { .. }
                | DependencyGraphState::Failed { .. } => None,
            };
            let Some(identity) = identity else {
                app.notification =
                    Some("No current dependency graph node is available to open.".into());
                return None;
            };
            let recipe = identity.recipe_name();
            if let Some(index) = app
                .workspace
                .recipes
                .iter()
                .position(|candidate| candidate.name == recipe)
            {
                app.recipe_selection = index;
                app.screen = Screen::Recipes;
            } else {
                app.notification = Some(format!(
                    "{recipe} is in the dependency graph but not in the authoritative recipe inventory."
                ));
            }
        }
        Action::OpenSelectedDependencyProvider => {
            let selected = app.dependency_graph_selection.as_ref();
            let provider = app.dependency_graph.graph().and_then(|graph| {
                graph
                    .nodes
                    .iter()
                    .find(|node| Some(&node.id) == selected)
                    .and_then(|node| node.provider.clone())
                    .filter(|path| path.is_absolute())
            });
            if let Some(provider) = provider {
                return Some(Effect::OpenInEditor(provider));
            }
            app.notification =
                Some("The selected dependency node has no authoritative provider path.".into());
        }
        Action::OpenSelectedDependencyTaskLog => {
            let selected = app.dependency_graph_selection.as_ref();
            if !matches!(selected, Some(DependencyNodeId::Task { .. })) {
                app.notification =
                    Some("Task logs are available only for typed task dependency nodes.".into());
                return None;
            }
            let log = app.dependency_graph.graph().and_then(|graph| {
                graph
                    .nodes
                    .iter()
                    .find(|node| Some(&node.id) == selected)
                    .and_then(|node| node.log.clone())
                    .filter(|path| path.is_absolute())
            });
            if let Some(log) = log {
                return Some(Effect::OpenInEditor(log));
            }
            app.notification =
                Some("The selected task dependency has no authoritative log path.".into());
        }
        Action::BeginSignatureDump(target) => {
            return begin_signature_dump(app, target);
        }
        Action::RefreshSignatureDump => {
            let Some(target) = app.signature_dump.target().cloned() else {
                app.notification = Some("No signature target is available to refresh.".into());
                return None;
            };
            if signature_operation_is_loading(app) {
                app.notification = Some("A signature operation is already running.".into());
                return None;
            }
            return begin_signature_dump(app, target);
        }
        Action::LeaveSignatureWorkspace => {
            if signature_operation_is_loading(app) {
                return Some(Effect::CancelSignatureOperation);
            }
            if let Some(identity) = app.signature_recipe.as_ref()
                && let Some(index) = app.workspace.recipes.iter().position(|recipe| {
                    recipe.name == identity.name && recipe.file.as_ref() == Some(&identity.file)
                })
            {
                app.recipe_selection = index;
            }
            app.screen = Screen::Recipes;
            app.focus = FocusTarget::Workspace;
        }
        Action::OpenSignatureProvider => {
            let Some(identity) = app.signature_recipe.as_ref() else {
                app.notification =
                    Some("No signature recipe provider is available to open.".into());
                return None;
            };
            if !identity.file.is_absolute() {
                app.notification =
                    Some("The signature recipe provider path is not absolute.".into());
                return None;
            }
            return Some(Effect::OpenInEditor(identity.file.clone()));
        }
        Action::SignatureDumpLoaded { target, records } => {
            if !matches!(
                &app.signature_dump,
                SignatureDumpState::Loading { target: requested } if requested == &target
            ) {
                return None;
            }
            set_signature_dump(app, target, records, None);
        }
        Action::SignatureDumpPartial {
            target,
            records,
            limitations,
        } => {
            if !matches!(
                &app.signature_dump,
                SignatureDumpState::Loading { target: requested } if requested == &target
            ) {
                return None;
            }
            set_signature_dump(app, target, records, Some(limitations));
        }
        Action::SignatureDumpFailed { target, message } => {
            if !matches!(
                &app.signature_dump,
                SignatureDumpState::Loading { target: requested } if requested == &target
            ) {
                return None;
            }
            app.signature_dump = SignatureDumpState::Failed {
                target,
                message: message.clone(),
            };
            app.notification = Some(format!("Signature dump is unavailable: {message}"));
        }
        Action::SelectSignatureRecord { delta } => {
            let records = app.signature_dump.records()?;
            if records.is_empty() {
                app.signature_selection = None;
                return None;
            }
            let current = app
                .signature_selection
                .as_ref()
                .and_then(|selected| {
                    records
                        .iter()
                        .position(|record| &record.identity == selected)
                })
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(records.len().saturating_sub(1))
            };
            app.signature_selection = Some(records[next].identity.clone());
        }
        Action::SetSelectedSignatureComparisonSide(side) => {
            let Some(selected) = app.signature_selection.clone() else {
                app.notification = Some("No signature record is selected.".into());
                return None;
            };
            if !app
                .signature_dump
                .records()
                .is_some_and(|records| records.iter().any(|record| record.identity == selected))
            {
                app.notification =
                    Some("The selected signature is not in the current dump result.".into());
                return None;
            }
            let (mut left, mut right) = signature_comparison_inputs(&app.signature_comparison);
            match side {
                SignatureComparisonSide::Left => left = Some(selected),
                SignatureComparisonSide::Right => right = Some(selected),
            }
            app.signature_comparison = SignatureComparisonState::Ready { left, right };
        }
        Action::BeginSignatureComparison => {
            let (Some(left), Some(right)) = signature_comparison_inputs(&app.signature_comparison)
            else {
                app.notification =
                    Some("Select both left and right signature records before comparing.".into());
                return None;
            };
            let request = SignatureComparisonRequest { left, right };
            if let Err(message) = request.validate() {
                app.notification = Some(message.into());
                return None;
            }
            if !app.signature_dump.records().is_some_and(|records| {
                records.iter().any(|record| record.identity == request.left)
                    && records
                        .iter()
                        .any(|record| record.identity == request.right)
            }) {
                app.notification = Some(
                    "Both signature comparison inputs must be in the current dump result.".into(),
                );
                return None;
            }
            app.signature_comparison = SignatureComparisonState::Loading {
                request: request.clone(),
            };
            return Some(Effect::CompareSignatures(request));
        }
        Action::SignatureComparisonLoaded {
            request,
            differences,
        } => {
            if !matches!(
                &app.signature_comparison,
                SignatureComparisonState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            let (differences, report) =
                normalize_signature_differences(differences, MAX_SIGNATURE_DIFFERENCES);
            app.signature_comparison = if report.is_partial() {
                SignatureComparisonState::Partial {
                    request,
                    differences,
                    limitations: vec![format!(
                        "Model bounds truncated {} signature differences.",
                        report.truncated_differences
                    )],
                }
            } else if differences.is_empty() {
                SignatureComparisonState::AvailableEmpty { request }
            } else {
                SignatureComparisonState::Available {
                    request,
                    differences,
                }
            };
        }
        Action::SignatureComparisonPartial {
            request,
            differences,
            mut limitations,
        } => {
            if !matches!(
                &app.signature_comparison,
                SignatureComparisonState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            let (differences, report) =
                normalize_signature_differences(differences, MAX_SIGNATURE_DIFFERENCES);
            if report.is_partial() {
                limitations.push(format!(
                    "Model bounds truncated {} signature differences.",
                    report.truncated_differences
                ));
            }
            app.signature_comparison = SignatureComparisonState::Partial {
                request,
                differences,
                limitations,
            };
        }
        Action::SignatureComparisonFailed { request, message } => {
            if !matches!(
                &app.signature_comparison,
                SignatureComparisonState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            app.signature_comparison = SignatureComparisonState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Signature comparison failed: {message}"));
        }
        Action::BeginPackageInventory => {
            if package_operation_is_loading(app) {
                app.notification = Some("A package-data operation is already running.".into());
                return None;
            }
            return Some(begin_package_inventory(app));
        }
        Action::RefreshPackageInventory => {
            if package_operation_is_loading(app) {
                app.notification = Some("A package-data operation is already running.".into());
                return None;
            }
            return Some(begin_package_inventory(app));
        }
        Action::CancelPackageOperation => {
            if package_operation_is_loading(app) {
                return Some(Effect::CancelPackageOperation);
            }
            app.notification = Some("No package-data operation is running.".into());
        }
        Action::PackageInventoryLoaded { request, packages } => {
            if !matches!(
                app.package_inventory,
                PackageInventoryState::Loading { request: pending } if pending == request
            ) {
                return None;
            }
            set_package_inventory(app, request, packages, None);
        }
        Action::PackageInventoryPartial {
            request,
            packages,
            limitations,
        } => {
            if !matches!(
                app.package_inventory,
                PackageInventoryState::Loading { request: pending } if pending == request
            ) {
                return None;
            }
            set_package_inventory(app, request, packages, Some(limitations));
        }
        Action::PackageInventoryFailed { request, message } => {
            if !matches!(
                app.package_inventory,
                PackageInventoryState::Loading { request: pending } if pending == request
            ) {
                return None;
            }
            app.package_inventory = PackageInventoryState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Package inventory is unavailable: {message}"));
        }
        Action::SelectPackage { delta } => {
            let visible = app
                .filtered_packages()
                .into_iter()
                .map(|package| package.identity.clone())
                .collect::<Vec<_>>();
            if visible.is_empty() {
                app.package_selection = None;
                return None;
            }
            let current = app
                .package_selection
                .as_ref()
                .and_then(|identity| visible.iter().position(|candidate| candidate == identity))
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(visible.len().saturating_sub(1))
            };
            app.package_selection = Some(visible[next].clone());
            app.package_dependency_selection = 0;
        }
        Action::BeginPackageSearch => app.package_searching = true,
        Action::AppendPackageQuery(character) => {
            if !character.is_control() && app.package_query.len() < 256 {
                app.package_query.push(character);
                set_package_selection_to_current_or_first(app, app.package_selection.clone());
            }
        }
        Action::BackspacePackageQuery => {
            app.package_query.pop();
            set_package_selection_to_current_or_first(app, app.package_selection.clone());
        }
        Action::FinishPackageSearch => app.package_searching = false,
        Action::BeginSelectedPackageDetail => {
            let Some(identity) = app
                .selected_package()
                .map(|package| package.identity.clone())
            else {
                app.notification = Some("No current package is selected for inspection.".into());
                return None;
            };
            if package_operation_is_loading(app) {
                app.notification = Some("A package-data operation is already running.".into());
                return None;
            }
            return Some(begin_package_detail(app, identity));
        }
        Action::PackageDetailLoaded { request, detail } => {
            if !app.package_details.get(&request.identity).is_some_and(
                |state| matches!(state, PackageDetailState::Loading { request: pending } if pending == &request),
            ) {
                return None;
            }
            let (detail, report) = normalize_package_detail(&request.identity, detail);
            let Some(detail) = detail else {
                app.package_details.insert(
                    request.identity.clone(),
                    PackageDetailState::Failed {
                        request,
                        message: "backend returned detail for a different or invalid package"
                            .into(),
                    },
                );
                return None;
            };
            let mut limitations = Vec::new();
            append_package_normalization_limitations(&mut limitations, &report);
            let limitations = normalize_package_limitations(limitations);
            let state = if !limitations.is_empty() {
                PackageDetailState::Partial {
                    request,
                    detail,
                    limitations,
                }
            } else if package_detail_is_empty(&detail) {
                PackageDetailState::AvailableEmpty { request }
            } else {
                PackageDetailState::Available { request, detail }
            };
            app.package_details
                .insert(state.request().unwrap().identity.clone(), state);
            app.package_dependency_selection = 0;
        }
        Action::PackageDetailPartial {
            request,
            detail,
            mut limitations,
        } => {
            if !app.package_details.get(&request.identity).is_some_and(
                |state| matches!(state, PackageDetailState::Loading { request: pending } if pending == &request),
            ) {
                return None;
            }
            let (detail, report) = normalize_package_detail(&request.identity, detail);
            let Some(detail) = detail else {
                app.package_details.insert(
                    request.identity.clone(),
                    PackageDetailState::Failed {
                        request,
                        message: "backend returned detail for a different or invalid package"
                            .into(),
                    },
                );
                return None;
            };
            append_package_normalization_limitations(&mut limitations, &report);
            let limitations = normalize_package_limitations(limitations);
            app.package_details.insert(
                request.identity.clone(),
                PackageDetailState::Partial {
                    request,
                    detail,
                    limitations,
                },
            );
            app.package_dependency_selection = 0;
        }
        Action::PackageDetailFailed { request, message } => {
            if !app.package_details.get(&request.identity).is_some_and(
                |state| matches!(state, PackageDetailState::Loading { request: pending } if pending == &request),
            ) {
                return None;
            }
            app.package_details.insert(
                request.identity.clone(),
                PackageDetailState::Failed {
                    request,
                    message: message.clone(),
                },
            );
            app.notification = Some(format!("Package detail is unavailable: {message}"));
        }
        Action::OpenPackageDependency { identity, reverse } => {
            let available = app
                .selected_package_detail()
                .and_then(PackageDetailState::detail)
                .and_then(|detail| {
                    if reverse {
                        detail.reverse_dependencies.available()
                    } else {
                        detail.runtime_dependencies.available()
                    }
                })
                .is_some_and(|dependencies| dependencies.contains(&identity));
            if !available {
                app.notification = Some(
                    "The requested package dependency is not in the current typed detail.".into(),
                );
                return None;
            }
            if app
                .package_inventory
                .packages()
                .is_some_and(|packages| packages.iter().any(|package| package.identity == identity))
            {
                return select_package_identity(app, identity, false);
            } else {
                app.notification =
                    Some("The dependency is not present in the current package inventory.".into());
            }
        }
        Action::TogglePackageDependencyKind => {
            app.package_dependency_reverse = !app.package_dependency_reverse;
            app.package_dependency_selection = 0;
        }
        Action::SelectPackageDependency { delta } => {
            let count = app
                .selected_package_dependencies()
                .map_or(0, <[PackageIdentity]>::len);
            app.package_dependency_selection = if delta.is_negative() {
                app.package_dependency_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.package_dependency_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::OpenSelectedPackageDependency => {
            let Some(identity) = app.selected_package_dependency().cloned() else {
                app.notification = Some(format!(
                    "No {} dependency is selected.",
                    if app.package_dependency_reverse {
                        "reverse"
                    } else {
                        "runtime"
                    }
                ));
                return None;
            };
            return select_package_identity(app, identity, true);
        }
        Action::BackPackageNavigation => {
            let Some(identity) = app.package_navigation.pop() else {
                app.notification = Some("Package navigation history is empty.".into());
                return None;
            };
            app.package_selection = Some(identity);
            app.package_dependency_selection = 0;
        }
        Action::OpenSelectedPackageRecipe => {
            let Some(PackageField::Available(recipe)) =
                app.selected_package().map(|package| &package.recipe)
            else {
                app.notification =
                    Some("The selected package has no authoritative recipe identity.".into());
                return None;
            };
            let Some(index) = app
                .workspace
                .recipes
                .iter()
                .position(|candidate| candidate.name == *recipe)
            else {
                app.notification = Some(format!(
                    "Recipe {recipe} is not present in the current workspace inventory."
                ));
                return None;
            };
            app.recipe_selection = index;
            app.screen = Screen::Recipes;
            app.focus = FocusTarget::Workspace;
        }
        Action::OpenSelectedPackageProvider => {
            let Some(PackageField::Available(provider)) =
                app.selected_package().map(|package| &package.provider)
            else {
                app.notification =
                    Some("The selected package has no authoritative provider path.".into());
                return None;
            };
            if !provider.is_absolute() {
                app.notification =
                    Some("The selected package provider path is not absolute.".into());
                return None;
            }
            return Some(Effect::OpenInEditor(provider.clone()));
        }
        Action::BeginSelectedRecipeMetadata => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                app.recipe_metadata_loading.insert(recipe.name.clone());
                app.recipe_metadata_errors.remove(&recipe.name);
                return Some(Effect::GetRecipeMetadata(recipe.name.clone()));
            }
            app.notification = Some("No recipe is selected for metadata inspection.".into());
        }
        Action::RecipeMetadataLoaded(metadata) => {
            let recipe = metadata.recipe.clone();
            app.recipe_metadata_loading.remove(&recipe);
            app.recipe_metadata_errors.remove(&recipe);
            if let Some(sources) = metadata.sources.as_ref() {
                app.recipe_sources.insert(recipe.clone(), sources.clone());
            } else {
                app.recipe_sources.remove(&recipe);
            }
            app.recipe_metadata.insert(recipe, metadata);
        }
        Action::RecipeMetadataFailed { recipe, message } => {
            app.recipe_metadata_loading.remove(&recipe);
            app.recipe_metadata_errors
                .insert(recipe.clone(), message.clone());
            app.notification = Some(format!(
                "Recipe metadata for {recipe} is unavailable: {message}"
            ));
        }
        Action::DependenciesLoaded(dependencies) => {
            app.screen = Screen::Dependencies;
            let root = DependencyNodeId::recipe(dependencies.recipe.clone());
            let edges = dependencies
                .build
                .iter()
                .map(|name| DependencyEdge {
                    from: root.clone(),
                    to: DependencyNodeId::recipe(name),
                    kind: DependencyEdgeKind::Build,
                })
                .chain(dependencies.runtime.iter().map(|name| DependencyEdge {
                    from: root.clone(),
                    to: DependencyNodeId::recipe(name),
                    kind: DependencyEdgeKind::Runtime,
                }))
                .collect::<Vec<_>>();
            let (graph, _) =
                DependencyGraph::normalize(root, Vec::new(), edges, usize::MAX, usize::MAX);
            set_dependency_graph(app, graph, None);
            app.dependencies = Some(dependencies);
            app.dependency_selection = 0;
        }
        Action::SelectDependency { delta } => {
            let count = app.dependencies.as_ref().map_or(0, |dependencies| {
                dependencies.build.len() + dependencies.runtime.len()
            });
            app.dependency_selection = if delta.is_negative() {
                app.dependency_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.dependency_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::OpenSelectedDependency => {
            let selected = app.dependencies.as_ref().and_then(|dependencies| {
                dependencies
                    .build
                    .iter()
                    .chain(dependencies.runtime.iter())
                    .nth(app.dependency_selection)
            });
            if let Some(name) = selected {
                if let Some(index) = app
                    .workspace
                    .recipes
                    .iter()
                    .position(|recipe| recipe.name == *name)
                {
                    app.recipe_selection = index;
                    app.screen = Screen::Recipes;
                } else {
                    app.notification = Some(format!(
                        "{name} is a dependency but is not an available recipe in this workspace."
                    ));
                }
            }
        }
        Action::ConfirmRecipeTask => {
            if let Some(Dialog::RecipeTaskConfirmation(request)) = app.active_dialog().cloned() {
                close_dialog(app);
                prepare_build(app, request.targets.first().cloned());
                synchronize_focus(app);
                return Some(Effect::Start(request));
            }
        }
        Action::CancelRecipeTask => {
            if matches!(app.active_dialog(), Some(Dialog::RecipeTaskConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolModify => {
            if let Some(Dialog::DevtoolModifyConfirmation(identity)) = app.active_dialog().cloned()
            {
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolModify(identity));
            }
        }
        Action::CancelDevtoolModify => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolModifyConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolReset => {
            if let Some(Dialog::DevtoolResetConfirmation(plan)) = app.active_dialog().cloned() {
                let Some(status) = app.devtool_statuses.get(&plan.identity) else {
                    app.notification =
                        Some("Authoritative Devtool status expired; refresh with t.".into());
                    return None;
                };
                if let Some(reason) = status.disabled_reason(DevtoolAction::Reset) {
                    app.notification = Some(reason);
                    return None;
                }
                let current_source = match &status.workspace {
                    DevtoolWorkspace::Present { source_path, .. }
                    | DevtoolWorkspace::MissingDirectory { source_path } => source_path,
                    DevtoolWorkspace::NotMember => return None,
                };
                if current_source != &plan.source_path || !current_source.is_absolute() {
                    app.notification = Some(
                        "The authoritative Devtool reset source changed; refresh with t.".into(),
                    );
                    return None;
                }
                if let Err(error) = plan.operation().validate() {
                    app.notification = Some(error.to_string());
                    return None;
                }
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolReset(plan));
            }
        }
        Action::CancelDevtoolReset => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolResetConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolUpdateRecipe => {
            if let Some(Dialog::DevtoolUpdateConfirmation(identity)) = app.active_dialog().cloned()
            {
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolUpdateRecipe(identity));
            }
        }
        Action::CancelDevtoolUpdateRecipe => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolUpdateConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::SelectDevtoolFinishLayer { delta } => {
            if let Some(Dialog::DevtoolFinishPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.layers.len().saturating_sub(1))
                };
            }
        }
        Action::PreviewDevtoolFinish => {
            if let Some(Dialog::DevtoolFinishPicker(picker)) = app.active_dialog() {
                let Some(layer) = picker.layers.get(picker.selection).cloned() else {
                    app.notification = Some("Select a configured finish layer.".into());
                    return None;
                };
                if !layer.path.is_absolute()
                    || !app.workspace.layers.iter().any(|configured| {
                        configured.name == layer.name && configured.path == layer.path
                    })
                {
                    app.notification =
                        Some("The selected finish layer is no longer configured.".into());
                    return None;
                }
                replace_dialog(
                    app,
                    Dialog::DevtoolFinishConfirmation(DevtoolFinishPlan {
                        identity: picker.identity.clone(),
                        layer,
                    }),
                );
            }
        }
        Action::CancelDevtoolFinish => {
            if matches!(app.active_dialog(), Some(Dialog::DevtoolFinishPicker(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolFinish => {
            if let Some(Dialog::DevtoolFinishConfirmation(plan)) = app.active_dialog().cloned() {
                let Some(status) = app.devtool_statuses.get(&plan.identity) else {
                    app.notification =
                        Some("Authoritative Devtool status expired; refresh with t.".into());
                    return None;
                };
                if let Some(reason) = status.disabled_reason(DevtoolAction::Finish) {
                    app.notification = Some(reason);
                    return None;
                }
                if !plan.layer.path.is_absolute()
                    || !app.workspace.layers.iter().any(|configured| {
                        configured.name == plan.layer.name && configured.path == plan.layer.path
                    })
                {
                    app.notification =
                        Some("The selected finish layer is no longer configured.".into());
                    return None;
                }
                if let Err(error) = DevtoolOperation::from(plan.request()).validate() {
                    app.notification = Some(error.to_string());
                    return None;
                }
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolFinish(plan));
            }
        }
        Action::CancelDevtoolFinishConfirmation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolFinishConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::AppendDevtoolDeployTarget(character) => {
            if let Some(Dialog::DevtoolDeploy(draft)) = app.active_dialog_mut() {
                draft.target.push(character);
            }
        }
        Action::BackspaceDevtoolDeployTarget => {
            if let Some(Dialog::DevtoolDeploy(draft)) = app.active_dialog_mut() {
                draft.target.pop();
            }
        }
        Action::PreviewDevtoolDeploy => {
            if let Some(Dialog::DevtoolDeploy(draft)) = app.active_dialog() {
                let plan = DevtoolDeployPlan {
                    identity: draft.identity.clone(),
                    target: draft.target.clone(),
                };
                if let Err(error) = DevtoolOperation::from(plan.request()).validate() {
                    app.notification = Some(error.to_string());
                    return None;
                }
                replace_dialog(app, Dialog::DevtoolDeployConfirmation(plan));
            }
        }
        Action::CancelDevtoolDeploy => {
            if matches!(app.active_dialog(), Some(Dialog::DevtoolDeploy(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolDeploy => {
            if let Some(Dialog::DevtoolDeployConfirmation(plan)) = app.active_dialog().cloned() {
                let Some(status) = app.devtool_statuses.get(&plan.identity) else {
                    app.notification =
                        Some("Authoritative Devtool status expired; refresh with t.".into());
                    return None;
                };
                if let Some(reason) = status.disabled_reason(DevtoolAction::Deploy) {
                    app.notification = Some(reason);
                    return None;
                }
                if let Err(error) = DevtoolOperation::from(plan.request()).validate() {
                    app.notification = Some(error.to_string());
                    return None;
                }
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolDeploy(plan));
            }
        }
        Action::CancelDevtoolDeployConfirmation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolDeployConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::OpenRecipeEditor {
            recipe,
            root,
            files,
        } => {
            open_dialog(
                app,
                Dialog::RecipeEditor(RecipeEditor {
                    recipe,
                    root,
                    files,
                    selection: 0,
                    content: String::new(),
                    editing: false,
                    dirty: false,
                }),
            );
            if let Some(path) = app.active_dialog().and_then(|dialog| match dialog {
                Dialog::RecipeEditor(editor) => editor.selected_path(),
                _ => None,
            }) {
                synchronize_focus(app);
                return Some(Effect::LoadRecipeEditorFile(path));
            }
            app.notification = Some("The Devtool workspace contains no editable files.".into());
        }
        Action::SelectRecipeEditorFile { delta } => {
            let path = if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                if editor.dirty {
                    app.notification =
                        Some("Save changes with Ctrl+S before selecting another file.".into());
                    None
                } else {
                    editor.selection = if delta.is_negative() {
                        editor.selection.saturating_sub(delta.unsigned_abs())
                    } else {
                        editor
                            .selection
                            .saturating_add(delta as usize)
                            .min(editor.files.len().saturating_sub(1))
                    };
                    editor.selected_path()
                }
            } else {
                None
            };
            if let Some(path) = path {
                return Some(Effect::LoadRecipeEditorFile(path));
            }
        }
        Action::LoadRecipeEditorContent(content) => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                editor.content = content;
                editor.editing = false;
                editor.dirty = false;
            }
        }
        Action::ToggleRecipeEditorEditing => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendRecipeEditor(character) => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.content.push(character);
                editor.dirty = true;
            }
        }
        Action::BackspaceRecipeEditor => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.content.pop();
                editor.dirty = true;
            }
        }
        Action::SaveRecipeEditor => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog()
                && editor.dirty
                && let Some(path) = editor.selected_path()
            {
                return Some(Effect::SaveRecipeEditorFile {
                    path,
                    content: editor.content.clone(),
                });
            }
        }
        Action::RecipeEditorSaved => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
                editor.dirty = false;
                app.notification = Some("Recipe file saved. Press Esc to return to Yoctui.".into());
            }
        }
        Action::BeginRecipeEditorBuild => {
            if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog().cloned() {
                if editor.dirty {
                    app.notification =
                        Some("Save workspace changes before starting the recipe build.".into());
                } else {
                    let recipe = editor.recipe;
                    close_dialog(app);
                    begin_recipe_task_for(app, &recipe, None, false);
                }
            }
        }
        Action::CloseRecipeEditor => {
            if matches!(app.active_dialog(), Some(Dialog::RecipeEditor(_))) {
                close_dialog(app);
            }
        }
        Action::SelectLayer { delta } => {
            app.layer_selection = if delta.is_negative() {
                app.layer_selection.saturating_sub(delta.unsigned_abs())
            } else {
                app.layer_selection
                    .saturating_add(delta as usize)
                    .min(app.workspace.layers.len().saturating_sub(1))
            };
        }
        Action::OpenSelectedLayer => {
            if let Some(layer) = app.workspace.layers.get(app.layer_selection) {
                return Some(Effect::OpenInEditor(layer.path.clone()));
            }
            app.notification = Some("No layer is selected to open.".into());
        }
        Action::BeginSelectedLayerWorkspaceEditor => {
            if let Some(layer) = app.workspace.layers.get(app.layer_selection) {
                return Some(Effect::OpenWorkspaceEditor {
                    label: format!("Layer: {}", layer.name),
                    root: layer.path.clone(),
                });
            }
            app.notification = Some("No layer is selected to edit.".into());
        }
        Action::BeginSelectedLayerBrowser => {
            if let Some(layer) = app.workspace.layers.get(app.layer_selection) {
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: layer.name.clone(),
                    root: layer.path.clone(),
                    directory: layer.path.clone(),
                });
            }
            app.notification = Some("No layer is selected to browse.".into());
        }
        Action::LoadLayerBrowserDirectory {
            layer,
            root,
            directory,
            mut entries,
        } => {
            entries.sort_by_key(|entry| {
                (
                    !entry.is_dir,
                    entry.path.file_name().map(|name| name.to_owned()),
                )
            });
            let preferred = app
                .layer_browser
                .as_ref()
                .and_then(LayerBrowser::selected_entry)
                .map(|entry| entry.path.clone());
            if let Some(browser) = app
                .layer_browser
                .as_mut()
                .filter(|browser| browser.layer == layer && browser.root == root)
            {
                browser.directory = directory.clone();
                browser.nodes.insert(directory.clone(), entries);
                browser.expanded.insert(directory);
                browser.rebuild(preferred.as_ref());
            } else {
                let mut browser = LayerBrowser::new(layer, root);
                browser.directory = directory.clone();
                browser.nodes.insert(directory, entries);
                browser.rebuild(None);
                app.layer_browser = Some(browser);
            }
            if let Some(path) = app
                .layer_browser
                .as_ref()
                .and_then(LayerBrowser::selected_entry)
                .filter(|entry| !entry.is_dir)
                .map(|entry| entry.path.clone())
            {
                return Some(Effect::LoadLayerBrowserPreview(path));
            }
        }
        Action::SelectLayerBrowserEntry { delta } => {
            let query = app.metadata_query.to_ascii_lowercase();
            let path = if let Some(browser) = app.layer_browser.as_mut() {
                let matches = browser
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| {
                        query.is_empty()
                            || entry
                                .path
                                .to_string_lossy()
                                .to_ascii_lowercase()
                                .contains(&query)
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                let position = matches
                    .iter()
                    .position(|index| *index == browser.selection)
                    .unwrap_or(0);
                let position = if delta.is_negative() {
                    position.saturating_sub(delta.unsigned_abs())
                } else {
                    position
                        .saturating_add(delta as usize)
                        .min(matches.len().saturating_sub(1))
                };
                browser.selection = matches.get(position).copied().unwrap_or(0);
                if browser.selected_entry().is_some_and(|entry| entry.is_dir) {
                    browser.preview.clear();
                    browser.preview_kind = PreviewKind::Unavailable;
                    browser.preview_truncated = false;
                }
                browser
                    .selected_entry()
                    .filter(|entry| !entry.is_dir)
                    .map(|entry| entry.path.clone())
            } else {
                None
            };
            if let Some(path) = path {
                return Some(Effect::LoadLayerBrowserPreview(path));
            }
        }
        Action::LayerBrowserExpand => {
            let selected = app
                .layer_browser
                .as_ref()
                .and_then(LayerBrowser::selected_entry)
                .cloned();
            if let Some(entry) = selected.filter(|entry| entry.is_dir) {
                let browser = app.layer_browser.as_mut().expect("browser was selected");
                if browser.expanded.contains(&entry.path) {
                    return None;
                }
                if browser.nodes.contains_key(&entry.path) {
                    browser.expanded.insert(entry.path.clone());
                    browser.rebuild(Some(&entry.path));
                    return None;
                }
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    directory: entry.path,
                });
            }
        }
        Action::LayerBrowserEnter => {
            let selected = app
                .layer_browser
                .as_ref()
                .and_then(LayerBrowser::selected_entry)
                .cloned();
            if let Some(entry) = selected.filter(|entry| entry.is_dir) {
                let browser = app.layer_browser.as_mut().expect("browser was selected");
                if browser.expanded.remove(&entry.path) {
                    browser.rebuild(Some(&entry.path));
                    return None;
                }
                if browser.nodes.contains_key(&entry.path) {
                    browser.expanded.insert(entry.path.clone());
                    browser.rebuild(Some(&entry.path));
                    return None;
                }
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    directory: entry.path,
                });
            }
            return update(app, Action::EditSelectedLayerBrowserFile);
        }
        Action::LayerBrowserUp => {
            if let Some(browser) = app.layer_browser.as_mut()
                && let Some(entry) = browser.selected_entry().cloned()
            {
                if entry.is_dir && browser.expanded.remove(&entry.path) {
                    browser.rebuild(Some(&entry.path));
                } else if let Some(parent) = entry.path.parent()
                    && parent != browser.root
                    && let Some(index) = browser
                        .entries
                        .iter()
                        .position(|candidate| candidate.path == parent)
                {
                    browser.selection = index;
                }
            }
        }
        Action::CloseLayerBrowser => app.layer_browser = None,
        Action::RefreshLayerBrowser => {
            if let Some(browser) = app.layer_browser.as_ref() {
                let directory = browser
                    .selected_entry()
                    .map(|entry| {
                        if entry.is_dir {
                            entry.path.clone()
                        } else {
                            entry.path.parent().unwrap_or(&browser.root).to_path_buf()
                        }
                    })
                    .unwrap_or_else(|| browser.root.clone());
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    directory,
                });
            }
        }
        Action::ToggleLayerBrowserHidden => {
            if let Some(browser) = app.layer_browser.as_mut() {
                let selected = browser.selected_entry().map(|entry| entry.path.clone());
                browser.show_hidden = !browser.show_hidden;
                browser.rebuild(selected.as_ref());
            }
        }
        Action::SetLayerInspectorMode(mode) => {
            if let Some(browser) = app.layer_browser.as_mut() {
                browser.inspector_mode = mode;
            }
        }
        Action::LoadLayerBrowserPreview {
            path,
            content,
            kind,
            truncated,
        } => {
            if let Some(browser) = app.layer_browser.as_mut()
                && browser
                    .selected_entry()
                    .is_some_and(|entry| entry.path == path && !entry.is_dir)
            {
                browser.preview = content;
                browser.preview_kind = kind;
                browser.preview_truncated = truncated;
            }
        }
        Action::EditSelectedLayerBrowserFile => {
            if let Some(browser) = app.layer_browser.as_ref()
                && let Some(entry) = browser.selected_entry()
                && !entry.is_dir
                && let Ok(file) = entry.path.strip_prefix(&browser.root)
            {
                return Some(Effect::OpenLayerBrowserEditor {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    file: file.to_path_buf(),
                });
            }
            app.notification = Some("Select a file to edit.".into());
        }
        Action::BeginLayerRelationships => return Some(Effect::GetLayerRelationships),
        Action::LayerRelationshipsLoaded(relationships) => {
            app.layer_relationships = Some(relationships);
            app.screen = Screen::LayerRelationships;
        }
        Action::SelectConfigVariable { delta } => {
            let count = filtered_config_identities(app).len();
            app.config_selection = if delta.is_negative() {
                app.config_selection.saturating_sub(delta.unsigned_abs())
            } else {
                app.config_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::BeginSelectedConfigDetail => {
            let Some(identity) = selected_config_identity(app) else {
                app.notification = Some("No configuration variable is selected to inspect.".into());
                return None;
            };
            app.variable_detail_loading.insert(identity.clone());
            app.variable_detail_errors.remove(&identity);
            return Some(Effect::GetVariable(identity));
        }
        Action::CopySelectedConfigEffective => {
            match selected_config_copy_value(app, ConfigCopyValue::Effective) {
                Ok(value) => return Some(Effect::CopyToClipboard(value.to_owned())),
                Err(reason) => app.notification = Some(reason),
            }
        }
        Action::CopySelectedConfigUnexpanded => {
            match selected_config_copy_value(app, ConfigCopyValue::Unexpanded) {
                Ok(value) => return Some(Effect::CopyToClipboard(value.to_owned())),
                Err(reason) => app.notification = Some(reason),
            }
        }
        Action::VariableDetailFailed { identity, message } => {
            app.variable_detail_loading.remove(&identity);
            app.variable_detail_errors
                .insert(identity.clone(), message.clone());
            app.notification = Some(format!(
                "Configuration detail for {} is unavailable: {message}",
                identity.name
            ));
        }
        Action::OpenSelectedConfigSource => match selected_config_sources(app) {
            Ok((_, sources)) if sources.len() == 1 => {
                match resolve_config_source(app, &sources[0].path) {
                    Ok(path) => return Some(Effect::OpenInEditor(path)),
                    Err(reason) => app.notification = Some(reason),
                }
            }
            Ok((identity, sources)) => {
                open_dialog(
                    app,
                    Dialog::ConfigSourcePicker(ConfigSourcePicker {
                        identity,
                        sources,
                        selection: 0,
                    }),
                );
            }
            Err(reason) => app.notification = Some(reason),
        },
        Action::SelectConfigSource { delta } => {
            if let Some(Dialog::ConfigSourcePicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.sources.len().saturating_sub(1))
                };
            }
        }
        Action::OpenSelectedConfigSourceChoice => {
            let path = app.active_dialog().and_then(|dialog| match dialog {
                Dialog::ConfigSourcePicker(picker) => picker
                    .sources
                    .get(picker.selection)
                    .map(|source| source.path.clone()),
                _ => None,
            });
            if let Some(path) = path {
                match resolve_config_source(app, &path) {
                    Ok(path) => {
                        close_dialog(app);
                        synchronize_focus(app);
                        return Some(Effect::OpenInEditor(path));
                    }
                    Err(reason) => app.notification = Some(reason),
                }
            } else {
                app.notification = Some("The selected configuration source is stale.".into());
            }
        }
        Action::CancelConfigSourcePicker => {
            if matches!(app.active_dialog(), Some(Dialog::ConfigSourcePicker(_))) {
                close_dialog(app);
            }
        }
        Action::OpenConfigScopePicker => {
            let Some(identity) = selected_config_identity(app) else {
                app.notification =
                    Some("No configuration variable is selected for scope inspection.".into());
                return None;
            };
            let mut recipes = app
                .workspace
                .recipes
                .iter()
                .map(|recipe| recipe.name.clone())
                .collect::<Vec<_>>();
            recipes.sort();
            recipes.dedup();
            let mut scopes = vec![None];
            scopes.extend(recipes.into_iter().map(Some));
            let selection = scopes
                .iter()
                .position(|scope| scope == &app.config_scope)
                .unwrap_or(0);
            open_dialog(
                app,
                Dialog::ConfigScopePicker(ConfigScopePicker {
                    variable: identity.name,
                    scopes,
                    selection,
                }),
            );
        }
        Action::SelectConfigScope { delta } => {
            if let Some(Dialog::ConfigScopePicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.scopes.len().saturating_sub(1))
                };
            }
        }
        Action::ConfirmConfigScope => {
            let scope = if let Some(Dialog::ConfigScopePicker(picker)) = app.active_dialog() {
                picker.scopes.get(picker.selection).cloned()
            } else {
                None
            };
            let Some(scope) = scope else {
                app.notification = Some("The selected configuration scope is stale.".into());
                return None;
            };
            app.config_scope = scope;
            close_dialog(app);
            synchronize_focus(app);
            let Some(identity) = selected_config_identity(app) else {
                app.notification =
                    Some("No configuration variable is selected for scope inspection.".into());
                return None;
            };
            app.variable_detail_loading.insert(identity.clone());
            app.variable_detail_errors.remove(&identity);
            return Some(Effect::GetVariable(identity));
        }
        Action::CancelConfigScopePicker => {
            if matches!(app.active_dialog(), Some(Dialog::ConfigScopePicker(_))) {
                close_dialog(app);
            }
        }
        Action::OpenConfigComparison => match config_comparison(app) {
            Ok(comparison) => open_dialog(app, Dialog::ConfigComparison(comparison)),
            Err(reason) => app.notification = Some(reason),
        },
        Action::CloseConfigComparison => {
            if matches!(app.active_dialog(), Some(Dialog::ConfigComparison(_))) {
                close_dialog(app);
            }
        }
        Action::BeginConfigEdit => match config_edit_context(app) {
            Ok((identity, value, _)) => {
                let mut editor =
                    PopupEditor::new(popup_toml_document("value", &value, Some(&identity.name)));
                let _ = editor.select_toml_value("value");
                open_dialog(app, Dialog::ConfigEdit { identity, editor });
            }
            Err(reason) => app.notification = Some(reason),
        },
        Action::ToggleConfigEdit => {
            if let Some(Dialog::ConfigEdit { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendConfigEdit(character) => {
            if character.is_control() {
                app.notification =
                    Some("Configuration values cannot contain control characters.".into());
            } else if let Some(Dialog::ConfigEdit { editor, .. }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceConfigEdit => {
            if let Some(Dialog::ConfigEdit { editor, .. }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::PreviewConfigEdit => {
            let edit = app.active_dialog().and_then(|dialog| match dialog {
                Dialog::ConfigEdit {
                    identity, editor, ..
                } => Some((identity.clone(), popup_toml_value(&editor.text, "value"))),
                _ => None,
            });
            let (identity, value) = edit?;
            let value = match value {
                Ok(value) => value,
                Err(reason) => {
                    app.notification = Some(reason);
                    return None;
                }
            };
            match config_edit_assignment(&identity.name, &value) {
                Ok(assignment) => {
                    let Some(build_dir) = app.workspace.build_dir.as_ref() else {
                        app.notification =
                            Some("An active build directory is required for editing.".into());
                        return None;
                    };
                    replace_dialog(
                        app,
                        Dialog::ConfigEditConfirmation(ConfigEditRequest {
                            identity,
                            value,
                            destination: build_dir.join("conf/local.conf"),
                            assignment,
                        }),
                    );
                }
                Err(reason) => app.notification = Some(reason),
            }
        }
        Action::CancelConfigEdit => {
            if matches!(app.active_dialog(), Some(Dialog::ConfigEdit { .. })) {
                close_dialog(app);
            }
        }
        Action::ConfirmConfigEdit => {
            if let Some(Dialog::ConfigEditConfirmation(request)) = app.active_dialog().cloned() {
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::WriteConfigAssignment(request));
            }
        }
        Action::CancelConfigEditConfirmation => {
            if matches!(app.active_dialog(), Some(Dialog::ConfigEditConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfigEditWriteSucceeded { identity } => {
            if identity.recipe.is_some()
                || !EDITABLE_CONFIG_VARIABLES.contains(&identity.name.as_str())
            {
                app.notification =
                    Some("The completed configuration edit identity is invalid.".into());
                return None;
            }
            app.variable_detail_loading.insert(identity.clone());
            app.variable_detail_errors.remove(&identity);
            app.notification = Some(format!(
                "{} was saved; refreshing authoritative configuration detail.",
                identity.name
            ));
            return Some(Effect::GetVariable(identity));
        }
        Action::ConfigEditWriteFailed { identity, message } => {
            app.notification = Some(format!(
                "Could not save configuration variable {}: {message}",
                identity.name
            ));
        }
        Action::ConfigEditRefreshSucceeded { identity } => {
            app.notification = Some(format!("{} saved and refreshed.", identity.name));
        }
        Action::ConfigEditRefreshFailed { identity, message } => {
            app.variable_detail_loading.remove(&identity);
            app.notification = Some(format!(
                "{} was saved, but authoritative refresh failed: {message}",
                identity.name
            ));
        }
        Action::BeginBbmaskEdit => {
            let input = app
                .workspace
                .variables
                .get("BBMASK")
                .cloned()
                .unwrap_or_default();
            let mut editor = PopupEditor::new(popup_toml_document("bbmask", &input, None));
            let _ = editor.select_toml_value("bbmask");
            open_dialog(app, Dialog::BbmaskEdit(editor));
        }
        Action::ToggleBbmaskEdit => {
            if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendBbmask(character) => {
            if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceBbmask => {
            if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::PreviewBbmaskEdit => {
            if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog() {
                let input = match popup_toml_value(&editor.text, "bbmask") {
                    Ok(value) => value,
                    Err(reason) => {
                        app.notification = Some(reason);
                        return None;
                    }
                };
                if input.contains(['\n', '\r']) {
                    app.notification = Some("BBMASK must be entered on one line.".into());
                } else {
                    replace_dialog(app, Dialog::BbmaskConfirmation(input));
                }
            }
        }
        Action::CancelBbmaskEdit => {
            if matches!(app.active_dialog(), Some(Dialog::BbmaskEdit(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmBbmaskWrite => {
            if let Some(Dialog::BbmaskConfirmation(value)) = app.active_dialog().cloned() {
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::WriteBbmask(value));
            }
        }
        Action::CancelBbmaskWrite => {
            if matches!(app.active_dialog(), Some(Dialog::BbmaskConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::BeginMetadataSearch => app.metadata_searching = true,
        Action::AppendMetadataQuery(character) if app.metadata_searching => {
            app.metadata_query.push(character);
            app.recipe_selection = 0;
            app.layer_selection = 0;
            app.config_selection = 0;
            select_first_matching_layer_entry(app);
            select_first_matching_recipe(app);
        }
        Action::BackspaceMetadataQuery if app.metadata_searching => {
            app.metadata_query.pop();
            app.recipe_selection = 0;
            app.layer_selection = 0;
            app.config_selection = 0;
            select_first_matching_layer_entry(app);
            select_first_matching_recipe(app);
        }
        Action::FinishMetadataSearch => app.metadata_searching = false,
        Action::AppendLogQuery(_)
        | Action::BackspaceLogQuery
        | Action::NextLogMatch
        | Action::PreviousLogMatch
        | Action::AppendCommandPaletteQuery(_)
        | Action::BackspaceCommandPaletteQuery
        | Action::AppendMetadataQuery(_)
        | Action::BackspaceMetadataQuery => {}
        Action::Notify(message) => app.notification = Some(message),
        Action::ActivateNotification => {
            if app.build.status == BuildStatus::Failed && app.logs.diagnostics().next().is_some() {
                app.screen = Screen::Errors;
                app.error_selection = app.logs.diagnostics().count().saturating_sub(1);
            }
            app.notification = None;
        }
        Action::DismissNotification => app.notification = None,
        Action::Quit => {
            if matches!(
                app.build.status,
                BuildStatus::Running | BuildStatus::Parsing | BuildStatus::Cancelling
            ) {
                open_dialog(app, Dialog::QuitConfirmation)
            } else {
                app.should_quit = true
            }
        }
        Action::ConfirmQuit => {
            if matches!(app.active_dialog(), Some(Dialog::QuitConfirmation)) {
                app.should_quit = true;
            }
        }
        Action::CancelQuit => {
            if matches!(app.active_dialog(), Some(Dialog::QuitConfirmation)) {
                close_dialog(app);
            }
        }
        Action::WorkspaceLoaded(w) => {
            let selected = selected_config_identity(app);
            app.workspace = w;
            app.available_images = if app.build_environment.connected() {
                app.workspace
                    .recipes
                    .iter()
                    .filter(|recipe| {
                        recipe.name.starts_with("core-image-") || recipe.name.ends_with("-image")
                    })
                    .map(|recipe| recipe.name.clone())
                    .collect()
            } else {
                Vec::new()
            };
            if app.config_scope.as_ref().is_some_and(|scope| {
                !app.workspace
                    .recipes
                    .iter()
                    .any(|recipe| &recipe.name == scope)
            }) {
                app.config_scope = None;
            }
            let names = app
                .workspace
                .variables
                .keys()
                .cloned()
                .collect::<HashSet<_>>();
            app.variable_details
                .retain(|identity, _| names.contains(&identity.name));
            app.variable_detail_loading
                .retain(|identity| names.contains(&identity.name));
            app.variable_detail_errors
                .retain(|identity, _| names.contains(&identity.name));
            let identities = filtered_config_identities(app);
            app.config_selection = selected
                .and_then(|selected| {
                    identities
                        .iter()
                        .position(|identity| identity.name == selected.name)
                })
                .unwrap_or_else(|| app.config_selection.min(identities.len().saturating_sub(1)));
        }
        Action::RecipesLoaded(mut recipes) => {
            let selected = app
                .workspace
                .recipes
                .get(app.recipe_selection)
                .map(|recipe| recipe.name.clone());
            recipes.sort_by(|left, right| left.name.cmp(&right.name));
            let names = recipes
                .iter()
                .map(|recipe| recipe.name.clone())
                .collect::<HashSet<_>>();
            app.workspace.recipes = recipes;
            app.available_images = if app.build_environment.connected() {
                app.workspace
                    .recipes
                    .iter()
                    .filter(|recipe| {
                        recipe.name.starts_with("core-image-") || recipe.name.ends_with("-image")
                    })
                    .map(|recipe| recipe.name.clone())
                    .collect()
            } else {
                Vec::new()
            };
            if app
                .config_scope
                .as_ref()
                .is_some_and(|scope| !names.contains(scope))
            {
                app.config_scope = None;
            }
            app.recipe_metadata
                .retain(|recipe, _| names.contains(recipe));
            app.recipe_sources
                .retain(|recipe, _| names.contains(recipe));
            app.recipe_metadata_loading
                .retain(|recipe| names.contains(recipe));
            app.recipe_metadata_errors
                .retain(|recipe, _| names.contains(recipe));
            app.recipe_selection = selected
                .and_then(|selected| {
                    app.workspace
                        .recipes
                        .iter()
                        .position(|recipe| recipe.name == selected)
                })
                .unwrap_or_else(|| {
                    app.recipe_selection
                        .min(app.workspace.recipes.len().saturating_sub(1))
                });
        }
        Action::LayersLoaded(mut layers) => {
            layers.sort_by(|left, right| left.name.cmp(&right.name));
            app.workspace.layers = layers;
            app.layer_selection = app
                .layer_selection
                .min(app.workspace.layers.len().saturating_sub(1));
        }
        Action::VariableLoaded(detail) => {
            app.variable_detail_loading.remove(&detail.identity);
            app.variable_detail_errors.remove(&detail.identity);
            if detail.identity.recipe.is_none() {
                if let Some(value) = detail.effective_value.clone() {
                    app.workspace
                        .variables
                        .insert(detail.identity.name.clone(), value);
                } else {
                    app.workspace.variables.remove(&detail.identity.name);
                }
                if let Some(provenance) = detail.provenance.clone() {
                    app.workspace
                        .variable_provenance
                        .insert(detail.identity.name.clone(), provenance);
                } else {
                    app.workspace
                        .variable_provenance
                        .remove(&detail.identity.name);
                }
                app.workspace.variable_provenance_chain.insert(
                    detail.identity.name.clone(),
                    detail
                        .operations
                        .iter()
                        .filter_map(|operation| {
                            operation.file.as_ref().map(|file| {
                                operation.line.map_or_else(
                                    || file.display().to_string(),
                                    |line| format!("{}:{line}", file.display()),
                                )
                            })
                        })
                        .collect(),
                );
            }
            app.variable_details.insert(detail.identity.clone(), detail);
        }
        Action::RecipeSourcesLoaded { recipe, paths } => {
            app.recipe_sources.insert(recipe, paths);
        }
        Action::HostTelemetryUpdated(telemetry) => {
            const HISTORY_LIMIT: usize = 60;
            if let Some(cpu) = telemetry.cpu_utilization_percent {
                app.host_cpu_history.push_back(cpu.min(100));
                while app.host_cpu_history.len() > HISTORY_LIMIT {
                    app.host_cpu_history.pop_front();
                }
            }
            if let (Some(total), Some(available)) = (
                telemetry.memory_total_bytes,
                telemetry.memory_available_bytes,
            ) && total > 0
                && available <= total
            {
                let used = total.saturating_sub(available);
                let percent = used.saturating_mul(100) / total;
                app.host_memory_history
                    .push_back(u8::try_from(percent.min(100)).unwrap_or(100));
                while app.host_memory_history.len() > HISTORY_LIMIT {
                    app.host_memory_history.pop_front();
                }
            }
            app.host_telemetry = telemetry;
        }
        Action::Failure(e) => {
            let message = e.to_string();
            insert_system_log(app, Severity::Error, message.clone());
            app.notification = Some(message);
            app.build.status = BuildStatus::Failed;
            app.build.errors = app.build.errors.max(1);
        }
        Action::Tick if !app.reduced_motion => {
            app.animation_frame = app.animation_frame.wrapping_add(1)
        }
        Action::Tick => {}
    }
    synchronize_focus(app);
    None
}

fn next_filter(values: &[String], current: Option<String>) -> Option<String> {
    let Some(current) = current else {
        return values.first().cloned();
    };
    values
        .iter()
        .position(|value| value == &current)
        .and_then(|index| values.get(index + 1))
        .cloned()
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    PersistSettings,
    GenerateProjectProfile {
        profile: ProjectProfile,
        replace: bool,
    },
    VerifyBuildEnvironment {
        profile: BuildEnvironmentProfile,
        generation: u64,
    },
    CloneBuildEnvironment(BuildEnvironmentClonePlan),
    Start(BuildRequest),
    Cancel,
    OpenInEditor(PathBuf),
    CopyToClipboard(String),
    OpenWorkspaceEditor {
        label: String,
        root: PathBuf,
    },
    LoadLayerBrowserDirectory {
        layer: String,
        root: PathBuf,
        directory: PathBuf,
    },
    LoadLayerBrowserPreview(PathBuf),
    OpenLayerBrowserEditor {
        layer: String,
        root: PathBuf,
        file: PathBuf,
    },
    DevtoolModify(RecipeIdentity),
    DevtoolReset(DevtoolResetPlan),
    DevtoolUpdateRecipe(RecipeIdentity),
    DevtoolFinish(DevtoolFinishPlan),
    DevtoolDeploy(DevtoolDeployPlan),
    InspectDevtoolStatus(RecipeIdentity),
    GetDependencies(String),
    GetSignatureDump(SignatureTarget),
    CompareSignatures(SignatureComparisonRequest),
    CancelSignatureOperation,
    GetPackageInventory(PackageInventoryRequest),
    GetPackageDetail(PackageDetailRequest),
    CancelPackageOperation,
    GetImageArtifacts(ImageArtifactRequest),
    CancelImageArtifactOperation,
    GetSdkArtifacts(SdkArtifactInventoryRequest),
    CancelSdkArtifactOperation,
    InspectSdkTools,
    StartSdkSession {
        id: SdkSessionId,
        operation: SdkOperation,
    },
    CancelSdkSession(SdkSessionId),
    InspectTestCapability,
    StartTestSession {
        id: TestSessionId,
        operation: TestOperation,
    },
    StartTestBuildSession {
        id: TestSessionId,
        request: BuildRequest,
    },
    CancelTestSession(TestSessionId),
    InspectResultToolCapability,
    ImportTestResults(TestResultImportRequest),
    CompareTestResults(TestComparisonRequest),
    InspectTestJunitDestination {
        result: TestResultIdentity,
        destination: PathBuf,
    },
    ExportTestJunit(TestJunitExportRequest),
    Security(SecurityEffect),
    Qa(QaEffect),
    Maintenance(MaintenanceEffect),
    InspectQemuCapability,
    StartQemuSession {
        id: QemuSessionId,
        request: QemuLaunchRequest,
    },
    CancelQemuSession(QemuSessionId),
    InspectWicCapability,
    GetWicOutputs(WicOutputInventoryRequest),
    GetWicDevices(WicDeviceInventoryRequest),
    StartWicSession {
        id: WicSessionId,
        operation: WicOperation,
    },
    CancelWicSession(WicSessionId),
    GetRecipeMetadata(String),
    GetVariable(VariableIdentity),
    WriteConfigAssignment(ConfigEditRequest),
    GetLayerRelationships,
    LoadRecipeEditorFile(PathBuf),
    SaveRecipeEditorFile {
        path: PathBuf,
        content: String,
    },
    WriteBbmask(String),
}
pub fn format_duration(duration: Duration) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        duration.as_secs() / 3600,
        duration.as_secs() / 60 % 60,
        duration.as_secs() % 60
    )
}
impl fmt::Display for BuildStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn log(message: &str) -> LogEntry {
        LogEntry {
            id: 0,
            severity: Severity::Info,
            message: message.into(),
            recipe: None,
            task: None,
            path: None,
            timestamp: SystemTime::now(),
            build: None,
            protected: false,
            diagnostic: None,
        }
    }
    fn tagged_log(recipe: &str, task: &str, severity: Severity, message: &str) -> LogEntry {
        LogEntry {
            id: 0,
            severity,
            message: message.into(),
            recipe: Some(recipe.into()),
            task: Some(task.into()),
            path: None,
            timestamp: SystemTime::now(),
            build: None,
            protected: false,
            diagnostic: None,
        }
    }
    fn background_job_spec(id: u64, cancellation_supported: bool) -> BackgroundJobSpec {
        BackgroundJobSpec {
            id: BackgroundJobId(id),
            kind: BackgroundJobKind::Build,
            title: format!("Build job {id}"),
            context: BackgroundJobContext {
                workspace: Some(Screen::Tasks),
                target: Some("core-image-minimal".into()),
                ..BackgroundJobContext::default()
            },
            cancellation_supported,
            queued_at: SystemTime::UNIX_EPOCH,
        }
    }
    fn run_background_job(app: &mut App, id: u64) {
        let id = BackgroundJobId(id);
        let _ = update(
            app,
            Action::StartBackgroundJob {
                id,
                started_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            },
        );
        let _ = update(app, Action::RunBackgroundJob { id });
    }
    #[test]
    fn background_job_completes_and_survives_workspace_navigation() {
        let mut app = App::new(10, 1_000);
        let id = BackgroundJobId(1);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(1, true)),
        );
        let _ = update(&mut app, Action::Open(Screen::Layers));
        run_background_job(&mut app, 1);
        let _ = update(
            &mut app,
            Action::UpdateBackgroundJobProgress {
                id,
                progress: BackgroundJobProgress::Units {
                    completed: 4,
                    total: 10,
                },
            },
        );
        let _ = update(
            &mut app,
            Action::AppendBackgroundJobOutput {
                id,
                entry: BackgroundJobOutputEntry {
                    severity: Severity::Warning,
                    message: "cache miss".into(),
                    source: BackgroundJobOutputSource::Backend,
                    truncated: false,
                    timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
                },
            },
        );
        let _ = update(
            &mut app,
            Action::SucceedBackgroundJob {
                id,
                result: BackgroundJobResult {
                    summary: "image built".into(),
                    artifacts: vec!["/deploy/core-image-minimal.wic".into()],
                },
                finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
            },
        );
        let _ = update(&mut app, Action::Open(Screen::Settings));

        let job = app.background_jobs.get(id).unwrap();
        assert_eq!(app.screen, Screen::Settings);
        assert_eq!(app.focus, FocusTarget::Workspace);
        assert_eq!(job.status, BackgroundJobStatus::Succeeded);
        assert_eq!(
            job.progress,
            BackgroundJobProgress::Units {
                completed: 4,
                total: 10
            }
        );
        assert_eq!(job.warnings, 1);
        assert_eq!(
            job.started_at,
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1))
        );
        assert_eq!(
            job.finished_at,
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(3))
        );
        assert_eq!(
            job.result.as_ref().map(|result| result.summary.as_str()),
            Some("image built")
        );
    }
    #[test]
    fn background_job_records_failure_and_loss() {
        let mut app = App::new(10, 1_000);
        for id in [1, 2] {
            let _ = update(
                &mut app,
                Action::QueueBackgroundJob(background_job_spec(id, true)),
            );
            run_background_job(&mut app, id);
        }
        let _ = update(
            &mut app,
            Action::FailBackgroundJob {
                id: BackgroundJobId(1),
                error: BackgroundJobError {
                    summary: "BitBake failed".into(),
                    detail: Some("exit code 1".into()),
                },
                finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(4),
            },
        );
        let _ = update(
            &mut app,
            Action::LoseBackgroundJob {
                id: BackgroundJobId(2),
                error: BackgroundJobError {
                    summary: "bridge disconnected".into(),
                    detail: None,
                },
                finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
            },
        );

        assert_eq!(
            app.background_jobs.get(BackgroundJobId(1)).unwrap().status,
            BackgroundJobStatus::Failed
        );
        let lost = app.background_jobs.get(BackgroundJobId(2)).unwrap();
        assert_eq!(lost.status, BackgroundJobStatus::Lost);
        assert_eq!(
            lost.error.as_ref().map(|error| error.summary.as_str()),
            Some("bridge disconnected")
        );
    }
    #[test]
    fn background_job_cancellation_requires_capability_and_acknowledgement() {
        let mut app = App::new(10, 1_000);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(1, true)),
        );
        let _ = update(
            &mut app,
            Action::RequestBackgroundJobCancellation {
                id: BackgroundJobId(1),
            },
        );
        assert_eq!(
            app.background_jobs.get(BackgroundJobId(1)).unwrap().status,
            BackgroundJobStatus::Cancelling
        );
        let _ = update(
            &mut app,
            Action::CancelBackgroundJob {
                id: BackgroundJobId(1),
                finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            },
        );
        assert_eq!(
            app.background_jobs.get(BackgroundJobId(1)).unwrap().status,
            BackgroundJobStatus::Cancelled
        );

        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(2, false)),
        );
        run_background_job(&mut app, 2);
        let ignored_before = app.background_jobs.ignored_transitions;
        let _ = update(
            &mut app,
            Action::RequestBackgroundJobCancellation {
                id: BackgroundJobId(2),
            },
        );
        assert_eq!(
            app.background_jobs.get(BackgroundJobId(2)).unwrap().status,
            BackgroundJobStatus::Running
        );
        assert_eq!(app.background_jobs.ignored_transitions, ignored_before + 1);
    }
    #[test]
    fn background_job_rejected_cancellation_returns_to_running() {
        let mut app = App::new(10, 1_000);
        let id = BackgroundJobId(1);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(1, true)),
        );
        run_background_job(&mut app, 1);
        let _ = update(&mut app, Action::RequestBackgroundJobCancellation { id });
        let _ = update(&mut app, Action::RejectBackgroundJobCancellation { id });
        assert_eq!(
            app.background_jobs.get(id).unwrap().status,
            BackgroundJobStatus::Running
        );
    }
    #[test]
    fn background_job_invalid_transitions_leave_state_unchanged() {
        let mut app = App::new(10, 1_000);
        let id = BackgroundJobId(1);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(1, true)),
        );
        let _ = update(&mut app, Action::RunBackgroundJob { id });
        let _ = update(
            &mut app,
            Action::UpdateBackgroundJobProgress {
                id,
                progress: BackgroundJobProgress::Percent(101),
            },
        );
        assert_eq!(
            app.background_jobs.get(id).unwrap().status,
            BackgroundJobStatus::Queued
        );
        assert_eq!(app.background_jobs.ignored_transitions, 2);

        run_background_job(&mut app, 1);
        let _ = update(
            &mut app,
            Action::SucceedBackgroundJob {
                id,
                result: BackgroundJobResult {
                    summary: "done".into(),
                    artifacts: vec![],
                },
                finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            },
        );
        let _ = update(
            &mut app,
            Action::FailBackgroundJob {
                id,
                error: BackgroundJobError {
                    summary: "late failure".into(),
                    detail: None,
                },
                finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
            },
        );
        assert_eq!(
            app.background_jobs.get(id).unwrap().status,
            BackgroundJobStatus::Succeeded
        );
        assert_eq!(app.background_jobs.ignored_transitions, 3);
    }
    #[test]
    fn background_job_history_and_output_retention_are_bounded_and_observable() {
        let mut app = App::new(10, 1_000);
        app.background_jobs = BackgroundJobs::new(2, 2, 4);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(1, true)),
        );
        run_background_job(&mut app, 1);
        let _ = update(
            &mut app,
            Action::SucceedBackgroundJob {
                id: BackgroundJobId(1),
                result: BackgroundJobResult {
                    summary: "done".into(),
                    artifacts: vec![],
                },
                finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            },
        );
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(2, true)),
        );
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(3, true)),
        );
        assert_eq!(app.background_jobs.jobs.len(), 2);
        assert_eq!(app.background_jobs.dropped_jobs, 1);
        assert!(app.background_jobs.get(BackgroundJobId(1)).is_none());

        let _ = update(
            &mut app,
            Action::AppendBackgroundJobOutput {
                id: BackgroundJobId(2),
                entry: BackgroundJobOutputEntry {
                    severity: Severity::Warning,
                    message: "abc".into(),
                    source: BackgroundJobOutputSource::Backend,
                    truncated: false,
                    timestamp: SystemTime::UNIX_EPOCH,
                },
            },
        );
        let _ = update(
            &mut app,
            Action::AppendBackgroundJobOutput {
                id: BackgroundJobId(2),
                entry: BackgroundJobOutputEntry {
                    severity: Severity::Error,
                    message: "de".into(),
                    source: BackgroundJobOutputSource::Backend,
                    truncated: false,
                    timestamp: SystemTime::UNIX_EPOCH,
                },
            },
        );
        let retained = app.background_jobs.get(BackgroundJobId(2)).unwrap();
        assert_eq!(retained.output.len(), 1);
        assert_eq!(retained.retained_output_bytes, 2);
        assert_eq!(retained.dropped_output_entries, 1);
        assert_eq!(retained.warnings, 1);
        assert_eq!(retained.errors, 1);

        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(background_job_spec(4, true)),
        );
        assert_eq!(app.background_jobs.jobs.len(), 2);
        assert_eq!(app.background_jobs.rejected_jobs, 1);
    }
    #[test]
    fn bounded_logs_report_eviction() {
        let mut l = LogState::new(2, 100);
        l.insert(log("a"));
        l.insert(log("b"));
        l.insert(log("c"));
        assert_eq!(l.entries.len(), 2);
        assert_eq!(l.dropped, 1)
    }
    #[test]
    fn navigator_selection_and_focus_cycle_are_bounded() {
        let mut app = App::new(10, 1_000);
        let _ = update(&mut app, Action::Focus(FocusTarget::Navigator));
        let _ = update(&mut app, Action::SelectNavigator { delta: 100 });
        assert_eq!(app.navigator_selection, NAVIGATOR_SCREENS.len() - 1);
        let _ = update(&mut app, Action::ActivateNavigator);
        assert_eq!(app.screen, Screen::Settings);
        assert_eq!(app.focus, FocusTarget::Workspace);
        let _ = update(&mut app, Action::CycleFocus { backwards: false });
        assert_eq!(app.focus, FocusTarget::Inspector);
        let _ = update(&mut app, Action::CycleFocus { backwards: true });
        assert_eq!(app.focus, FocusTarget::Workspace);
    }

    #[test]
    fn navigator_screen_projects_the_bounded_selection() {
        let mut app = App::new(10, 1_000);
        app.navigator_selection = 1;
        assert_eq!(app.navigator_screen(), Screen::Layers);
        app.navigator_selection = usize::MAX;
        assert_eq!(app.navigator_screen(), Screen::Dashboard);
    }

    #[test]
    fn navigator_workbench_order_keeps_build_and_validation_groups_contiguous() {
        assert_eq!(
            NAVIGATOR_SCREENS,
            [
                Screen::Dashboard,
                Screen::Layers,
                Screen::Recipes,
                Screen::Packages,
                Screen::Images,
                Screen::Sdk,
                Screen::Tasks,
                Screen::Logs,
                Screen::Errors,
                Screen::Configuration,
                Screen::Dependencies,
                Screen::Testing,
                Screen::Security,
                Screen::Qa,
                Screen::Recipes,
                Screen::Maintenance,
                Screen::BuildEnvironment,
                Screen::Settings,
            ]
        );
    }
    #[test]
    fn responsive_pane_focus_cycle_cannot_escape_modal_focus() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Dialog;
        let _ = update(&mut app, Action::CycleFocus { backwards: false });
        assert_eq!(app.focus, FocusTarget::Dialog);

        app.focus = FocusTarget::CommandPalette;
        let _ = update(&mut app, Action::CycleFocus { backwards: true });
        assert_eq!(app.focus, FocusTarget::CommandPalette);

        app.focus = FocusTarget::Workspace;
        let _ = update(&mut app, Action::CycleFocus { backwards: false });
        assert_eq!(app.focus, FocusTarget::Inspector);
        let _ = update(&mut app, Action::CycleFocus { backwards: false });
        assert_eq!(app.focus, FocusTarget::Navigator);
    }
    #[test]
    fn focus_restores_exact_pane_after_nested_dialog_transitions() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Inspector;

        let _ = update(&mut app, Action::OpenBuildOptions);
        assert_eq!(app.focus, FocusTarget::Dialog);
        assert_eq!(app.focus_return, Some(FocusTarget::Inspector));

        let _ = update(&mut app, Action::BeginBuildTargetEdit);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::BuildTarget { .. })
        ));
        assert_eq!(app.focus, FocusTarget::Dialog);
        assert_eq!(app.focus_return, Some(FocusTarget::Inspector));

        let _ = update(&mut app, Action::CancelBuildTargetEdit);
        assert_eq!(app.focus, FocusTarget::Inspector);
        assert_eq!(app.focus_return, None);
    }
    #[test]
    fn focus_command_palette_restores_or_transitions_without_leaking_input() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Navigator;
        app.workspace.build_dir = Some(PathBuf::from("/build"));

        let _ = update(&mut app, Action::OpenCommandPalette);
        assert_eq!(app.focus, FocusTarget::CommandPalette);
        assert_eq!(app.focus_return, Some(FocusTarget::Navigator));

        let original_screen = app.screen;
        let original_selection = app.navigator_selection;
        let _ = update(&mut app, Action::Open(Screen::Logs));
        let _ = update(&mut app, Action::SelectNavigator { delta: 1 });
        let _ = update(&mut app, Action::Focus(FocusTarget::Workspace));
        assert_eq!(app.screen, original_screen);
        assert_eq!(app.navigator_selection, original_selection);
        assert_eq!(app.focus, FocusTarget::CommandPalette);

        let _ = update(&mut app, Action::ActivateCommandPalette);
        assert!(matches!(app.active_dialog(), Some(Dialog::BuildOptions)));
        assert_eq!(app.focus, FocusTarget::Dialog);
        assert_eq!(app.focus_return, Some(FocusTarget::Navigator));
        let _ = update(&mut app, Action::CloseBuildOptions);
        assert_eq!(app.focus, FocusTarget::Navigator);
    }
    #[test]
    fn command_palette_search_is_case_insensitive_and_selection_is_bounded() {
        let mut app = App::new(10, 1_000);
        let _ = update(&mut app, Action::OpenCommandPalette);
        for character in "PROVENANCE".chars() {
            let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
        }
        let commands = app.filtered_command_palette_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].id, CommandId::OpenConfiguration);

        let _ = update(&mut app, Action::SelectCommandPalette { delta: 99 });
        assert_eq!(app.command_palette_selection, 0);
        for _ in 0.."PROVENANCE".len() {
            let _ = update(&mut app, Action::BackspaceCommandPaletteQuery);
        }
        assert!(app.command_palette_query.is_empty());
        assert!(app.filtered_command_palette_commands().len() > 6);
    }
    #[test]
    fn command_palette_empty_and_disabled_activation_are_inert() {
        let mut app = App::new(10, 1_000);
        let _ = update(&mut app, Action::OpenCommandPalette);
        let original = app.clone();
        assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
        assert_eq!(app, original, "disabled Build image must remain open");

        for character in "no such command".chars() {
            let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
        }
        let no_results = app.clone();
        assert!(app.filtered_command_palette_commands().is_empty());
        assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
        assert_eq!(app, no_results);
    }
    #[test]
    fn command_palette_available_entry_dispatches_existing_typed_action() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Inspector;
        let _ = update(&mut app, Action::OpenCommandPalette);
        for character in "Open Settings".chars() {
            let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
        }

        assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
        assert_eq!(app.screen, Screen::Settings);
        assert!(!app.command_palette_open);
        assert_eq!(app.focus, FocusTarget::Workspace);
    }
    #[test]
    fn theme_command_palette_entry_opens_named_picker() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Inspector;
        let _ = update(&mut app, Action::OpenCommandPalette);
        for character in "Choose theme".chars() {
            let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
        }

        assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::ThemePicker { .. })
        ));
        assert_eq!(app.focus, FocusTarget::Dialog);
        assert_eq!(app.focus_return, Some(FocusTarget::Inspector));
    }
    #[test]
    fn focus_async_dialog_waits_behind_palette_then_restores() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Inspector;
        let _ = update(&mut app, Action::OpenCommandPalette);
        let _ = update(
            &mut app,
            Action::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
        );
        assert!(matches!(app.active_dialog(), Some(Dialog::BuildCompletion)));
        assert_eq!(app.focus, FocusTarget::CommandPalette);

        let _ = update(&mut app, Action::CloseCommandPalette);
        assert_eq!(app.focus, FocusTarget::Dialog);
        let _ = update(&mut app, Action::DismissBuildCompletion);
        assert_eq!(app.focus, FocusTarget::Inspector);
    }
    #[test]
    fn dialog_completion_queues_behind_active_dialog_and_restores_focus_after_both_close() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Navigator;
        let _ = update(&mut app, Action::OpenBuildOptions);
        let _ = update(
            &mut app,
            Action::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
        );

        assert_eq!(
            app.dialogs.iter().collect::<Vec<_>>(),
            vec![&Dialog::BuildOptions, &Dialog::BuildCompletion]
        );
        assert_eq!(app.focus, FocusTarget::Dialog);
        let _ = update(&mut app, Action::DismissBuildCompletion);
        assert_eq!(app.dialogs.len(), 2, "only the active dialog may dismiss");

        let _ = update(&mut app, Action::CloseBuildOptions);
        assert!(matches!(app.active_dialog(), Some(Dialog::BuildCompletion)));
        assert_eq!(app.focus, FocusTarget::Dialog);
        let _ = update(&mut app, Action::DismissBuildCompletion);
        assert!(app.dialogs.is_empty());
        assert_eq!(app.focus, FocusTarget::Navigator);
    }
    #[test]
    fn dialog_invalid_actions_leave_active_state_unchanged() {
        let mut app = App::new(10, 1_000);
        let _ = update(&mut app, Action::OpenBuildOptions);
        let original = app.clone();

        assert_eq!(update(&mut app, Action::ConfirmDevtoolReset), None);
        let _ = update(&mut app, Action::AppendBbmask('x'));
        let _ = update(&mut app, Action::CancelImagePicker);

        assert_eq!(app, original);
    }
    #[test]
    fn focus_quit_confirmation_traps_and_restores() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Navigator;
        app.build.status = BuildStatus::Running;
        let _ = update(&mut app, Action::Quit);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::QuitConfirmation)
        ));
        assert_eq!(app.focus, FocusTarget::Dialog);

        let _ = update(&mut app, Action::Open(Screen::Logs));
        assert_eq!(app.screen, Screen::Dashboard);
        let _ = update(&mut app, Action::CancelQuit);
        assert!(app.active_dialog().is_none());
        assert_eq!(app.focus, FocusTarget::Navigator);
    }
    #[test]
    fn parse_progress_tracks_current_and_total() {
        let mut app = App::new(10, 1_000);
        let _ = update(
            &mut app,
            Action::ParseProgress {
                current: Some(8),
                total: Some(20),
            },
        );
        assert_eq!(app.build.status, BuildStatus::Parsing);
        assert_eq!(app.build.parse_current, Some(8));
        assert_eq!(app.build.parse_total, Some(20));
        let _ = update(&mut app, Action::BuildStarted);
        assert_eq!(app.build.parse_current, None);
        assert_eq!(app.build.parse_total, None);
    }
    #[test]
    fn eviction_counts_dropped_warnings_and_errors() {
        let mut logs = LogState::new(1, 100);
        logs.insert(tagged_log(
            "busybox",
            "do_compile",
            Severity::Warning,
            "warning",
        ));
        logs.insert(tagged_log(
            "busybox",
            "do_compile",
            Severity::Error,
            "error",
        ));
        logs.insert(log("latest"));
        assert_eq!(logs.dropped, 2);
        assert_eq!(logs.dropped_warnings, 1);
        assert_eq!(logs.dropped_errors, 0);
        assert_eq!(
            logs.entries.front().map(|entry| entry.severity),
            Some(Severity::Error)
        );
    }
    #[test]
    fn high_volume_logs_remain_within_retention_limits() {
        let mut logs = LogState::new(128, 4_096);
        for index in 0..20_000 {
            logs.insert(log(&format!("line {index}: {}", "x".repeat(index % 80))));
        }
        assert!(logs.entries.len() <= 128);
        assert!(logs.retained_bytes <= 4_096);
        assert_eq!(
            logs.retained_bytes,
            logs.entries.iter().map(|entry| entry.message.len()).sum()
        );
        assert!(logs.dropped > 0);
    }
    #[test]
    fn reducer_covers_build_lifecycle_and_log_controls() {
        let mut app = App::new(10, 1_000);
        assert!(
            update(
                &mut app,
                Action::Start(BuildRequest {
                    targets: vec!["bad target".into()],
                    task: None,
                    force: false,
                }),
            )
            .is_none()
        );
        assert!(app.notification.is_some());
        let request = BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("compile".into()),
            force: false,
        };
        assert_eq!(
            update(&mut app, Action::Start(request.clone())),
            Some(Effect::Start(request))
        );
        let _ = update(&mut app, Action::BuildStarted);
        let id = TaskId("busybox:do_compile".into());
        let _ = update(
            &mut app,
            Action::TaskStarted(TaskInfo {
                id: id.clone(),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: None,
                ..TaskInfo::default()
            }),
        );
        let _ = update(
            &mut app,
            Action::TaskProgress {
                id: id.clone(),
                progress: Some(50),
            },
        );
        let _ = update(&mut app, Action::TaskCompleted { id, success: true });
        assert_eq!(update(&mut app, Action::Cancel), Some(Effect::Cancel));
        let _ = update(
            &mut app,
            Action::BuildCompleted {
                success: false,
                exit_code: Some(1),
            },
        );
        assert_eq!(app.build.status, BuildStatus::Failed);
        assert_eq!(app.build.exit_code, Some(1));
        let _ = update(&mut app, Action::Open(Screen::Logs));
        let _ = update(&mut app, Action::BeginLogSearch);
        let _ = update(&mut app, Action::AppendLogQuery('x'));
        let _ = update(&mut app, Action::BackspaceLogQuery);
        let _ = update(&mut app, Action::FinishLogSearch);
        let _ = update(&mut app, Action::ScrollLogsHorizontally { delta: 5 });
        let _ = update(&mut app, Action::ScrollLogsHorizontally { delta: -5 });
        let _ = update(
            &mut app,
            Action::Failure(AppError::new("test", "failure", "retry")),
        );
        let _ = update(&mut app, Action::DismissNotification);
        assert!(app.notification.is_none());
    }
    #[test]
    fn beginning_a_build_clears_stale_build_state() {
        let mut app = App::new(10, 1_000);
        app.build.completed = 7;
        app.build.total = Some(10);
        app.build.parse_current = Some(3);
        app.build.parse_total = Some(4);
        app.build.warnings = 2;
        app.build.errors = 1;
        app.build.exit_code = Some(1);
        app.build.started = Some(SystemTime::now());
        app.tasks.insert(
            TaskId("old:task".into()),
            TaskInfo {
                id: TaskId("old:task".into()),
                recipe: "old".into(),
                task: "task".into(),
                progress: Some(50),
                ..TaskInfo::default()
            },
        );
        let request = BuildRequest {
            targets: vec!["busybox".into()],
            task: None,
            force: false,
        };
        assert_eq!(
            update(&mut app, Action::Start(request.clone())),
            Some(Effect::Start(request))
        );
        assert_eq!(app.build.status, BuildStatus::LoadingWorkspace);
        assert_eq!(app.build.target.as_deref(), Some("busybox"));
        assert_eq!(app.build.completed, 0);
        assert_eq!(app.build.total, None);
        assert_eq!(app.build.parse_current, None);
        assert_eq!(app.build.parse_total, None);
        assert_eq!(app.build.warnings, 0);
        assert_eq!(app.build.errors, 0);
        assert_eq!(app.build.exit_code, None);
        assert_eq!(app.build.started, None);
        assert!(app.tasks.is_empty());
    }
    #[test]
    fn completed_builds_are_retained_in_session_history() {
        let mut app = App::new(10, 1_000);
        app.build.target = Some("core-image-minimal".into());
        app.build.completed = 12;
        app.build.warnings = 2;
        app.build.errors = 1;
        app.build.started = Some(SystemTime::now());
        let _ = update(
            &mut app,
            Action::BuildCompleted {
                success: false,
                exit_code: Some(1),
            },
        );
        assert_eq!(app.build_history.len(), 1);
        assert_eq!(
            app.build_history[0].target.as_deref(),
            Some("core-image-minimal")
        );
        assert!(!app.build_history[0].success);
        assert_eq!(app.build_history[0].completed_tasks, 12);
        assert_eq!(app.build_history[0].errors, 1);
    }
    #[test]
    fn selected_error_jumps_to_exact_log_without_replacing_user_filters() {
        let mut app = App::new(10, 1_000);
        let _ = update(
            &mut app,
            Action::Log(tagged_log(
                "busybox",
                "do_compile",
                Severity::Error,
                "compile failed",
            )),
        );
        app.logs.query = "user query".into();
        app.logs.filter = Some(Severity::Warning);
        let _ = update(&mut app, Action::Open(Screen::Errors));
        let _ = update(&mut app, Action::JumpToSelectedError);
        assert_eq!(app.screen, Screen::Logs);
        assert_eq!(app.logs.query, "user query");
        assert_eq!(app.logs.filter, Some(Severity::Warning));
        assert_eq!(
            app.logs.selected().map(|entry| entry.message.as_str()),
            Some("compile failed")
        );
    }
    #[test]
    fn selected_error_opens_its_source_path() {
        let mut app = App::new(10, 1_000);
        let mut entry = tagged_log("busybox", "do_compile", Severity::Error, "compile failed");
        entry.path = Some(PathBuf::from("/tmp/log.do_compile"));
        let _ = update(&mut app, Action::Log(entry));

        assert_eq!(
            update(&mut app, Action::OpenSelectedErrorSource),
            Some(Effect::OpenInEditor(PathBuf::from("/tmp/log.do_compile")))
        );
    }
    #[test]
    fn error_entries_gain_typed_category_summary_metadata_and_suggestions() {
        let mut app = App::new(10, 1_000);
        app.build.target = Some("core-image-minimal".into());
        let mut entry = tagged_log(
            "busybox",
            "do_compile",
            Severity::Error,
            "compile failed\nfull compiler context",
        );
        entry.path = Some(PathBuf::from("/tmp/log.do_compile"));
        let _ = update(&mut app, Action::Log(entry));
        let retained = app.logs.diagnostics().next().unwrap();
        let diagnostic = retained.diagnostic.as_ref().unwrap();
        assert_eq!(diagnostic.category, "BitBake error");
        assert_eq!(diagnostic.summary, "compile failed");
        assert!(
            diagnostic
                .event_metadata
                .iter()
                .any(|(name, value)| name == "build" && value == "core-image-minimal")
        );
        assert!(diagnostic.suggestions.len() >= 2);
        assert_eq!(retained.build.as_deref(), Some("core-image-minimal"));
    }
    #[test]
    fn error_completion_outcomes_are_distinct_and_actionable() {
        let mut success = App::new(10, 1_000);
        let _ = update(
            &mut success,
            Action::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
        );
        assert!(
            success
                .notification
                .as_deref()
                .is_some_and(|message| message.contains("successfully"))
        );

        let mut warning = App::new(10, 1_000);
        let _ = update(
            &mut warning,
            Action::Log(tagged_log(
                "busybox",
                "do_compile",
                Severity::Warning,
                "deprecated option",
            )),
        );
        let _ = update(
            &mut warning,
            Action::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
        );
        assert!(
            warning
                .notification
                .as_deref()
                .is_some_and(|message| message.contains("warning"))
        );

        let mut failed = App::new(10, 1_000);
        let _ = update(
            &mut failed,
            Action::Log(tagged_log(
                "busybox",
                "do_compile",
                Severity::Error,
                "compile failed",
            )),
        );
        let _ = update(
            &mut failed,
            Action::BuildCompleted {
                success: false,
                exit_code: Some(1),
            },
        );
        assert!(
            failed
                .notification
                .as_deref()
                .is_some_and(|message| message.contains("Press Enter"))
        );
        let _ = update(&mut failed, Action::OpenBuildCompletionErrors);
        assert_eq!(failed.screen, Screen::Errors);
        assert!(failed.active_dialog().is_none());

        let mut cancelled = App::new(10, 1_000);
        let _ = update(
            &mut cancelled,
            Action::BuildCancelled {
                exit_code: Some(130),
            },
        );
        assert!(
            cancelled
                .notification
                .as_deref()
                .is_some_and(|message| message.contains("distinct"))
        );
    }
    #[test]
    fn recipe_selection_stays_in_workspace_bounds() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![
            Recipe {
                name: "alpha".into(),
                version: None,
                layer: None,
                ..Recipe::default()
            },
            Recipe {
                name: "beta".into(),
                version: None,
                layer: None,
                ..Recipe::default()
            },
        ];
        let _ = update(&mut app, Action::SelectRecipe { delta: 8 });
        assert_eq!(app.recipe_selection, 1);
        let _ = update(&mut app, Action::SelectRecipe { delta: -8 });
        assert_eq!(app.recipe_selection, 0);
    }
    #[test]
    fn recipe_metadata_refresh_is_typed_and_replaces_stale_detail() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes.push(Recipe {
            name: "busybox".into(),
            version: Some("1.36".into()),
            layer: Some("core".into()),
            preferred_version: None,
            file: Some("/layers/meta/recipes-core/busybox/busybox.bb".into()),
            append_count: Some(2),
        });
        assert_eq!(
            update(&mut app, Action::BeginSelectedRecipeMetadata),
            Some(Effect::GetRecipeMetadata("busybox".into()))
        );
        let _ = update(
            &mut app,
            Action::RecipeMetadataLoaded(RecipeMetadata {
                recipe: "busybox".into(),
                workspace_status: None,
                build_status: None,
                tasks: Some(vec!["do_build".into()]),
                sources: Some(vec!["/layers/meta/busybox.bb".into()]),
                patches: Some(vec![]),
                packages: Some(vec!["busybox".into()]),
                history: None,
            }),
        );
        let _ = update(
            &mut app,
            Action::RecipeMetadataLoaded(RecipeMetadata {
                recipe: "busybox".into(),
                tasks: Some(vec!["do_compile".into()]),
                sources: None,
                ..RecipeMetadata::default()
            }),
        );
        let metadata = &app.recipe_metadata["busybox"];
        assert_eq!(metadata.tasks, Some(vec!["do_compile".into()]));
        assert_eq!(metadata.sources, None);
        assert!(!app.recipe_sources.contains_key("busybox"));
        assert_eq!(metadata.workspace_status, None);
        assert_eq!(metadata.history, None);
    }
    #[test]
    fn recipes_workspace_filter_selection_refresh_and_failure_are_identity_stable() {
        let mut app = App::new(10, 1_000);
        let _ = update(
            &mut app,
            Action::RecipesLoaded(vec![
                Recipe {
                    name: "alpha".into(),
                    version: Some("1".into()),
                    layer: Some("core".into()),
                    ..Recipe::default()
                },
                Recipe {
                    name: "busybox".into(),
                    version: Some("1.36".into()),
                    layer: Some("base".into()),
                    file: Some("/layers/base/recipes-core/busybox.bb".into()),
                    ..Recipe::default()
                },
                Recipe {
                    name: "zlib".into(),
                    version: Some("1.3".into()),
                    layer: Some("core".into()),
                    ..Recipe::default()
                },
            ]),
        );
        let _ = update(&mut app, Action::BeginMetadataSearch);
        for character in "base".chars() {
            let _ = update(&mut app, Action::AppendMetadataQuery(character));
        }
        assert_eq!(app.recipe_selection, 1);
        let _ = update(&mut app, Action::SelectRecipe { delta: isize::MAX });
        assert_eq!(app.recipe_selection, 1);
        assert_eq!(
            update(&mut app, Action::BeginSelectedRecipeMetadata),
            Some(Effect::GetRecipeMetadata("busybox".into()))
        );
        assert!(app.recipe_metadata_loading.contains("busybox"));
        let _ = update(
            &mut app,
            Action::RecipeMetadataFailed {
                recipe: "busybox".into(),
                message: "server unavailable".into(),
            },
        );
        assert!(!app.recipe_metadata_loading.contains("busybox"));
        assert_eq!(
            app.recipe_metadata_errors
                .get("busybox")
                .map(String::as_str),
            Some("server unavailable")
        );
        app.recipe_metadata_errors
            .insert("alpha".into(), "stale".into());

        let _ = update(
            &mut app,
            Action::RecipesLoaded(vec![
                Recipe {
                    name: "busybox".into(),
                    version: Some("1.37".into()),
                    layer: Some("base".into()),
                    ..Recipe::default()
                },
                Recipe {
                    name: "new".into(),
                    ..Recipe::default()
                },
            ]),
        );
        assert_eq!(app.workspace.recipes[app.recipe_selection].name, "busybox");
        assert_eq!(
            app.recipe_metadata_errors
                .get("busybox")
                .map(String::as_str),
            Some("server unavailable")
        );
        assert!(!app.recipe_metadata_errors.contains_key("alpha"));
    }
    #[test]
    fn recipe_bitbake_action_uses_authoritative_tasks_picker_and_confirmation() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes.push(Recipe {
            name: "busybox".into(),
            ..Recipe::default()
        });
        app.recipe_metadata.insert(
            "busybox".into(),
            RecipeMetadata {
                recipe: "busybox".into(),
                tasks: Some(
                    [
                        "do_clean",
                        "do_cleansstate",
                        "do_devshell",
                        "do_diffconfig",
                        "do_diffsigs",
                        "do_menuconfig",
                    ]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                ),
                ..RecipeMetadata::default()
            },
        );

        let actions = [
            (Action::BeginSelectedRecipeClean, "clean"),
            (Action::BeginSelectedRecipeCleanState, "cleansstate"),
            (Action::BeginSelectedRecipeMenuConfig, "menuconfig"),
            (Action::BeginSelectedRecipeDevshell, "devshell"),
            (Action::BeginSelectedRecipeDiffconfig, "diffconfig"),
            (Action::BeginSelectedRecipeDiffsigs, "diffsigs"),
        ];
        for (action, expected) in actions {
            app.dialogs.clear();
            let _ = update(&mut app, action);
            assert!(matches!(
                app.active_dialog(),
                Some(Dialog::RecipeTaskConfirmation(BuildRequest {
                    targets,
                    task: Some(task),
                    force: false,
                })) if targets == &vec!["busybox".to_owned()] && task == expected
            ));
        }

        app.dialogs.clear();
        let _ = update(&mut app, Action::BeginSelectedRecipeForceTask);
        let Some(Dialog::RecipeTaskPicker(picker)) = app.active_dialog() else {
            panic!("authoritative task picker did not open");
        };
        assert!(picker.force);
        assert_eq!(picker.tasks[0], "clean");
        let _ = update(&mut app, Action::SelectRecipeTask { delta: 3 });
        let _ = update(&mut app, Action::PreviewSelectedRecipeTask);
        let Some(Dialog::RecipeTaskConfirmation(request)) = app.active_dialog() else {
            panic!("forced task was not previewed");
        };
        assert!(request.force);
        assert_eq!(request.targets, vec!["busybox"]);
        let request = request.clone();
        assert_eq!(
            update(&mut app, Action::ConfirmRecipeTask),
            Some(Effect::Start(request))
        );
    }

    #[test]
    fn recipe_bitbake_action_rejects_unavailable_and_malformed_tasks() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes.push(Recipe {
            name: "demo".into(),
            ..Recipe::default()
        });
        let _ = update(&mut app, Action::BeginSelectedRecipeDevshell);
        assert_eq!(
            app.notification.as_deref(),
            Some("Load selected recipe metadata with Enter before choosing a task.")
        );
        app.recipe_metadata.insert(
            "demo".into(),
            RecipeMetadata {
                recipe: "demo".into(),
                tasks: Some(vec!["do_build".into(), "bad task".into()]),
                ..RecipeMetadata::default()
            },
        );
        app.notification = None;
        let _ = update(&mut app, Action::BeginSelectedRecipeMenuConfig);
        assert_eq!(
            app.notification.as_deref(),
            Some("Task menuconfig is not reported for recipe demo.")
        );
        app.notification = None;
        let _ = update(
            &mut app,
            Action::BeginSelectedRecipeTask {
                task: Some("bad task".into()),
                force: true,
            },
        );
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("invalid build target"))
        );
        assert!(app.active_dialog().is_none());
    }
    #[test]
    fn selected_recipe_build_requires_confirmation() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
            ..Recipe::default()
        }];
        let _ = update(&mut app, Action::BeginSelectedRecipeBuild);
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["busybox".into()],
                task: None,
                force: false,
            }))
        );
    }
    #[test]
    fn selected_recipe_clean_prefills_the_clean_task() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
            ..Recipe::default()
        }];
        app.recipe_metadata.insert(
            "busybox".into(),
            RecipeMetadata {
                recipe: "busybox".into(),
                tasks: Some(vec!["do_clean".into()]),
                ..RecipeMetadata::default()
            },
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeClean);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::RecipeTaskConfirmation(BuildRequest {
                targets,
                task: Some(task),
                force: false,
            })) if targets == &vec!["busybox".to_owned()] && task == "clean"
        ));
    }
    #[test]
    fn selected_recipe_menuconfig_prefills_the_menuconfig_task() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
            ..Recipe::default()
        }];
        app.recipe_metadata.insert(
            "busybox".into(),
            RecipeMetadata {
                recipe: "busybox".into(),
                tasks: Some(vec!["do_menuconfig".into()]),
                ..RecipeMetadata::default()
            },
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeMenuConfig);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::RecipeTaskConfirmation(BuildRequest {
                targets,
                task: Some(task),
                force: false,
            })) if targets == &vec!["busybox".to_owned()] && task == "menuconfig"
        ));
    }
    #[test]
    fn devtool_modify_requires_authoritative_status_and_confirmation() {
        let mut app = App::new(10, 1_000);
        let identity = RecipeIdentity {
            name: "busybox".into(),
            file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
        };
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            file: Some(identity.file.clone()),
            ..Recipe::default()
        }];
        assert_eq!(
            update(&mut app, Action::BeginSelectedRecipeDevtoolModify),
            None
        );
        assert_eq!(
            app.notification.as_deref(),
            Some("Refresh authoritative Devtool status with t before modifying.")
        );
        app.devtool_statuses.insert(
            identity.clone(),
            DevtoolStatus {
                identity: identity.clone(),
                capability: DevtoolCapability::MissingExecutable,
                workspace: DevtoolWorkspace::NotMember,
                git: DevtoolGitState::NotApplicable,
                error: None,
            },
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolModify);
        assert_eq!(
            app.notification.as_deref(),
            Some("Devtool executable is missing.")
        );
        app.devtool_statuses.insert(
            identity.clone(),
            DevtoolStatus {
                identity: identity.clone(),
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: DevtoolGitState::NotApplicable,
                error: None,
            },
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolModify);
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::DevtoolModifyConfirmation(identity.clone()))
        );
        assert_eq!(
            update(&mut app, Action::ConfirmDevtoolModify),
            Some(Effect::DevtoolModify(identity.clone()))
        );
        app.devtool_statuses.insert(
            identity.clone(),
            DevtoolStatus {
                identity,
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::Present {
                    source_path: PathBuf::from("/build/workspace/sources/busybox"),
                    recipe_file: None,
                },
                git: DevtoolGitState::NotRepository,
                error: None,
            },
        );
        assert_eq!(
            update(&mut app, Action::BeginSelectedRecipeDevtoolModify),
            Some(Effect::OpenWorkspaceEditor {
                label: "busybox".into(),
                root: PathBuf::from("/build/workspace/sources/busybox"),
            })
        );
    }
    #[test]
    fn selected_recipe_requests_authoritative_dependencies() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
            ..Recipe::default()
        }];
        assert_eq!(
            update(&mut app, Action::BeginSelectedRecipeDependencies),
            Some(Effect::GetDependencies("busybox".into()))
        );
        let _ = update(
            &mut app,
            Action::DependenciesLoaded(RecipeDependencies {
                recipe: "busybox".into(),
                build: vec!["virtual/libc".into()],
                runtime: vec!["base-files".into()],
            }),
        );
        assert_eq!(app.screen, Screen::Dependencies);
        assert_eq!(app.dependencies.as_ref().unwrap().build, ["virtual/libc"]);
        app.workspace.recipes.push(Recipe {
            name: "base-files".into(),
            version: None,
            layer: None,
            ..Recipe::default()
        });
        let _ = update(&mut app, Action::SelectDependency { delta: 1 });
        let _ = update(&mut app, Action::OpenSelectedDependency);
        assert_eq!(app.screen, Screen::Recipes);
        assert_eq!(app.recipe_selection, 1);
    }
    #[test]
    fn dependency_graph_normalizes_nodes_edges_and_reverse_lookup() {
        let root = DependencyNodeId::recipe("image");
        let library = DependencyNodeId::recipe("library");
        let compile = DependencyNodeId::task("library", "do_compile");
        let duplicate_library = DependencyNode {
            id: library.clone(),
            provider: Some(PathBuf::from("/z/library.bb")),
            log: None,
        };
        let preferred_library = DependencyNode {
            id: library.clone(),
            provider: Some(PathBuf::from("/a/library.bb")),
            log: None,
        };
        let build_edge = DependencyEdge {
            from: root.clone(),
            to: library.clone(),
            kind: DependencyEdgeKind::Build,
        };
        let (graph, report) = DependencyGraph::normalize(
            root.clone(),
            vec![duplicate_library, preferred_library],
            vec![
                build_edge.clone(),
                build_edge,
                DependencyEdge {
                    from: library.clone(),
                    to: library.clone(),
                    kind: DependencyEdgeKind::Runtime,
                },
                DependencyEdge {
                    from: library.clone(),
                    to: compile.clone(),
                    kind: DependencyEdgeKind::Task,
                },
            ],
            10,
            10,
        );

        assert_eq!(report.duplicate_nodes, 1);
        assert_eq!(report.duplicate_edges, 1);
        assert_eq!(report.self_edges, 1);
        assert_eq!(report.synthesized_nodes, 1);
        assert!(!report.is_partial());
        assert_eq!(
            graph
                .nodes
                .iter()
                .find(|node| node.id == library)
                .and_then(|node| node.provider.as_deref()),
            Some(Path::new("/a/library.bb"))
        );
        assert_eq!(graph.incoming(&compile).len(), 1);
        assert_eq!(graph.incoming(&compile)[0].from, library);
    }
    #[test]
    fn dependency_graph_finds_deterministic_bounded_paths_through_cycles() {
        let root = DependencyNodeId::recipe("root");
        let a = DependencyNodeId::recipe("a");
        let b = DependencyNodeId::recipe("b");
        let target = DependencyNodeId::recipe("target");
        let isolated = DependencyNodeId::recipe("isolated");
        let edge = |from: &DependencyNodeId, to: &DependencyNodeId| DependencyEdge {
            from: from.clone(),
            to: to.clone(),
            kind: DependencyEdgeKind::Build,
        };
        let (graph, _) = DependencyGraph::normalize(
            root.clone(),
            vec![DependencyNode::identity(isolated.clone())],
            vec![
                edge(&root, &b),
                edge(&b, &target),
                edge(&root, &a),
                edge(&a, &root),
                edge(&a, &target),
            ],
            20,
            20,
        );

        assert_eq!(
            graph.why_built(&target, 4, 20),
            DependencyPathResult::Found(vec![root.clone(), a, target.clone()])
        );
        assert_eq!(
            graph.why_built(&target, 1, 20),
            DependencyPathResult::LimitReached
        );
        assert_eq!(
            graph.why_built(&target, 4, 1),
            DependencyPathResult::LimitReached
        );
        assert_eq!(
            graph.why_built(&isolated, 4, 20),
            DependencyPathResult::Unreachable
        );
    }
    #[test]
    fn dependency_graph_reducer_preserves_identity_and_explicit_states() {
        let root = DependencyNodeId::recipe("image");
        let selected = DependencyNodeId::recipe("library");
        let edge = DependencyEdge {
            from: root.clone(),
            to: selected.clone(),
            kind: DependencyEdgeKind::Build,
        };
        let (graph, _) = DependencyGraph::normalize(root.clone(), Vec::new(), vec![edge], 10, 10);
        let mut app = App::new(10, 1_000);

        assert_eq!(
            update(
                &mut app,
                Action::BeginDependencyGraph { root: root.clone() }
            ),
            Some(Effect::GetDependencies("image".into()))
        );
        assert_eq!(
            app.dependency_graph,
            DependencyGraphState::Loading { root: root.clone() }
        );
        let _ = update(&mut app, Action::DependencyGraphLoaded(graph.clone()));
        app.dependency_graph_selection = Some(selected.clone());
        let _ = update(
            &mut app,
            Action::DependencyGraphPartial {
                graph: graph.clone(),
                limitations: vec!["task edges unavailable".into()],
            },
        );
        assert_eq!(app.dependency_graph_selection, Some(selected.clone()));
        assert!(matches!(
            app.dependency_graph,
            DependencyGraphState::Partial { .. }
        ));

        let (without_selected, _) =
            DependencyGraph::normalize(root.clone(), Vec::new(), Vec::new(), 10, 10);
        let _ = update(&mut app, Action::DependencyGraphLoaded(without_selected));
        assert_eq!(app.dependency_graph_selection, Some(root.clone()));
        assert_eq!(
            app.dependency_graph,
            DependencyGraphState::AvailableEmpty { root: root.clone() }
        );

        let _ = update(
            &mut app,
            Action::DependencyGraphFailed {
                root: root.clone(),
                message: "backend failed".into(),
            },
        );
        assert_eq!(
            app.dependency_graph,
            DependencyGraphState::Failed {
                root,
                message: "backend failed".into()
            }
        );
    }
    #[test]
    fn dependency_graph_normalization_reports_hard_bounds() {
        let root = DependencyNodeId::recipe("root");
        let (graph, report) = DependencyGraph::normalize(
            root.clone(),
            vec![
                DependencyNode::identity(DependencyNodeId::recipe("a")),
                DependencyNode::identity(DependencyNodeId::recipe("b")),
            ],
            vec![DependencyEdge {
                from: root.clone(),
                to: DependencyNodeId::recipe("missing"),
                kind: DependencyEdgeKind::Runtime,
            }],
            1,
            1,
        );
        assert_eq!(graph.nodes, [DependencyNode::identity(root)]);
        assert!(graph.edges.is_empty());
        assert!(report.truncated_nodes >= 2);
        assert_eq!(report.truncated_edges, 1);
        assert!(report.is_partial());
    }
    #[test]
    fn dependency_workspace_routes_only_typed_identity_provider_and_task_log() {
        let root = DependencyNodeId::recipe("image");
        let task = DependencyNodeId::task("busybox", "do_compile");
        let (graph, _) = DependencyGraph::normalize(
            root.clone(),
            vec![DependencyNode {
                id: task.clone(),
                provider: Some(PathBuf::from("/layers/meta/busybox.bb")),
                log: Some(PathBuf::from("/build/tmp/log.do_compile")),
            }],
            vec![DependencyEdge {
                from: root.clone(),
                to: task.clone(),
                kind: DependencyEdgeKind::Task,
            }],
            10,
            10,
        );
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            ..Recipe::default()
        }];
        let _ = update(&mut app, Action::DependencyGraphLoaded(graph));
        app.dependency_graph_selection = Some(task);

        assert_eq!(
            update(&mut app, Action::OpenSelectedDependencyProvider),
            Some(Effect::OpenInEditor(PathBuf::from(
                "/layers/meta/busybox.bb"
            )))
        );
        assert_eq!(
            update(&mut app, Action::OpenSelectedDependencyTaskLog),
            Some(Effect::OpenInEditor(PathBuf::from(
                "/build/tmp/log.do_compile"
            )))
        );
        let _ = update(&mut app, Action::OpenSelectedDependencyRecipe);
        assert_eq!(app.screen, Screen::Recipes);
        assert_eq!(app.recipe_selection, 0);

        let _ = update(&mut app, Action::Open(Screen::Dependencies));
        assert_eq!(
            update(&mut app, Action::RefreshDependencyGraph),
            Some(Effect::GetDependencies("image".into()))
        );
        assert_eq!(
            app.dependency_graph,
            DependencyGraphState::Loading { root: root.clone() }
        );
        assert_eq!(app.dependency_graph_selection, Some(root));
        let _ = update(&mut app, Action::OpenSelectedDependencyTaskLog);
        assert_eq!(
            app.notification.as_deref(),
            Some("Task logs are available only for typed task dependency nodes.")
        );
    }
    fn signature_record(recipe: &str, task: &str, hash: &str, path: &str) -> SignatureRecord {
        SignatureRecord {
            identity: SignatureIdentity {
                target: SignatureTarget {
                    recipe: recipe.into(),
                    task: task.into(),
                },
                hash: Some(hash.into()),
                path: Some(PathBuf::from(path)),
            },
            base_hash: Some(format!("base-{hash}")),
            task_hash: Some(format!("task-{hash}")),
            variables: Vec::new(),
            dependencies: Vec::new(),
        }
    }
    #[test]
    fn signature_model_validates_normalizes_duplicates_and_bounds() {
        let target = SignatureTarget {
            recipe: "busybox".into(),
            task: "do_compile".into(),
        };
        let mut preferred = signature_record("busybox", "do_compile", "aaa", "/tmp/aaa.sigdata");
        preferred.variables = vec![
            SignatureValue {
                name: "Z".into(),
                value: Some("last".into()),
            },
            SignatureValue {
                name: "A".into(),
                value: Some("first".into()),
            },
            SignatureValue {
                name: "A".into(),
                value: Some("second".into()),
            },
        ];
        preferred.dependencies = vec!["z".into(), "a".into(), "a".into()];
        let mut duplicate = preferred.clone();
        duplicate.base_hash = Some("zzz".into());
        let invalid = signature_record("other", "do_compile", "bad", "/tmp/bad.sigdata");
        let overflow = signature_record("busybox", "do_compile", "ccc", "/tmp/ccc.sigdata");

        let (records, report) =
            normalize_signature_records(&target, vec![duplicate, invalid, overflow, preferred], 1);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].base_hash.as_deref(), Some("base-aaa"));
        assert_eq!(
            records[0].variables,
            [
                SignatureValue {
                    name: "A".into(),
                    value: Some("first".into())
                },
                SignatureValue {
                    name: "Z".into(),
                    value: Some("last".into())
                }
            ]
        );
        assert_eq!(records[0].dependencies, ["a", "z"]);
        assert_eq!(report.duplicate_records, 1);
        assert_eq!(report.invalid_records, 1);
        assert_eq!(report.truncated_records, 1);
        assert!(report.is_partial());

        let relative = SignatureIdentity {
            target,
            hash: Some("abc".into()),
            path: Some(PathBuf::from("relative.sigdata")),
        };
        assert_eq!(relative.validate(), Err("signature paths must be absolute"));
    }
    #[test]
    fn signature_model_derives_deterministic_typed_differences() {
        let mut left = signature_record("busybox", "do_compile", "left", "/tmp/left.sigdata");
        left.base_hash = Some("base-left".into());
        left.task_hash = Some("task-left".into());
        left.variables = vec![
            SignatureValue {
                name: "CC".into(),
                value: Some("gcc".into()),
            },
            SignatureValue {
                name: "ONLY_LEFT".into(),
                value: Some("yes".into()),
            },
        ];
        left.dependencies = vec!["dep-left".into(), "dep-shared".into()];
        let mut right = signature_record("busybox", "do_compile", "right", "/tmp/right.sigdata");
        right.base_hash = Some("base-right".into());
        right.task_hash = None;
        right.variables = vec![SignatureValue {
            name: "CC".into(),
            value: Some("clang".into()),
        }];
        right.dependencies = vec!["dep-right".into(), "dep-shared".into()];

        let (differences, report) = compare_signature_records(&left, &right, 20);
        assert!(!report.is_partial());
        assert!(differences.iter().any(|difference| {
            difference.category == SignatureDifferenceCategory::BaseHash
                && difference.key == "base_hash"
        }));
        assert!(differences.iter().any(|difference| {
            difference.category == SignatureDifferenceCategory::ChangedValue
                && difference.key == "CC"
        }));
        assert!(differences.iter().any(|difference| {
            difference.category == SignatureDifferenceCategory::Unavailable
                && difference.key == "ONLY_LEFT"
        }));
        assert_eq!(
            differences
                .iter()
                .filter(|difference| {
                    difference.category == SignatureDifferenceCategory::Dependency
                })
                .count(),
            2
        );
        let (_, bounded) = compare_signature_records(&left, &right, 2);
        assert!(bounded.is_partial());
    }
    #[test]
    fn signature_model_reducer_correlates_states_selection_and_comparison() {
        let target = SignatureTarget {
            recipe: "busybox".into(),
            task: "do_compile".into(),
        };
        let left = signature_record("busybox", "do_compile", "aaa", "/tmp/aaa.sigdata");
        let right = signature_record("busybox", "do_compile", "bbb", "/tmp/bbb.sigdata");
        let mut app = App::new(10, 1_000);
        assert_eq!(
            update(&mut app, Action::BeginSignatureDump(target.clone())),
            Some(Effect::GetSignatureDump(target.clone()))
        );
        let stale_target = SignatureTarget {
            recipe: "other".into(),
            task: "do_compile".into(),
        };
        let _ = update(
            &mut app,
            Action::SignatureDumpLoaded {
                target: stale_target,
                records: vec![left.clone()],
            },
        );
        assert!(matches!(
            app.signature_dump,
            SignatureDumpState::Loading { .. }
        ));
        let _ = update(
            &mut app,
            Action::SignatureDumpLoaded {
                target: target.clone(),
                records: vec![right.clone(), left.clone()],
            },
        );
        assert_eq!(app.signature_selection, Some(left.identity.clone()));
        assert!(matches!(
            app.signature_dump,
            SignatureDumpState::Available { .. }
        ));

        let _ = update(
            &mut app,
            Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Left),
        );
        let _ = update(&mut app, Action::SelectSignatureRecord { delta: 1 });
        assert_eq!(app.signature_selection, Some(right.identity.clone()));
        let _ = update(
            &mut app,
            Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Right),
        );
        let request = SignatureComparisonRequest {
            left: left.identity.clone(),
            right: right.identity.clone(),
        };
        assert_eq!(
            update(&mut app, Action::BeginSignatureComparison),
            Some(Effect::CompareSignatures(request.clone()))
        );
        let stale_request = SignatureComparisonRequest {
            left: right.identity.clone(),
            right: left.identity.clone(),
        };
        let _ = update(
            &mut app,
            Action::SignatureComparisonLoaded {
                request: stale_request,
                differences: Vec::new(),
            },
        );
        assert!(matches!(
            app.signature_comparison,
            SignatureComparisonState::Loading { .. }
        ));
        let _ = update(
            &mut app,
            Action::SignatureComparisonLoaded {
                request: request.clone(),
                differences: vec![SignatureDifference {
                    category: SignatureDifferenceCategory::ChangedValue,
                    key: "CC".into(),
                    left: Some("gcc".into()),
                    right: Some("clang".into()),
                }],
            },
        );
        assert!(matches!(
            app.signature_comparison,
            SignatureComparisonState::Available { .. }
        ));
        let _ = update(
            &mut app,
            Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Left),
        );
        assert!(matches!(
            app.signature_comparison,
            SignatureComparisonState::Ready { .. }
        ));

        let _ = update(&mut app, Action::BeginSignatureDump(target.clone()));
        let _ = update(
            &mut app,
            Action::SignatureDumpPartial {
                target: target.clone(),
                records: vec![right.clone()],
                limitations: vec!["one artifact unreadable".into()],
            },
        );
        assert_eq!(app.signature_selection, Some(right.identity));
        assert!(matches!(
            app.signature_dump,
            SignatureDumpState::Partial { .. }
        ));
        let _ = update(&mut app, Action::BeginSignatureDump(target.clone()));
        let _ = update(
            &mut app,
            Action::SignatureDumpLoaded {
                target: target.clone(),
                records: Vec::new(),
            },
        );
        assert_eq!(
            app.signature_dump,
            SignatureDumpState::AvailableEmpty {
                target: target.clone()
            }
        );
        let _ = update(&mut app, Action::BeginSignatureDump(target.clone()));
        let _ = update(
            &mut app,
            Action::SignatureDumpFailed {
                target: target.clone(),
                message: "tool unavailable".into(),
            },
        );
        assert_eq!(
            app.signature_dump,
            SignatureDumpState::Failed {
                target,
                message: "tool unavailable".into()
            }
        );
    }
    #[test]
    fn devtool_target_reset_requires_authoritative_removable_source_and_confirmation() {
        let mut app = App::new(10, 1_000);
        let identity = RecipeIdentity {
            name: "busybox".into(),
            file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
        };
        let source_path = PathBuf::from("/build/workspace/sources/busybox");
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            file: Some(identity.file.clone()),
            ..Recipe::default()
        }];
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolReset);
        assert_eq!(
            app.notification.as_deref(),
            Some("Refresh authoritative Devtool status with t before reset.")
        );
        app.devtool_statuses.insert(
            identity.clone(),
            DevtoolStatus {
                identity: identity.clone(),
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::Present {
                    source_path: source_path.clone(),
                    recipe_file: Some(identity.file.clone()),
                },
                git: DevtoolGitState::NotRepository,
                error: None,
            },
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolReset);
        let plan = DevtoolResetPlan {
            identity: identity.clone(),
            source_path: source_path.clone(),
        };
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::DevtoolResetConfirmation(plan.clone()))
        );
        app.devtool_statuses.get_mut(&identity).unwrap().workspace =
            DevtoolWorkspace::MissingDirectory {
                source_path: PathBuf::from("/build/workspace/sources/moved"),
            };
        assert_eq!(update(&mut app, Action::ConfirmDevtoolReset), None);
        assert_eq!(
            app.notification.as_deref(),
            Some("The authoritative Devtool reset source changed; refresh with t.")
        );
        app.devtool_statuses.get_mut(&identity).unwrap().workspace =
            DevtoolWorkspace::MissingDirectory { source_path };
        assert_eq!(
            update(&mut app, Action::ConfirmDevtoolReset),
            Some(Effect::DevtoolReset(plan))
        );
    }
    #[test]
    fn devtool_publish_update_requires_authoritative_workspace_and_confirmation() {
        let mut app = App::new(10, 1_000);
        let identity = RecipeIdentity {
            name: "busybox".into(),
            file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
        };
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            file: Some(identity.file.clone()),
            ..Recipe::default()
        }];
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolUpdateRecipe);
        assert_eq!(
            app.notification.as_deref(),
            Some("Refresh authoritative Devtool status with t before update-recipe.")
        );
        app.devtool_statuses.insert(
            identity.clone(),
            DevtoolStatus {
                identity: identity.clone(),
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: DevtoolGitState::NotApplicable,
                error: None,
            },
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolUpdateRecipe);
        assert_eq!(
            app.notification.as_deref(),
            Some("Recipe is not in the Devtool workspace.")
        );
        app.devtool_statuses.insert(
            identity.clone(),
            DevtoolStatus {
                identity: identity.clone(),
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::Present {
                    source_path: PathBuf::from("/build/workspace/sources/busybox"),
                    recipe_file: Some(identity.file.clone()),
                },
                git: DevtoolGitState::NotRepository,
                error: None,
            },
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolUpdateRecipe);
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::DevtoolUpdateConfirmation(identity.clone()))
        );
        assert_eq!(
            update(&mut app, Action::ConfirmDevtoolUpdateRecipe),
            Some(Effect::DevtoolUpdateRecipe(identity))
        );
    }
    #[test]
    fn devtool_publish_finish_requires_clean_status_and_configured_layer_confirmation() {
        let mut app = App::new(10, 1_000);
        let destination = Layer {
            name: "meta-demo".into(),
            path: PathBuf::from("/layers/meta-demo"),
            priority: Some(7),
        };
        app.workspace.layers = vec![
            Layer {
                name: "meta-core".into(),
                path: PathBuf::from("/layers/meta-core"),
                priority: Some(5),
            },
            destination.clone(),
            Layer {
                name: "relative".into(),
                path: PathBuf::from("layers/relative"),
                priority: None,
            },
        ];
        let identity = RecipeIdentity {
            name: "busybox".into(),
            file: PathBuf::from("/layers/meta-core/recipes-core/busybox/busybox.bb"),
        };
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            layer: Some("meta-demo".into()),
            file: Some(identity.file.clone()),
            ..Recipe::default()
        }];
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolFinish);
        assert_eq!(
            app.notification.as_deref(),
            Some("Refresh authoritative Devtool status with t before finish.")
        );
        app.devtool_statuses.insert(
            identity.clone(),
            DevtoolStatus {
                identity: identity.clone(),
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::Present {
                    source_path: PathBuf::from("/build/workspace/sources/busybox"),
                    recipe_file: Some(identity.file.clone()),
                },
                git: DevtoolGitState::Available {
                    branch: Some("devtool".into()),
                    head: Some("abc123".into()),
                    modified: 1,
                    untracked: 0,
                    conflicted: 0,
                },
                error: None,
            },
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolFinish);
        assert_eq!(
            app.notification.as_deref(),
            Some("Commit all workspace changes before Devtool finish.")
        );
        app.devtool_statuses.get_mut(&identity).unwrap().git = DevtoolGitState::Available {
            branch: Some("devtool".into()),
            head: Some("abc123".into()),
            modified: 0,
            untracked: 0,
            conflicted: 0,
        };
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolFinish);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::DevtoolFinishPicker(picker))
                if picker.identity == identity
                    && picker.layers.len() == 2
                    && picker.layers[picker.selection] == destination
        ));
        let _ = update(&mut app, Action::PreviewDevtoolFinish);
        let plan = DevtoolFinishPlan {
            identity: identity.clone(),
            layer: destination.clone(),
        };
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::DevtoolFinishConfirmation(plan.clone()))
        );
        assert_eq!(
            update(&mut app, Action::ConfirmDevtoolFinish),
            Some(Effect::DevtoolFinish(plan))
        );

        app.dialogs
            .push_back(Dialog::DevtoolFinishConfirmation(DevtoolFinishPlan {
                identity,
                layer: Layer {
                    name: "meta-rogue".into(),
                    path: PathBuf::from("/tmp/meta-rogue"),
                    priority: None,
                },
            }));
        assert_eq!(update(&mut app, Action::ConfirmDevtoolFinish), None);
        assert_eq!(
            app.notification.as_deref(),
            Some("The selected finish layer is no longer configured.")
        );
    }
    #[test]
    fn devtool_target_deploy_requires_authoritative_workspace_and_validated_confirmation() {
        let mut app = App::new(10, 1_000);
        let identity = RecipeIdentity {
            name: "busybox".into(),
            file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
        };
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            file: Some(identity.file.clone()),
            ..Recipe::default()
        }];
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolDeploy);
        assert_eq!(
            app.notification.as_deref(),
            Some("Refresh authoritative Devtool status with t before deploy-target.")
        );
        app.devtool_statuses.insert(
            identity.clone(),
            DevtoolStatus {
                identity: identity.clone(),
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::Present {
                    source_path: PathBuf::from("/build/workspace/sources/busybox"),
                    recipe_file: Some(identity.file.clone()),
                },
                git: DevtoolGitState::NotRepository,
                error: None,
            },
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolDeploy);
        let _ = update(&mut app, Action::AppendDevtoolDeployTarget('q'));
        let _ = update(&mut app, Action::AppendDevtoolDeployTarget('e'));
        let _ = update(&mut app, Action::AppendDevtoolDeployTarget('m'));
        let _ = update(&mut app, Action::AppendDevtoolDeployTarget('u'));
        let _ = update(&mut app, Action::PreviewDevtoolDeploy);
        let plan = DevtoolDeployPlan {
            identity: identity.clone(),
            target: "qemu".into(),
        };
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::DevtoolDeployConfirmation(plan.clone()))
        );
        assert_eq!(
            update(&mut app, Action::ConfirmDevtoolDeploy),
            Some(Effect::DevtoolDeploy(plan))
        );

        app.dialogs
            .push_back(Dialog::DevtoolDeploy(DevtoolDeployDraft {
                identity,
                target: "--help".into(),
            }));
        assert_eq!(update(&mut app, Action::PreviewDevtoolDeploy), None);
        assert_eq!(
            app.notification.as_deref(),
            Some(
                "Devtool target must be one non-option value without whitespace or control characters"
            )
        );
    }
    #[test]
    fn devtool_modify_editor_loads_saves_and_builds_selected_recipe() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes.push(Recipe {
            name: "busybox".into(),
            ..Recipe::default()
        });
        let root = PathBuf::from("/build/workspace/sources/busybox");
        assert_eq!(
            update(
                &mut app,
                Action::OpenRecipeEditor {
                    recipe: "busybox".into(),
                    root: root.clone(),
                    files: vec![PathBuf::from("main.c")],
                },
            ),
            Some(Effect::LoadRecipeEditorFile(root.join("main.c")))
        );
        let _ = update(
            &mut app,
            Action::LoadRecipeEditorContent("int main() {}".into()),
        );
        let _ = update(&mut app, Action::ToggleRecipeEditorEditing);
        let _ = update(&mut app, Action::AppendRecipeEditor('\n'));
        let _ = update(&mut app, Action::BeginRecipeEditorBuild);
        assert_eq!(
            app.notification.as_deref(),
            Some("Save workspace changes before starting the recipe build.")
        );
        assert!(matches!(app.active_dialog(), Some(Dialog::RecipeEditor(_))));
        assert_eq!(
            update(&mut app, Action::SaveRecipeEditor),
            Some(Effect::SaveRecipeEditorFile {
                path: root.join("main.c"),
                content: "int main() {}\n".into(),
            })
        );
        let _ = update(&mut app, Action::RecipeEditorSaved);
        let _ = update(&mut app, Action::BeginRecipeEditorBuild);
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["busybox".into()],
                task: None,
                force: false,
            }))
        );
    }
    #[test]
    fn clean_state_requires_confirmation_before_starting() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
            ..Recipe::default()
        }];
        app.recipe_metadata.insert(
            "busybox".into(),
            RecipeMetadata {
                recipe: "busybox".into(),
                tasks: Some(vec!["do_cleansstate".into()]),
                ..RecipeMetadata::default()
            },
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeCleanState);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::RecipeTaskConfirmation(_))
        ));
        assert_eq!(app.build.status, BuildStatus::Idle);

        assert_eq!(
            update(&mut app, Action::ConfirmRecipeTask),
            Some(Effect::Start(BuildRequest {
                targets: vec!["busybox".into()],
                task: Some("cleansstate".into()),
                force: false,
            }))
        );
        assert_eq!(app.build.status, BuildStatus::LoadingWorkspace);
    }
    #[test]
    fn layer_selection_stays_in_workspace_bounds() {
        let mut app = App::new(10, 1_000);
        app.workspace.layers = vec![
            Layer {
                name: "alpha".into(),
                path: PathBuf::from("/layers/alpha"),
                priority: Some(1),
            },
            Layer {
                name: "beta".into(),
                path: PathBuf::from("/layers/beta"),
                priority: None,
            },
        ];
        let _ = update(&mut app, Action::SelectLayer { delta: 8 });
        assert_eq!(app.layer_selection, 1);
        let _ = update(&mut app, Action::SelectLayer { delta: -8 });
        assert_eq!(app.layer_selection, 0);
    }
    #[test]
    fn selected_layer_opens_its_directory() {
        let mut app = App::new(10, 1_000);
        app.workspace.layers = vec![Layer {
            name: "meta-demo".into(),
            path: PathBuf::from("/layers/meta-demo"),
            priority: None,
        }];
        assert_eq!(
            update(&mut app, Action::OpenSelectedLayer),
            Some(Effect::OpenInEditor(PathBuf::from("/layers/meta-demo")))
        );
    }
    #[test]
    fn selected_layer_opens_the_in_tui_workspace_editor() {
        let mut app = App::new(10, 1_000);
        app.workspace.layers = vec![Layer {
            name: "meta-demo".into(),
            path: PathBuf::from("/layers/meta-demo"),
            priority: None,
        }];
        assert_eq!(
            update(&mut app, Action::BeginSelectedLayerWorkspaceEditor),
            Some(Effect::OpenWorkspaceEditor {
                label: "Layer: meta-demo".into(),
                root: PathBuf::from("/layers/meta-demo"),
            })
        );
    }
    #[test]
    fn layer_tree_loads_children_lazily_and_collapses_without_losing_parent() {
        let mut app = App::new(10, 1_000);
        app.workspace.layers.push(Layer {
            name: "meta-demo".into(),
            path: "/layers/meta-demo".into(),
            priority: Some(5),
        });
        assert_eq!(
            update(&mut app, Action::BeginSelectedLayerBrowser),
            Some(Effect::LoadLayerBrowserDirectory {
                layer: "meta-demo".into(),
                root: "/layers/meta-demo".into(),
                directory: "/layers/meta-demo".into(),
            })
        );
        let _ = update(
            &mut app,
            Action::LoadLayerBrowserDirectory {
                layer: "meta-demo".into(),
                root: "/layers/meta-demo".into(),
                directory: "/layers/meta-demo".into(),
                entries: vec![LayerBrowserEntry {
                    path: "/layers/meta-demo/recipes-core".into(),
                    is_dir: true,
                    ..LayerBrowserEntry::default()
                }],
            },
        );
        assert_eq!(
            update(&mut app, Action::LayerBrowserEnter),
            Some(Effect::LoadLayerBrowserDirectory {
                layer: "meta-demo".into(),
                root: "/layers/meta-demo".into(),
                directory: "/layers/meta-demo/recipes-core".into(),
            })
        );
        let _ = update(
            &mut app,
            Action::LoadLayerBrowserDirectory {
                layer: "meta-demo".into(),
                root: "/layers/meta-demo".into(),
                directory: "/layers/meta-demo/recipes-core".into(),
                entries: vec![LayerBrowserEntry {
                    path: "/layers/meta-demo/recipes-core/demo.bb".into(),
                    ..LayerBrowserEntry::default()
                }],
            },
        );
        let browser = app.layer_browser.as_ref().unwrap();
        assert_eq!(browser.entries.len(), 2);
        assert_eq!(browser.entries[0].depth, 0);
        assert_eq!(browser.entries[1].depth, 1);
        assert!(
            browser
                .nodes
                .contains_key(&PathBuf::from("/layers/meta-demo/recipes-core"))
        );
        let _ = update(&mut app, Action::LayerBrowserUp);
        assert_eq!(app.layer_browser.as_ref().unwrap().entries.len(), 1);
    }
    #[test]
    fn layer_tree_hidden_filter_and_search_keep_selection_bounded() {
        let mut app = App::new(10, 1_000);
        let _ = update(
            &mut app,
            Action::LoadLayerBrowserDirectory {
                layer: "meta-demo".into(),
                root: "/layers/meta-demo".into(),
                directory: "/layers/meta-demo".into(),
                entries: vec![
                    LayerBrowserEntry {
                        path: "/layers/meta-demo/.hidden".into(),
                        is_hidden: true,
                        ..LayerBrowserEntry::default()
                    },
                    LayerBrowserEntry {
                        path: "/layers/meta-demo/visible.bb".into(),
                        ..LayerBrowserEntry::default()
                    },
                ],
            },
        );
        assert_eq!(app.layer_browser.as_ref().unwrap().entries.len(), 1);
        let _ = update(&mut app, Action::ToggleLayerBrowserHidden);
        assert_eq!(app.layer_browser.as_ref().unwrap().entries.len(), 2);
        let _ = update(&mut app, Action::BeginMetadataSearch);
        let _ = update(&mut app, Action::AppendMetadataQuery('v'));
        let _ = update(
            &mut app,
            Action::SelectLayerBrowserEntry { delta: isize::MAX },
        );
        assert_eq!(
            app.layer_browser
                .as_ref()
                .unwrap()
                .selected_entry()
                .unwrap()
                .path,
            PathBuf::from("/layers/meta-demo/visible.bb")
        );
    }
    #[test]
    fn layer_tree_ignores_stale_preview_and_tracks_binary_metadata() {
        let mut app = App::new(10, 1_000);
        let path = PathBuf::from("/layers/meta-demo/image.bin");
        let _ = update(
            &mut app,
            Action::LoadLayerBrowserDirectory {
                layer: "meta-demo".into(),
                root: "/layers/meta-demo".into(),
                directory: "/layers/meta-demo".into(),
                entries: vec![LayerBrowserEntry {
                    path: path.clone(),
                    size: Some(100_000),
                    git: GitFileState::Untracked,
                    ..LayerBrowserEntry::default()
                }],
            },
        );
        let _ = update(
            &mut app,
            Action::LoadLayerBrowserPreview {
                path: PathBuf::from("/layers/meta-demo/stale.bb"),
                content: "stale".into(),
                kind: PreviewKind::Text,
                truncated: false,
            },
        );
        assert!(app.layer_browser.as_ref().unwrap().preview.is_empty());
        let _ = update(
            &mut app,
            Action::LoadLayerBrowserPreview {
                path,
                content: String::new(),
                kind: PreviewKind::Binary,
                truncated: true,
            },
        );
        let browser = app.layer_browser.as_ref().unwrap();
        assert_eq!(browser.preview_kind, PreviewKind::Binary);
        assert!(browser.preview_truncated);
    }
    #[test]
    fn layer_tree_external_editor_effect_is_typed_and_missing_selection_is_visible() {
        let mut app = App::new(10, 1_000);
        let mut browser = LayerBrowser::new("meta-demo".into(), "/layers/meta-demo".into());
        browser.entries.push(LayerBrowserEntry {
            path: "/layers/meta-demo/recipes-demo/demo/demo.bb".into(),
            ..LayerBrowserEntry::default()
        });
        app.layer_browser = Some(browser);
        assert_eq!(
            update(&mut app, Action::EditSelectedLayerBrowserFile),
            Some(Effect::OpenLayerBrowserEditor {
                layer: "meta-demo".into(),
                root: "/layers/meta-demo".into(),
                file: "recipes-demo/demo/demo.bb".into(),
            })
        );
        app.layer_browser.as_mut().unwrap().entries.clear();
        assert_eq!(update(&mut app, Action::EditSelectedLayerBrowserFile), None);
        assert_eq!(app.notification.as_deref(), Some("Select a file to edit."));
    }
    #[test]
    fn configuration_selection_stays_in_workspace_bounds() {
        let mut app = App::new(10, 1_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemuarm".into());
        app.workspace
            .variables
            .insert("DISTRO".into(), "poky".into());
        let _ = update(&mut app, Action::SelectConfigVariable { delta: 8 });
        assert_eq!(app.config_selection, 1);
        let _ = update(&mut app, Action::SelectConfigVariable { delta: -8 });
        assert_eq!(app.config_selection, 0);
    }
    #[test]
    fn config_source_opens_single_typed_relative_operation() {
        let mut app = App::new(10, 1_000);
        app.workspace.build_dir = Some(PathBuf::from("/build"));
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemuarm".into());
        let identity = VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_details.insert(
            identity.clone(),
            VariableDetail {
                identity,
                effective_value: Some("qemuarm".into()),
                unexpanded_value: None,
                provenance: Some("conf/local.conf:12".into()),
                operations: vec![VariableOperation {
                    operation: "set".into(),
                    file: Some("conf/local.conf".into()),
                    line: Some(12),
                    value: Some("qemuarm".into()),
                }],
                active_overrides: vec![],
            },
        );
        assert_eq!(
            update(&mut app, Action::OpenSelectedConfigSource),
            Some(Effect::OpenInEditor(PathBuf::from(
                "/build/conf/local.conf"
            )))
        );
    }

    #[test]
    fn config_source_picker_uses_typed_operation_line_and_restores_focus() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Inspector;
        app.workspace.build_dir = Some("/build".into());
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemuarm".into());
        let identity = VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_details.insert(
            identity.clone(),
            VariableDetail {
                identity,
                effective_value: Some("qemuarm".into()),
                unexpanded_value: None,
                provenance: None,
                operations: vec![
                    VariableOperation {
                        operation: "set".into(),
                        file: Some("meta/conf/bitbake.conf".into()),
                        line: Some(10),
                        value: None,
                    },
                    VariableOperation {
                        operation: "override".into(),
                        file: Some("conf/local.conf".into()),
                        line: Some(12),
                        value: None,
                    },
                ],
                active_overrides: vec![],
            },
        );
        assert_eq!(update(&mut app, Action::OpenSelectedConfigSource), None);
        assert_eq!(app.focus, FocusTarget::Dialog);
        let Some(Dialog::ConfigSourcePicker(picker)) = app.active_dialog() else {
            panic!("source picker was not opened");
        };
        assert_eq!(picker.sources[1].operation, "override");
        assert_eq!(picker.sources[1].line, Some(12));
        let _ = update(&mut app, Action::SelectConfigSource { delta: 1 });
        assert_eq!(
            update(&mut app, Action::OpenSelectedConfigSourceChoice),
            Some(Effect::OpenInEditor("/build/conf/local.conf".into()))
        );
        assert_eq!(app.focus, FocusTarget::Inspector);
    }

    #[test]
    fn config_source_rejects_escape_and_explains_unloaded_detail() {
        let mut app = App::new(10, 1_000);
        app.workspace.build_dir = Some("/build".into());
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemuarm".into());
        assert_eq!(update(&mut app, Action::OpenSelectedConfigSource), None);
        assert!(app.notification.as_deref().unwrap().contains("with Enter"));
        let identity = VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_details.insert(
            identity.clone(),
            VariableDetail {
                identity,
                effective_value: Some("qemuarm".into()),
                unexpanded_value: None,
                provenance: None,
                operations: vec![VariableOperation {
                    operation: "set".into(),
                    file: Some("../outside.conf".into()),
                    line: Some(1),
                    value: None,
                }],
                active_overrides: vec![],
            },
        );
        let _ = update(&mut app, Action::OpenSelectedConfigSource);
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("escapes the build directory")
        );
    }
    #[test]
    fn metadata_search_tracks_query_and_resets_metadata_selection() {
        let mut app = App::new(10, 1_000);
        app.recipe_selection = 3;
        app.layer_selection = 2;
        app.config_selection = 1;

        let _ = update(&mut app, Action::BeginMetadataSearch);
        let _ = update(&mut app, Action::AppendMetadataQuery('q'));
        let _ = update(&mut app, Action::AppendMetadataQuery('e'));

        assert!(app.metadata_searching);
        assert_eq!(app.metadata_query, "qe");
        assert_eq!(
            (
                app.recipe_selection,
                app.layer_selection,
                app.config_selection
            ),
            (0, 0, 0)
        );

        let _ = update(&mut app, Action::BackspaceMetadataQuery);
        let _ = update(&mut app, Action::FinishMetadataSearch);
        assert_eq!(app.metadata_query, "q");
        assert!(!app.metadata_searching);
    }
    #[test]
    fn log_match_navigation_stays_within_active_search_results() {
        let mut app = App::new(10, 1_000);
        app.logs.insert(log("alpha match"));
        app.logs.insert(log("not relevant"));
        app.logs.insert(log("beta match"));
        app.logs.query = "match".into();

        let _ = update(&mut app, Action::NextLogMatch);
        assert_eq!(app.logs.selection, 1);
        assert_eq!(app.logs.match_position(), Some((2, 2)));
        assert!(!app.logs.follow);

        let _ = update(&mut app, Action::NextLogMatch);
        assert_eq!(app.logs.selection, 1);
        let _ = update(&mut app, Action::PreviousLogMatch);
        assert_eq!(app.logs.selection, 0);
        assert_eq!(app.logs.scroll_offset, 1);
    }
    #[test]
    fn build_target_editor_requires_confirmation_before_starting() {
        let mut app = App::new(10, 1_000);
        let _ = update(&mut app, Action::BeginBuildTargetEdit);
        if let Some(Dialog::BuildTarget { editor, .. }) = app.active_dialog_mut() {
            editor.text = "target = \"core-image-minimal\"\n".into();
            editor.cursor = editor.text.len();
        }
        let effect = update(&mut app, Action::ConfirmBuildTarget);

        assert_eq!(effect, None);
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
                force: false,
            }))
        );
    }
    #[test]
    fn image_picker_selects_an_image_then_requires_build_confirmation() {
        let mut app = App::new(10, 1_000);
        let _ = update(
            &mut app,
            Action::OpenImagePicker(vec!["core-image-base".into(), "core-image-minimal".into()]),
        );
        let _ = update(&mut app, Action::SelectImage { delta: 1 });
        let _ = update(&mut app, Action::ConfirmImagePicker);
        assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
        let _ = update(&mut app, Action::BeginCurrentImageBuild);
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
                force: false,
            }))
        );
    }

    #[test]
    fn theme_picker_applies_named_selection_immediately_and_persists_on_accept() {
        let mut app = App::new(10, 1_000);
        app.color_enabled = false;
        assert_eq!(update(&mut app, Action::OpenThemePicker), None);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::ThemePicker { .. })
        ));
        let _ = update(&mut app, Action::SelectTheme { delta: 1 });
        assert_eq!(app.theme, Theme::WhiteClassic);
        assert!(app.color_enabled);
        assert!(app.settings_dirty);
        assert!(matches!(
            update(&mut app, Action::ApplySelectedTheme),
            Some(Effect::PersistSettings)
        ));
        assert!(app.active_dialog().is_none());
    }

    #[test]
    fn theme_picker_respects_no_color_launch_override() {
        let mut app = App::new(10, 1_000);
        app.color_enabled = false;
        app.color_forced_off = true;
        let _ = update(&mut app, Action::OpenThemePicker);
        let _ = update(&mut app, Action::SelectTheme { delta: 1 });
        assert_eq!(app.theme, Theme::WhiteClassic);
        assert!(!app.color_enabled);

        app.settings_selection = 3;
        assert_eq!(
            update(&mut app, Action::ChangeSelectedSetting { backwards: false }),
            None
        );
        assert_eq!(
            app.notification.as_deref(),
            Some("Color is disabled by --no-color for this launch")
        );
    }

    #[test]
    fn theme_picker_restores_original_theme_when_closed_with_escape() {
        let mut app = App::new(10, 1_000);
        app.color_enabled = false;
        let _ = update(&mut app, Action::OpenThemePicker);
        let _ = update(&mut app, Action::SelectTheme { delta: 2 });
        assert_eq!(app.theme, Theme::MatrixGreen);
        assert!(app.color_enabled);
        let _ = update(&mut app, Action::CloseThemePicker);
        assert_eq!(app.theme, Theme::DarkPro);
        assert!(!app.color_enabled);
        assert!(!app.settings_dirty);
        assert!(app.active_dialog().is_none());
    }
    #[test]
    fn build_completion_stays_open_until_dismissed() {
        let mut app = App::new(10, 1_000);
        app.build.target = Some("core-image-minimal".into());
        let _ = update(
            &mut app,
            Action::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
        );
        assert!(matches!(app.active_dialog(), Some(Dialog::BuildCompletion)));
        let _ = update(&mut app, Action::DismissBuildCompletion);
        assert!(app.active_dialog().is_none());
    }
    #[test]
    fn build_options_prefill_the_current_target_and_requested_task() {
        let mut app = App::new(10, 1_000);
        app.build.target = Some("core-image-minimal".into());

        let _ = update(&mut app, Action::OpenBuildOptions);
        assert!(matches!(app.active_dialog(), Some(Dialog::BuildOptions)));
        assert_eq!(app.focus, FocusTarget::Dialog);
        let _ = update(&mut app, Action::BeginBuildTargetTask(Some("clean".into())));

        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::BuildTarget { editor, task })
                if !editor.editing
                    && editor.text.contains("target = \"core-image-minimal\"")
                    && editor.selected_text() == Some("core-image-minimal")
                    && task.as_deref() == Some("clean")
        ));
    }
    #[test]
    fn host_telemetry_updates_current_values_and_bounds_valid_history() {
        let mut app = App::new(10, 1_000);
        for sample in 0..75 {
            let telemetry = HostTelemetry {
                cpu_utilization_percent: Some(sample),
                memory_total_bytes: Some(1_000),
                memory_available_bytes: Some(750),
                disk_available_bytes: Some(8 * 1024 * 1024 * 1024),
                ..HostTelemetry::default()
            };
            let _ = update(&mut app, Action::HostTelemetryUpdated(telemetry));
        }
        let telemetry = app.host_telemetry.clone();
        assert_eq!(app.host_telemetry, telemetry);
        assert_eq!(app.host_cpu_history.len(), 60);
        assert_eq!(app.host_cpu_history.front(), Some(&15));
        assert_eq!(app.host_cpu_history.back(), Some(&74));
        assert_eq!(app.host_memory_history.len(), 60);
        assert!(app.host_memory_history.iter().all(|sample| *sample == 25));

        let _ = update(
            &mut app,
            Action::HostTelemetryUpdated(HostTelemetry {
                cpu_utilization_percent: None,
                memory_total_bytes: Some(100),
                memory_available_bytes: Some(101),
                ..HostTelemetry::default()
            }),
        );
        assert_eq!(app.host_cpu_history.len(), 60);
        assert_eq!(app.host_memory_history.len(), 60);
    }
    #[test]
    fn settings_selection_and_changes_are_typed_and_persisted() {
        let mut app = App::new(10, 1_000);
        assert_eq!(SETTINGS[app.settings_selection], Setting::Theme);
        assert_eq!(
            update(&mut app, Action::ChangeSelectedSetting { backwards: false }),
            Some(Effect::PersistSettings)
        );
        assert_eq!(app.theme, Theme::WhiteClassic);
        assert!(app.settings_dirty);

        let _ = update(&mut app, Action::SelectSetting { delta: 99 });
        assert_eq!(SETTINGS[app.settings_selection], Setting::LogFollow);
        assert_eq!(
            update(&mut app, Action::ChangeSelectedSetting { backwards: true }),
            Some(Effect::PersistSettings)
        );
        assert!(!app.logs.follow);
        assert_eq!(app.logs.paused_len, Some(0));

        let _ = update(&mut app, Action::SettingsPersisted);
        assert!(!app.settings_dirty);
        assert!(app.notification.is_none());
    }
    #[test]
    fn settings_persistence_failure_retains_the_preview_and_dirty_state() {
        let mut app = App::new(10, 1_000);
        let _ = update(&mut app, Action::ChangeSelectedSetting { backwards: true });
        assert_eq!(app.theme, Theme::HighContrast);

        let _ = update(
            &mut app,
            Action::SettingsPersistenceFailed("read-only filesystem".into()),
        );
        assert_eq!(app.theme, Theme::HighContrast);
        assert!(app.settings_dirty);
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("read-only filesystem")
        );
        assert_eq!(
            update(&mut app, Action::RetrySettingsPersistence),
            Some(Effect::PersistSettings)
        );
    }
    #[test]
    fn animation_ticks_advance_unless_reduced_motion_is_enabled() {
        let mut app = App::new(10, 1_000);
        let _ = update(&mut app, Action::Tick);
        let _ = update(&mut app, Action::Tick);
        assert_eq!(app.animation_frame, 2);

        app.reduced_motion = true;
        let _ = update(&mut app, Action::Tick);
        assert_eq!(app.animation_frame, 2);
    }
    #[test]
    fn typed_event_actions_update_metadata_and_preserve_unknown_progress() {
        let mut app = App::new(10, 1_000);
        let _ = update(
            &mut app,
            Action::RecipesLoaded(vec![
                Recipe {
                    name: "zlib".into(),
                    version: None,
                    layer: Some("core".into()),
                    ..Recipe::default()
                },
                Recipe {
                    name: "base-files".into(),
                    version: None,
                    layer: Some("core".into()),
                    ..Recipe::default()
                },
            ]),
        );
        let _ = update(
            &mut app,
            Action::LayersLoaded(vec![Layer {
                name: "core".into(),
                path: "/poky/meta".into(),
                priority: Some(5),
            }]),
        );
        let _ = update(
            &mut app,
            Action::VariableLoaded(VariableDetail {
                identity: VariableIdentity {
                    name: "MACHINE".into(),
                    recipe: None,
                },
                effective_value: Some("qemux86-64".into()),
                unexpanded_value: None,
                provenance: Some("/build/conf/local.conf:1".into()),
                operations: vec![],
                active_overrides: vec![],
            }),
        );
        let _ = update(
            &mut app,
            Action::RecipeSourcesLoaded {
                recipe: "base-files".into(),
                paths: vec!["/poky/meta/recipes-core/base-files/base-files.bb".into()],
            },
        );
        assert_eq!(app.workspace.recipes[0].name, "base-files");
        assert_eq!(app.workspace.layers[0].path, PathBuf::from("/poky/meta"));
        assert_eq!(app.workspace.variables["MACHINE"], "qemux86-64");
        assert_eq!(
            app.recipe_sources["base-files"][0],
            PathBuf::from("/poky/meta/recipes-core/base-files/base-files.bb")
        );

        let id = TaskId("base-files:do_install".into());
        let _ = update(
            &mut app,
            Action::TaskStarted(TaskInfo {
                id: id.clone(),
                recipe: "base-files".into(),
                task: "do_install".into(),
                progress: None,
                ..TaskInfo::default()
            }),
        );
        let _ = update(
            &mut app,
            Action::TaskProgress {
                id: id.clone(),
                progress: None,
            },
        );
        assert_eq!(app.tasks[&id].progress, None);
        let _ = update(
            &mut app,
            Action::TaskProgress {
                id: id.clone(),
                progress: Some(250),
            },
        );
        assert_eq!(app.tasks[&id].progress, Some(100));
    }
    #[test]
    fn bbmask_editing_requires_a_preview_and_confirmation() {
        let mut app = App::new(10, 1_000);
        app.workspace
            .variables
            .insert("BBMASK".into(), "meta-old/.*".into());
        let _ = update(&mut app, Action::BeginBbmaskEdit);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::BbmaskEdit(editor))
                if editor.text == "bbmask = \"meta-old/.*\"\n"
                    && editor.selected_text() == Some("meta-old/.*")
        ));
        if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog_mut() {
            editor.text = "bbmask = \"meta-old/.* x\"\n".into();
            editor.cursor = editor.text.len();
        }
        let _ = update(&mut app, Action::PreviewBbmaskEdit);
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::BbmaskConfirmation("meta-old/.* x".into()))
        );
        assert_eq!(
            update(&mut app, Action::ConfirmBbmaskWrite),
            Some(Effect::WriteBbmask("meta-old/.* x".into()))
        );
    }
    proptest! {
        #[test]
        fn retention_never_exceeds_count_or_bytes(messages in proptest::collection::vec(".{0,64}", 0..80), max_entries in 1usize..20, max_bytes in 1usize..256) {
            let mut logs = LogState::new(max_entries, max_bytes);
            for message in messages { logs.insert(log(&message)); }
            prop_assert!(logs.entries.len() <= max_entries);
            prop_assert!(logs.retained_bytes <= max_bytes || logs.entries.is_empty());
            prop_assert_eq!(logs.retained_bytes, logs.entries.iter().map(|entry| entry.message.len()).sum::<usize>());
        }
    }
    #[test]
    fn hardening_stress_model_retention_preserves_high_volume_invariants() {
        const EVENTS: usize = 20_000;
        let mut app = App::new(128, 4_096);
        let _ = update(&mut app, Action::ToggleLogFollow);
        for index in 0..EVENTS {
            let severity = match index % 4 {
                0 => Severity::Trace,
                1 => Severity::Info,
                2 => Severity::Warning,
                _ => Severity::Error,
            };
            let _ = update(
                &mut app,
                Action::Log(LogEntry {
                    id: 0,
                    severity,
                    message: format!("stress-event-{index:05}-{}", "x".repeat(index % 31)),
                    recipe: Some(format!("recipe-{}", index % 17)),
                    task: Some(format!("do_task_{}", index % 11)),
                    path: None,
                    timestamp: SystemTime::UNIX_EPOCH + Duration::from_millis(index as u64),
                    build: Some(format!("build-{}", index % 3)),
                    protected: false,
                    diagnostic: None,
                }),
            );
            if index % 257 == 0 {
                let _ = update(&mut app, Action::ScrollLogs { delta: 19 });
            }
        }

        assert!(app.logs.entries.len() <= app.logs.max_entries);
        assert!(app.logs.retained_bytes <= app.logs.max_bytes);
        assert_eq!(
            app.logs.retained_bytes,
            app.logs
                .entries
                .iter()
                .map(|entry| entry.message.len())
                .sum::<usize>()
        );
        assert_eq!(app.logs.dropped + app.logs.entries.len(), EVENTS);
        assert_eq!(app.logs.coalesced, 0);
        assert_eq!(app.build.warnings, EVENTS / 4);
        assert_eq!(app.build.errors, EVENTS / 4);
        let visible = app.logs.filtered().count();
        assert!(visible == 0 || app.logs.selection < visible);
        assert!(app.logs.scroll_offset <= visible.saturating_sub(1));
    }
    #[test]
    fn running_build_requires_confirmation() {
        let mut a = App::new(2, 10);
        a.build.status = BuildStatus::Running;
        update(&mut a, Action::Quit);
        assert!(matches!(a.active_dialog(), Some(Dialog::QuitConfirmation)));
        assert!(!a.should_quit)
    }
    #[test]
    fn duplicate_or_unknown_completion_does_not_increment_task_count() {
        let mut app = App::new(2, 10);
        let id = TaskId("busybox:do_compile".into());
        let _ = update(
            &mut app,
            Action::TaskStarted(TaskInfo {
                id: id.clone(),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: None,
                ..TaskInfo::default()
            }),
        );
        let _ = update(
            &mut app,
            Action::TaskCompleted {
                id: id.clone(),
                success: true,
            },
        );
        let _ = update(&mut app, Action::TaskCompleted { id, success: true });
        assert_eq!(app.build.completed, 1);
        assert_eq!(app.completed_tasks.len(), 1);
        assert!(app.completed_tasks.front().is_some_and(|task| task.success));
    }
    #[test]
    fn build_task_scrolling_stays_within_observed_task_history() {
        let mut app = App::new(2, 10);
        for recipe in ["busybox", "bash"] {
            let id = TaskId(format!("{recipe}:do_compile"));
            let _ = update(
                &mut app,
                Action::TaskStarted(TaskInfo {
                    id: id.clone(),
                    recipe: recipe.into(),
                    task: "do_compile".into(),
                    progress: None,
                    ..TaskInfo::default()
                }),
            );
            let _ = update(&mut app, Action::TaskCompleted { id, success: true });
        }
        let _ = update(&mut app, Action::ScrollBuildTasks { delta: 8 });
        assert_eq!(app.task_progress_scroll, 1);
        let _ = update(&mut app, Action::ScrollBuildTasks { delta: -8 });
        assert_eq!(app.task_progress_scroll, 0);
    }
    #[test]
    fn log_filters_combine_severity_recipe_task_and_search() {
        let mut logs = LogState::new(10, 1_000);
        logs.insert(tagged_log(
            "busybox",
            "do_compile",
            Severity::Warning,
            "Compiler warning",
        ));
        logs.insert(tagged_log(
            "bash",
            "do_install",
            Severity::Warning,
            "Install warning",
        ));
        logs.filter = Some(Severity::Warning);
        logs.recipe_filter = Some("busybox".into());
        logs.task_filter = Some("do_compile".into());
        logs.query = "compiler".into();
        assert_eq!(logs.filtered().count(), 1);
    }
    #[test]
    fn toggles_log_view_preferences() {
        let mut app = App::new(2, 10);
        let _ = update(&mut app, Action::ToggleLogFollow);
        let _ = update(&mut app, Action::ToggleLogWrap);
        assert!(!app.logs.follow);
        assert!(app.logs.wrap);
    }
    #[test]
    fn paused_log_view_holds_the_visible_horizon() {
        let mut app = App::new(10, 100);
        app.logs.insert(log("before pause"));
        let _ = update(&mut app, Action::ToggleLogFollow);
        app.logs.insert(log("after pause"));
        assert_eq!(app.logs.filtered().count(), 1);
        let _ = update(&mut app, Action::ToggleLogFollow);
        assert_eq!(app.logs.filtered().count(), 2);
    }

    #[test]
    fn scrolling_logs_pauses_follow_and_bounds_offset() {
        let mut app = App::new(10, 1_000);
        let _ = update(&mut app, Action::Log(log("first")));
        let _ = update(&mut app, Action::Log(log("second")));
        let _ = update(&mut app, Action::ScrollLogs { delta: 9 });
        assert!(!app.logs.follow);
        assert_eq!(app.logs.scroll_offset, 1);
        assert_eq!(
            app.logs.selected().map(|entry| entry.message.as_str()),
            Some("first")
        );
        let _ = update(&mut app, Action::ScrollLogs { delta: -9 });
        assert_eq!(app.logs.scroll_offset, 0);
        assert_eq!(
            app.logs.selected().map(|entry| entry.message.as_str()),
            Some("second")
        );
    }
    #[test]
    fn cycles_log_severity_filter() {
        let mut app = App::new(2, 10);
        for expected in [
            Some(Severity::Info),
            Some(Severity::Warning),
            Some(Severity::Error),
            None,
        ] {
            let _ = update(&mut app, Action::CycleLogSeverity);
            assert_eq!(app.logs.filter, expected);
        }
    }
    #[test]
    fn log_retention_prefers_important_diagnostics_and_reports_coalescing() {
        let mut logs = LogState::new(3, 1_000);
        logs.insert(tagged_log(
            "busybox",
            "do_compile",
            Severity::Warning,
            "warning retained",
        ));
        logs.insert(tagged_log(
            "busybox",
            "do_compile",
            Severity::Error,
            "error retained",
        ));
        for index in 0..20 {
            logs.insert(log(&format!("ordinary {index}")));
        }
        assert_eq!(logs.entries.len(), 3);
        assert!(
            logs.entries
                .iter()
                .any(|entry| entry.message == "warning retained")
        );
        assert!(
            logs.entries
                .iter()
                .any(|entry| entry.message == "error retained")
        );
        assert_eq!(logs.dropped_warnings, 0);
        assert_eq!(logs.dropped_errors, 0);

        logs.insert(log("repeat"));
        logs.insert(log("repeat"));
        assert_eq!(logs.coalesced, 1);
    }
    #[test]
    fn log_build_filter_selection_source_and_copy_are_typed() {
        let mut app = App::new(10, 1_000);
        app.build.target = Some("core-image-minimal".into());
        let mut first = tagged_log("busybox", "do_compile", Severity::Info, "compiler output");
        first.path = Some(PathBuf::from("/tmp/log.do_compile"));
        let _ = update(&mut app, Action::Log(first));
        app.build.target = Some("core-image-full-cmdline".into());
        let _ = update(&mut app, Action::Log(log("second build")));

        let _ = update(&mut app, Action::CycleLogBuildFilter);
        assert_eq!(
            app.logs.build_filter.as_deref(),
            Some("core-image-full-cmdline")
        );
        assert_eq!(app.logs.filtered().count(), 1);
        let _ = update(&mut app, Action::CycleLogBuildFilter);
        assert_eq!(app.logs.build_filter.as_deref(), Some("core-image-minimal"));
        assert_eq!(
            update(&mut app, Action::OpenSelectedLogSource),
            Some(Effect::OpenInEditor(PathBuf::from("/tmp/log.do_compile")))
        );
        let Some(Effect::CopyToClipboard(details)) = update(&mut app, Action::CopySelectedLog)
        else {
            panic!("selected log details were not copied through a typed effect");
        };
        assert!(details.contains("Build: core-image-minimal"));
        assert!(details.contains("compiler output"));
    }
    #[test]
    fn log_terminal_diagnostics_are_protected_and_observable() {
        let mut app = App::new(10, 1_000);
        app.build.target = Some("core-image-minimal".into());
        let _ = update(
            &mut app,
            Action::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
        );
        let entry = app.logs.entries.back().unwrap();
        assert!(entry.protected);
        assert_eq!(entry.build.as_deref(), Some("core-image-minimal"));
        assert!(entry.message.contains("completed"));
    }
    #[test]
    fn request_validation() {
        assert!(
            BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
                force: false,
            }
            .validate()
            .is_ok()
        );
        assert!(
            BuildRequest {
                targets: vec!["bad target".into()],
                task: None,
                force: false,
            }
            .validate()
            .is_err()
        );
        assert!(
            BuildRequest {
                targets: vec!["..".into()],
                task: None,
                force: false,
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn live_tasks_reducer_keeps_honest_counts_filters_and_bounded_selection() {
        let mut app = App::new(20, 2_000);
        let first = TaskId("busybox:do_compile".into());
        let second = TaskId("openssl:do_install".into());
        let mut busybox = TaskInfo::active(first.clone(), "busybox".into(), "do_compile".into());
        busybox.worker = Some("worker-1".into());
        busybox.stats = Some(TaskStats {
            completed: 1,
            total: 5,
            active: 1,
            failed: 0,
        });
        let _ = update(&mut app, Action::TaskStarted(busybox));
        let mut openssl = TaskInfo::active(second.clone(), "openssl".into(), "do_install".into());
        openssl.worker = Some("worker-2".into());
        let _ = update(&mut app, Action::TaskStarted(openssl));
        assert_eq!(app.build.completed, 1);
        assert_eq!(app.build.total, Some(5));
        assert_eq!(app.waiting_task_count(), 2);
        assert!(matches!(
            app.visible_task_rows().last(),
            Some(TaskRow::WaitingSummary(2))
        ));

        let _ = update(
            &mut app,
            Action::TaskCompleted {
                id: second,
                success: false,
            },
        );
        let _ = update(&mut app, Action::CycleTaskStateFilter);
        assert_eq!(app.task_filters.state, TaskStateFilter::Active);
        assert_eq!(app.visible_task_rows().len(), 1);
        for _ in 0..3 {
            let _ = update(&mut app, Action::CycleTaskStateFilter);
        }
        assert_eq!(app.task_filters.state, TaskStateFilter::Failed);
        assert!(matches!(
            app.visible_task_rows().as_slice(),
            [TaskRow::Task(task)] if task.recipe == "openssl" && task.state == TaskState::Failed
        ));

        app.task_progress_scroll = 99;
        let _ = update(&mut app, Action::CycleTaskDurationFilter);
        assert_eq!(app.task_progress_scroll, 0);
        let _ = update(
            &mut app,
            Action::TaskCompleted {
                id: first,
                success: true,
            },
        );
        assert!(app.task_progress_scroll <= app.visible_task_rows().len().saturating_sub(1));
        let _ = update(
            &mut app,
            Action::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
        );
        assert_eq!(app.build.completed, 5);
        assert_eq!(app.waiting_task_count(), 0);
    }

    #[test]
    fn live_tasks_filter_supports_recipe_task_worker_and_duration() {
        let mut app = App::new(20, 2_000);
        let mut task = TaskInfo::active(
            TaskId("linux-yocto:do_compile_kernel".into()),
            "linux-yocto".into(),
            "do_compile_kernel".into(),
        );
        task.worker = Some("remote-7".into());
        task.started = Some(SystemTime::UNIX_EPOCH);
        task.finished = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(20));
        task.state = TaskState::Completed;
        app.completed_tasks.push_back(CompletedTask {
            task,
            success: true,
        });
        app.task_filters.recipe = "LINUX".into();
        app.task_filters.task = "kernel".into();
        app.task_filters.worker = "REMOTE".into();
        app.task_filters.minimum_duration = Some(Duration::from_secs(10));
        assert_eq!(app.visible_task_rows().len(), 1);
        app.task_filters.minimum_duration = Some(Duration::from_secs(60));
        assert!(app.visible_task_rows().is_empty());
    }

    #[test]
    fn recipe_navigation_uses_authoritative_provider_logs_and_local_patches() {
        let mut app = App::new(20, 4_000);
        app.workspace.recipes.push(Recipe {
            name: "busybox".into(),
            file: Some("/layers/meta/recipes-core/busybox/busybox_1.0.bb".into()),
            ..Recipe::default()
        });
        app.recipe_metadata.insert(
            "busybox".into(),
            RecipeMetadata {
                recipe: "busybox".into(),
                patches: Some(vec![
                    "/layers/meta/recipes-core/busybox/files/a.patch".into(),
                    "https://example.invalid/remote.diff".into(),
                    "/layers/meta/recipes-core/busybox/files/b.patch".into(),
                ]),
                ..RecipeMetadata::default()
            },
        );
        let mut active = TaskInfo::active(
            TaskId("busybox:do_compile".into()),
            "busybox".into(),
            "do_compile".into(),
        );
        active.log_path = Some("/tmp/log.do_compile".into());
        app.tasks.insert(active.id.clone(), active);
        let mut completed = TaskInfo::active(
            TaskId("busybox:do_install".into()),
            "busybox".into(),
            "do_install".into(),
        );
        completed.state = TaskState::Completed;
        completed.log_path = Some("/tmp/log.do_install".into());
        app.completed_tasks.push_back(CompletedTask {
            task: completed,
            success: true,
        });

        assert_eq!(
            update(&mut app, Action::OpenSelectedRecipeProvider),
            Some(Effect::OpenInEditor(
                "/layers/meta/recipes-core/busybox/busybox_1.0.bb".into()
            ))
        );
        assert_eq!(update(&mut app, Action::BeginSelectedRecipeTaskLog), None);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::RecipeTaskLogPicker(picker)) if picker.logs.len() == 2
        ));
        let _ = update(&mut app, Action::SelectRecipeTaskLog { delta: 1 });
        assert_eq!(
            update(&mut app, Action::OpenSelectedRecipeTaskLog),
            Some(Effect::OpenInEditor("/tmp/log.do_install".into()))
        );

        assert_eq!(
            update(&mut app, Action::BeginSelectedRecipePatchReview),
            None
        );
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::RecipePatchPicker(picker)) if picker.patches.len() == 2
        ));
        let _ = update(&mut app, Action::SelectRecipePatch { delta: 1 });
        assert_eq!(
            update(&mut app, Action::OpenSelectedRecipePatch),
            Some(Effect::OpenInEditor(
                "/layers/meta/recipes-core/busybox/files/b.patch".into()
            ))
        );
    }

    #[test]
    fn recipe_navigation_explains_missing_and_remote_only_paths() {
        let mut app = App::new(20, 4_000);
        app.workspace.recipes.push(Recipe {
            name: "demo".into(),
            ..Recipe::default()
        });
        let _ = update(&mut app, Action::OpenSelectedRecipeProvider);
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("provider path")
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeTaskLog);
        assert!(app.notification.as_deref().unwrap().contains("evicted"));
        app.recipe_metadata.insert(
            "demo".into(),
            RecipeMetadata {
                recipe: "demo".into(),
                patches: Some(vec!["file://unresolved.patch".into()]),
                ..RecipeMetadata::default()
            },
        );
        let _ = update(&mut app, Action::BeginSelectedRecipePatchReview);
        assert!(app.notification.as_deref().unwrap().contains("unresolved"));
    }

    #[test]
    fn recipe_qa_action_requires_authoritative_tasks_and_exact_confirmation() {
        let mut app = App::new(20, 4_000);
        app.workspace.recipes.push(Recipe {
            name: "busybox".into(),
            ..Recipe::default()
        });
        let _ = update(&mut app, Action::BeginSelectedRecipeCveCheck);
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("Load selected recipe metadata")
        );
        app.recipe_metadata.insert(
            "busybox".into(),
            RecipeMetadata {
                recipe: "busybox".into(),
                tasks: Some(vec!["do_cve_check".into(), "do_create_spdx".into()]),
                ..RecipeMetadata::default()
            },
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeCveCheck);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::RecipeTaskConfirmation(BuildRequest {
                targets,
                task: Some(task),
                force: false,
            })) if targets == &["busybox"] && task == "cve_check"
        ));
        assert_eq!(
            update(&mut app, Action::ConfirmRecipeTask),
            Some(Effect::Start(BuildRequest {
                targets: vec!["busybox".into()],
                task: Some("cve_check".into()),
                force: false,
            }))
        );
        let _ = update(&mut app, Action::BeginSelectedRecipeSpdx);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::RecipeTaskConfirmation(BuildRequest {
                task: Some(task),
                ..
            })) if task == "create_spdx"
        ));
        let _ = update(&mut app, Action::CancelRecipeTask);
        app.recipe_metadata.get_mut("busybox").unwrap().tasks = Some(vec![]);
        let _ = update(&mut app, Action::BeginSelectedRecipeSpdx);
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("Task create_spdx is not reported")
        );
    }

    #[test]
    fn recipe_qa_action_reducer_retains_output_and_honest_empty_artifacts() {
        let mut app = App::new(20, 4_000);
        let id = BackgroundJobId(9);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(BackgroundJobSpec {
                id,
                kind: BackgroundJobKind::CveCheck,
                title: "CVE check busybox".into(),
                context: BackgroundJobContext {
                    workspace: Some(Screen::Recipes),
                    recipe: Some("busybox".into()),
                    task: Some("cve_check".into()),
                    ..BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at: SystemTime::UNIX_EPOCH,
            }),
        );
        let _ = update(
            &mut app,
            Action::StartBackgroundJob {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::RunBackgroundJob { id });
        let _ = update(
            &mut app,
            Action::AppendBackgroundJobOutput {
                id,
                entry: BackgroundJobOutputEntry {
                    severity: Severity::Warning,
                    message: "CVE-2026-0001 requires review".into(),
                    source: BackgroundJobOutputSource::Backend,
                    truncated: false,
                    timestamp: SystemTime::UNIX_EPOCH,
                },
            },
        );
        let _ = update(
            &mut app,
            Action::SucceedBackgroundJob {
                id,
                result: BackgroundJobResult {
                    summary: "CVE check completed; BitBake reported no result path".into(),
                    artifacts: vec![],
                },
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let job = app.background_jobs.get(id).unwrap();
        assert_eq!(job.status, BackgroundJobStatus::Succeeded);
        assert_eq!(job.warnings, 1);
        assert_eq!(
            job.output.back().unwrap().message,
            "CVE-2026-0001 requires review"
        );
        assert!(job.result.as_ref().unwrap().artifacts.is_empty());
    }

    #[test]
    fn config_metadata_keeps_global_and_recipe_scopes_typed_and_independent() {
        let mut app = App::new(20, 4_000);
        let global = VariableDetail {
            identity: VariableIdentity {
                name: "PACKAGE_ARCH".into(),
                recipe: None,
            },
            effective_value: Some("qemux86_64".into()),
            unexpanded_value: Some("${MACHINE_ARCH}".into()),
            provenance: Some("/build/conf/local.conf:8".into()),
            operations: vec![VariableOperation {
                operation: "set".into(),
                file: Some("/build/conf/local.conf".into()),
                line: Some(8),
                value: Some("${MACHINE_ARCH}".into()),
            }],
            active_overrides: vec!["qemux86-64".into()],
        };
        let _ = update(&mut app, Action::VariableLoaded(global.clone()));
        assert_eq!(app.workspace.variables["PACKAGE_ARCH"], "qemux86_64");
        assert_eq!(
            app.workspace.variable_provenance_chain["PACKAGE_ARCH"],
            ["/build/conf/local.conf:8"]
        );

        let recipe = VariableDetail {
            identity: VariableIdentity {
                name: "PACKAGE_ARCH".into(),
                recipe: Some("base-files".into()),
            },
            effective_value: Some("all".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        };
        let _ = update(&mut app, Action::VariableLoaded(recipe.clone()));
        assert_eq!(
            app.workspace.variables["PACKAGE_ARCH"], "qemux86_64",
            "a scoped response must not overwrite global summary state"
        );
        assert_eq!(
            app.variable_details
                .get(&recipe.identity)
                .unwrap()
                .effective_value
                .as_deref(),
            Some("all")
        );
        assert_eq!(app.variable_details.get(&global.identity), Some(&global));
    }

    #[test]
    fn devtool_metadata_uses_absolute_identity_and_ignores_other_recipe_status() {
        let mut app = App::new(20, 4_000);
        app.workspace.recipes = vec![
            Recipe {
                name: "busybox".into(),
                file: Some("/layers/core/recipes-core/busybox/busybox_1.0.bb".into()),
                ..Recipe::default()
            },
            Recipe {
                name: "bash".into(),
                file: Some("/layers/core/recipes-extended/bash/bash_5.0.bb".into()),
                ..Recipe::default()
            },
        ];
        let busybox = match update(&mut app, Action::BeginSelectedRecipeDevtoolStatus) {
            Some(Effect::InspectDevtoolStatus(identity)) => identity,
            effect => panic!("unexpected effect: {effect:?}"),
        };
        assert!(app.devtool_status_loading.contains(&busybox));

        let bash = RecipeIdentity {
            name: "bash".into(),
            file: "/layers/core/recipes-extended/bash/bash_5.0.bb".into(),
        };
        let _ = update(
            &mut app,
            Action::DevtoolStatusLoaded(DevtoolStatus {
                identity: bash.clone(),
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: DevtoolGitState::NotApplicable,
                error: None,
            }),
        );
        assert!(
            app.devtool_status_loading.contains(&busybox),
            "a response for another absolute recipe identity is stale for the selection"
        );
        assert_eq!(app.devtool_statuses[&bash].identity, bash);
        assert!(
            app.devtool_statuses[&bash]
                .disabled_reason(DevtoolAction::ModifyOrEdit)
                .is_none()
        );
        assert_eq!(
            app.devtool_statuses[&bash]
                .disabled_reason(DevtoolAction::UpdateRecipe)
                .as_deref(),
            Some("Recipe is not in the Devtool workspace.")
        );
    }

    #[test]
    fn devtool_metadata_rejects_missing_or_relative_provider_identity() {
        let mut app = App::new(20, 4_000);
        app.workspace.recipes.push(Recipe {
            name: "busybox".into(),
            file: Some("recipes-core/busybox.bb".into()),
            ..Recipe::default()
        });
        assert_eq!(
            update(&mut app, Action::BeginSelectedRecipeDevtoolStatus),
            None
        );
        assert_eq!(
            app.notification.as_deref(),
            Some("The selected recipe provider path is not absolute.")
        );
        assert!(app.devtool_status_loading.is_empty());
    }

    #[test]
    fn config_workspace_lazy_detail_is_identity_correlated_and_search_bounded() {
        let mut app = App::new(20, 4_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace
            .variables
            .insert("DISTRO".into(), "poky".into());
        app.metadata_query = "machine".into();
        let identity = match update(&mut app, Action::BeginSelectedConfigDetail) {
            Some(Effect::GetVariable(identity)) => identity,
            effect => panic!("unexpected effect: {effect:?}"),
        };
        assert_eq!(identity.name, "MACHINE");
        assert_eq!(identity.recipe, None);
        assert!(app.variable_detail_loading.contains(&identity));

        let scoped = VariableDetail {
            identity: VariableIdentity {
                name: "MACHINE".into(),
                recipe: Some("base-files".into()),
            },
            effective_value: Some("qemux86-64".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        };
        let _ = update(&mut app, Action::VariableLoaded(scoped.clone()));
        assert!(
            app.variable_detail_loading.contains(&identity),
            "a scoped response must not complete the selected global request"
        );
        assert_eq!(app.variable_details.get(&scoped.identity), Some(&scoped));

        let _ = update(&mut app, Action::SelectConfigVariable { delta: 99 });
        assert_eq!(app.config_selection, 0);
        let _ = update(
            &mut app,
            Action::VariableDetailFailed {
                identity: identity.clone(),
                message: "server unavailable".into(),
            },
        );
        assert!(!app.variable_detail_loading.contains(&identity));
        assert_eq!(
            app.variable_detail_errors
                .get(&identity)
                .map(String::as_str),
            Some("server unavailable")
        );
    }

    #[test]
    fn config_workspace_refresh_preserves_selected_variable_identity() {
        let mut app = App::new(20, 4_000);
        app.workspace.variables.insert("A".into(), "one".into());
        app.workspace.variables.insert("B".into(), "two".into());
        app.config_selection = 1;
        let mut workspace = Workspace::default();
        workspace.variables.insert("B".into(), "updated".into());
        workspace.variables.insert("C".into(), "three".into());
        let _ = update(&mut app, Action::WorkspaceLoaded(workspace));
        assert_eq!(app.config_selection, 0);
        assert_eq!(
            selected_config_identity(&app).map(|identity| identity.name),
            Some("B".into())
        );
    }

    #[test]
    fn config_copy_uses_only_loaded_detail_for_the_exact_identity() {
        let mut app = App::new(20, 4_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "summary-value".into());
        assert_eq!(
            update(&mut app, Action::CopySelectedConfigEffective),
            None,
            "the summary value must not be copied as authoritative detail"
        );
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("with Enter"))
        );
        let identity = VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_details.insert(
            identity.clone(),
            VariableDetail {
                identity,
                effective_value: Some("qemux86-64".into()),
                unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            },
        );
        assert_eq!(
            update(&mut app, Action::CopySelectedConfigEffective),
            Some(Effect::CopyToClipboard("qemux86-64".into()))
        );
        assert_eq!(
            update(&mut app, Action::CopySelectedConfigUnexpanded),
            Some(Effect::CopyToClipboard("${DEFAULT_MACHINE}".into()))
        );
    }

    #[test]
    fn config_copy_explains_loading_failure_and_absent_unexpanded_value() {
        let mut app = App::new(20, 4_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let identity = VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_detail_loading.insert(identity.clone());
        assert_eq!(update(&mut app, Action::CopySelectedConfigEffective), None);
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("still loading")
        );
        app.variable_detail_loading.clear();
        app.variable_detail_errors
            .insert(identity.clone(), "Tinfoil unavailable".into());
        let _ = update(&mut app, Action::CopySelectedConfigEffective);
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("Tinfoil unavailable")
        );
        app.variable_detail_errors.clear();
        app.variable_details.insert(
            identity.clone(),
            VariableDetail {
                identity,
                effective_value: Some("qemux86-64".into()),
                unexpanded_value: None,
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            },
        );
        let _ = update(&mut app, Action::CopySelectedConfigUnexpanded);
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("unexpanded value")
        );
    }

    #[test]
    fn config_scope_keeps_global_and_recipe_detail_independent() {
        let mut app = App::new(20, 4_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace.recipes.push(Recipe {
            name: "base-files".into(),
            ..Recipe::default()
        });
        let global = VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_details.insert(
            global.clone(),
            VariableDetail {
                identity: global.clone(),
                effective_value: Some("global-machine".into()),
                unexpanded_value: None,
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            },
        );
        let _ = update(&mut app, Action::OpenConfigScopePicker);
        let Some(Dialog::ConfigScopePicker(picker)) = app.active_dialog() else {
            panic!("scope picker was not opened");
        };
        assert_eq!(picker.scopes, [None, Some("base-files".into())]);
        let _ = update(&mut app, Action::SelectConfigScope { delta: 1 });
        let scoped = match update(&mut app, Action::ConfirmConfigScope) {
            Some(Effect::GetVariable(identity)) => identity,
            effect => panic!("unexpected effect: {effect:?}"),
        };
        assert_eq!(scoped.recipe.as_deref(), Some("base-files"));
        assert!(app.variable_detail_loading.contains(&scoped));
        assert_eq!(
            app.variable_details[&global].effective_value.as_deref(),
            Some("global-machine")
        );
        let _ = update(
            &mut app,
            Action::VariableLoaded(VariableDetail {
                identity: scoped.clone(),
                effective_value: Some("recipe-machine".into()),
                unexpanded_value: None,
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            }),
        );
        assert_eq!(
            update(&mut app, Action::CopySelectedConfigEffective),
            Some(Effect::CopyToClipboard("recipe-machine".into()))
        );
    }

    #[test]
    fn config_scope_falls_back_to_global_when_recipe_disappears() {
        let mut app = App::new(20, 4_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace.recipes.push(Recipe {
            name: "base-files".into(),
            ..Recipe::default()
        });
        app.config_scope = Some("base-files".into());
        let _ = update(&mut app, Action::RecipesLoaded(vec![]));
        assert_eq!(app.config_scope, None);
        let _ = update(&mut app, Action::OpenConfigScopePicker);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::ConfigScopePicker(picker)) if picker.scopes == [None]
        ));
    }

    #[test]
    fn config_compare_reports_equal_different_and_unavailable_fields() {
        let mut app = App::new(20, 4_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace.recipes.push(Recipe {
            name: "base-files".into(),
            ..Recipe::default()
        });
        app.config_scope = Some("base-files".into());
        for (recipe, effective, unexpanded) in [
            (None, Some("qemux86-64"), Some("${DEFAULT_MACHINE}")),
            (Some("base-files"), Some("qemux86-64"), None),
        ] {
            let identity = VariableIdentity {
                name: "MACHINE".into(),
                recipe: recipe.map(str::to_owned),
            };
            app.variable_details.insert(
                identity.clone(),
                VariableDetail {
                    identity,
                    effective_value: effective.map(str::to_owned),
                    unexpanded_value: unexpanded.map(str::to_owned),
                    provenance: None,
                    operations: vec![],
                    active_overrides: vec![],
                },
            );
        }
        let comparison = config_comparison(&app).unwrap();
        assert_eq!(comparison.effective.outcome, ConfigComparisonOutcome::Equal);
        assert_eq!(
            comparison.unexpanded.outcome,
            ConfigComparisonOutcome::Unavailable
        );
        let _ = update(&mut app, Action::OpenConfigComparison);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::ConfigComparison(value)) if value == &comparison
        ));
        let _ = update(&mut app, Action::CloseConfigComparison);
        assert!(app.active_dialog().is_none());

        app.variable_details
            .get_mut(&VariableIdentity {
                name: "MACHINE".into(),
                recipe: Some("base-files".into()),
            })
            .unwrap()
            .effective_value = Some("qemuarm".into());
        assert_eq!(
            config_comparison(&app).unwrap().effective.outcome,
            ConfigComparisonOutcome::Different
        );
    }

    #[test]
    fn config_compare_explains_scope_loading_and_missing_detail() {
        let mut app = App::new(20, 4_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        assert_eq!(
            config_comparison(&app),
            Err("Select a recipe scope with s before comparing.".into())
        );
        app.workspace.recipes.push(Recipe {
            name: "base-files".into(),
            ..Recipe::default()
        });
        app.config_scope = Some("base-files".into());
        let scoped = VariableIdentity {
            name: "MACHINE".into(),
            recipe: Some("base-files".into()),
        };
        app.variable_detail_loading.insert(scoped);
        assert!(
            config_comparison(&app)
                .unwrap_err()
                .contains("still loading")
        );
    }

    #[test]
    fn config_edit_preview_requires_allowlisted_loaded_global_detail() {
        let mut app = App::new(20, 4_000);
        app.focus = FocusTarget::Inspector;
        app.workspace.build_dir = Some("/build".into());
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let identity = VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_details.insert(
            identity.clone(),
            VariableDetail {
                identity: identity.clone(),
                effective_value: Some("qemux86-64".into()),
                unexpanded_value: None,
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            },
        );
        let _ = update(&mut app, Action::BeginConfigEdit);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::ConfigEdit { identity: selected, editor })
                if selected == &identity
                    && !editor.editing
                    && editor.text.contains("value = \"qemux86-64\"")
                    && editor.selected_text() == Some("qemux86-64")
        ));
        assert!(matches!(
            update(
                &mut app,
                Action::EditActivePopup(PopupEditorCommand::Copy)
            ),
            Some(Effect::CopyToClipboard(value)) if value == "qemux86-64"
        ));
        if let Some(Dialog::ConfigEdit { editor, .. }) = app.active_dialog_mut() {
            editor.text = "# MACHINE\nvalue = \"qemux86-64\\\"\"\n".into();
            editor.cursor = editor.text.len();
        }
        let _ = update(&mut app, Action::PreviewConfigEdit);
        let Some(Dialog::ConfigEditConfirmation(request)) = app.active_dialog() else {
            panic!("confirmation was not opened");
        };
        assert_eq!(request.destination, PathBuf::from("/build/conf/local.conf"));
        assert_eq!(request.assignment, "MACHINE = \"qemux86-64\\\"\"");
        let expected = request.clone();
        assert_eq!(
            update(&mut app, Action::ConfirmConfigEdit),
            Some(Effect::WriteConfigAssignment(expected))
        );
        assert_eq!(app.focus, FocusTarget::Inspector);
    }

    #[test]
    fn config_edit_preview_rejects_read_only_scope_and_control_injection() {
        let mut app = App::new(20, 4_000);
        app.workspace.build_dir = Some("/build".into());
        app.workspace
            .variables
            .insert("BB_NUMBER_THREADS".into(), "8".into());
        let identity = VariableIdentity {
            name: "BB_NUMBER_THREADS".into(),
            recipe: None,
        };
        app.variable_details.insert(
            identity.clone(),
            VariableDetail {
                identity,
                effective_value: Some("8".into()),
                unexpanded_value: None,
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            },
        );
        let _ = update(&mut app, Action::BeginConfigEdit);
        assert!(app.notification.as_deref().unwrap().contains("read-only"));
        assert!(app.active_dialog().is_none());

        assert_eq!(
            config_edit_assignment("MACHINE", "qemu\nMALICIOUS = \"1\""),
            Err("Configuration values cannot contain newlines or control characters.".into())
        );
        app.config_scope = Some("base-files".into());
        assert!(
            config_edit_disabled_reason(&app)
                .unwrap()
                .contains("Recipe-scoped")
        );
    }

    #[test]
    fn config_edit_write_revalidates_request_and_preserves_detail_on_failures() {
        let identity = VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        let request = ConfigEditRequest {
            identity: identity.clone(),
            value: "qemux86-64".into(),
            destination: "/build/conf/local.conf".into(),
            assignment: "MACHINE = \"qemux86-64\"".into(),
        };
        assert_eq!(
            validate_config_edit_request(&request, Path::new("/build")),
            Ok(())
        );

        let mut tampered = request.clone();
        tampered.assignment = "MACHINE = \"injected\"".into();
        assert!(
            validate_config_edit_request(&tampered, Path::new("/build"))
                .unwrap_err()
                .contains("does not match")
        );
        let mut scoped = request.clone();
        scoped.identity.recipe = Some("base-files".into());
        assert!(
            validate_config_edit_request(&scoped, Path::new("/build"))
                .unwrap_err()
                .contains("Recipe-scoped")
        );

        let mut app = App::new(10, 1_000);
        let detail = VariableDetail {
            identity: identity.clone(),
            effective_value: Some("old".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        };
        app.variable_details
            .insert(identity.clone(), detail.clone());
        assert_eq!(
            update(
                &mut app,
                Action::ConfigEditWriteSucceeded {
                    identity: identity.clone(),
                },
            ),
            Some(Effect::GetVariable(identity.clone()))
        );
        assert!(app.variable_detail_loading.contains(&identity));
        let _ = update(
            &mut app,
            Action::ConfigEditRefreshFailed {
                identity: identity.clone(),
                message: "bridge unavailable".into(),
            },
        );
        assert_eq!(app.variable_details.get(&identity), Some(&detail));
        assert!(!app.variable_detail_loading.contains(&identity));

        let _ = update(
            &mut app,
            Action::ConfigEditWriteFailed {
                identity: identity.clone(),
                message: "permission denied".into(),
            },
        );
        assert_eq!(app.variable_details.get(&identity), Some(&detail));
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("permission denied")
        );
    }

    #[test]
    fn devtool_job_spec_validates_every_typed_operation() {
        let operations = [
            DevtoolOperation::Modify {
                recipe: "busybox".into(),
            },
            DevtoolOperation::UpdateRecipe {
                recipe: "busybox".into(),
            },
            DevtoolOperation::Finish {
                recipe: "busybox".into(),
                destination: "/layers/meta-custom".into(),
            },
            DevtoolOperation::DeployTarget {
                recipe: "busybox".into(),
                target: "root@192.0.2.1:/opt".into(),
            },
            DevtoolOperation::UndeployTarget {
                recipe: "busybox".into(),
                target: "root@192.0.2.1".into(),
            },
            DevtoolOperation::Reset {
                recipe: "busybox".into(),
            },
        ];
        for operation in operations {
            assert_eq!(operation.recipe(), "busybox");
            assert_eq!(operation.validate(), Ok(()));
        }
    }

    #[test]
    fn devtool_job_spec_rejects_ambiguous_tokens_and_relative_finish_destinations() {
        for recipe in ["", "busy box", "busy\nbox", "--help"] {
            assert_eq!(
                DevtoolOperation::Modify {
                    recipe: recipe.into(),
                }
                .validate(),
                Err(DevtoolOperationError::InvalidRecipe)
            );
        }
        for target in ["", "root@host /opt", "root@host\n--help", "--help"] {
            assert_eq!(
                DevtoolOperation::DeployTarget {
                    recipe: "busybox".into(),
                    target: target.into(),
                }
                .validate(),
                Err(DevtoolOperationError::InvalidTarget)
            );
        }
        assert_eq!(
            DevtoolOperation::Finish {
                recipe: "busybox".into(),
                destination: "meta-custom".into(),
            }
            .validate(),
            Err(DevtoolOperationError::RelativeFinishDestination)
        );
    }

    #[test]
    fn devtool_job_lifecycle_retains_typed_output_and_outcome_across_navigation() {
        let mut app = App::new(10, 1_000);
        let id = BackgroundJobId(1_u64 << 63);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(BackgroundJobSpec {
                id,
                kind: BackgroundJobKind::Devtool,
                title: "Devtool reset busybox".into(),
                context: BackgroundJobContext {
                    workspace: Some(Screen::Recipes),
                    recipe: Some("busybox".into()),
                    ..BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at: SystemTime::UNIX_EPOCH,
            }),
        );
        let _ = update(
            &mut app,
            Action::StartBackgroundJob {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::RunBackgroundJob { id });
        let _ = update(
            &mut app,
            Action::AppendBackgroundJobOutput {
                id,
                entry: BackgroundJobOutputEntry {
                    severity: Severity::Info,
                    message: "workspace reset".into(),
                    source: BackgroundJobOutputSource::Stderr,
                    truncated: true,
                    timestamp: SystemTime::UNIX_EPOCH,
                },
            },
        );
        let _ = update(&mut app, Action::Open(Screen::Dashboard));
        let _ = update(
            &mut app,
            Action::SucceedBackgroundJob {
                id,
                result: BackgroundJobResult {
                    summary: "Devtool completed successfully".into(),
                    artifacts: vec![],
                },
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let job = app.background_jobs.get(id).unwrap();
        assert_eq!(job.status, BackgroundJobStatus::Succeeded);
        assert_eq!(job.context.recipe.as_deref(), Some("busybox"));
        assert_eq!(job.output[0].source, BackgroundJobOutputSource::Stderr);
        assert!(job.output[0].truncated);
        assert_eq!(app.screen, Screen::Dashboard);
    }

    #[test]
    fn signature_workspace_uses_authoritative_task_picker_and_exact_provider_identity() {
        let provider = PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb");
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Recipes;
        app.focus = FocusTarget::Inspector;
        app.workspace.recipes.push(Recipe {
            name: "busybox".into(),
            file: Some(provider.clone()),
            ..Recipe::default()
        });

        let _ = update(&mut app, Action::BeginSelectedRecipeSignatures);
        assert_eq!(
            app.notification.as_deref(),
            Some("Load authoritative recipe tasks with Enter before inspecting signatures.")
        );
        app.notification = None;
        app.recipe_metadata.insert(
            "busybox".into(),
            RecipeMetadata {
                recipe: "busybox".into(),
                tasks: Some(vec![
                    "do_fetch".into(),
                    "bad task".into(),
                    "do_compile".into(),
                    "do_compile".into(),
                ]),
                ..RecipeMetadata::default()
            },
        );

        let _ = update(&mut app, Action::BeginSelectedRecipeSignatures);
        let Some(Dialog::SignatureTaskPicker(picker)) = app.active_dialog() else {
            panic!("signature task picker was not opened");
        };
        assert_eq!(picker.recipe.name, "busybox");
        assert_eq!(picker.recipe.file, provider);
        assert_eq!(picker.tasks, ["do_compile", "do_fetch"]);
        assert_eq!(app.focus, FocusTarget::Dialog);

        let _ = update(&mut app, Action::SelectSignatureTask { delta: 1 });
        assert_eq!(
            update(&mut app, Action::ConfirmSignatureTask),
            Some(Effect::GetSignatureDump(SignatureTarget {
                recipe: "busybox".into(),
                task: "do_fetch".into(),
            }))
        );
        assert_eq!(app.screen, Screen::Signatures);
        assert_eq!(app.focus, FocusTarget::Workspace);
        assert!(app.active_dialog().is_none());
        assert_eq!(
            update(&mut app, Action::LeaveSignatureWorkspace),
            Some(Effect::CancelSignatureOperation)
        );

        let target = SignatureTarget {
            recipe: "busybox".into(),
            task: "do_fetch".into(),
        };
        let record = signature_record(
            "busybox",
            "do_fetch",
            "aaa",
            "/build/tmp/stamps/busybox/do_fetch.sigdata.aaa",
        );
        let _ = update(
            &mut app,
            Action::SignatureDumpLoaded {
                target,
                records: vec![record],
            },
        );
        assert_eq!(
            update(&mut app, Action::OpenSignatureProvider),
            Some(Effect::OpenInEditor(provider))
        );
        let _ = update(&mut app, Action::LeaveSignatureWorkspace);
        assert_eq!(app.screen, Screen::Recipes);
        assert_eq!(app.recipe_selection, 0);
    }

    #[test]
    fn signature_workspace_refresh_comparison_and_stale_results_remain_correlated() {
        let target = SignatureTarget {
            recipe: "busybox".into(),
            task: "do_compile".into(),
        };
        let left = signature_record(
            "busybox",
            "do_compile",
            "aaa",
            "/build/tmp/stamps/busybox/do_compile.sigdata.aaa",
        );
        let right = signature_record(
            "busybox",
            "do_compile",
            "bbb",
            "/build/tmp/stamps/busybox/do_compile.sigdata.bbb",
        );
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Signatures;
        let _ = update(&mut app, Action::BeginSignatureDump(target.clone()));
        let _ = update(
            &mut app,
            Action::SignatureDumpLoaded {
                target: target.clone(),
                records: vec![left.clone(), right.clone()],
            },
        );
        let _ = update(
            &mut app,
            Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Left),
        );
        let _ = update(&mut app, Action::SelectSignatureRecord { delta: 1 });
        let _ = update(
            &mut app,
            Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Right),
        );
        let request = SignatureComparisonRequest {
            left: left.identity,
            right: right.identity,
        };
        assert_eq!(
            update(&mut app, Action::BeginSignatureComparison),
            Some(Effect::CompareSignatures(request.clone()))
        );
        assert_eq!(
            update(&mut app, Action::RefreshSignatureDump),
            None,
            "refresh is inert while a comparison is loading"
        );
        let stale = SignatureComparisonRequest {
            left: request.right.clone(),
            right: request.left.clone(),
        };
        let _ = update(
            &mut app,
            Action::SignatureComparisonLoaded {
                request: stale,
                differences: Vec::new(),
            },
        );
        assert!(matches!(
            app.signature_comparison,
            SignatureComparisonState::Loading { .. }
        ));
        let _ = update(
            &mut app,
            Action::SignatureComparisonLoaded {
                request,
                differences: Vec::new(),
            },
        );
        assert!(matches!(
            app.signature_comparison,
            SignatureComparisonState::AvailableEmpty { .. }
        ));
        assert_eq!(
            update(&mut app, Action::RefreshSignatureDump),
            Some(Effect::GetSignatureDump(target))
        );
    }

    fn package_summary(name: &str, recipe: &str) -> PackageSummary {
        PackageSummary {
            identity: PackageIdentity::new(name),
            recipe: PackageField::Available(recipe.into()),
            provider: PackageField::Available(format!("/layers/meta/recipes/{recipe}.bb").into()),
            version: PackageField::Available("1.0".into()),
            installed_size_bytes: PackageField::Unavailable,
            license: PackageField::Unavailable,
            image_membership: PackageField::Available(Vec::new()),
        }
    }

    #[test]
    fn pkgdata_model_reducer_correlates_inventory_states_search_and_selection() {
        let mut app = App::new(10, 1_000);
        assert_eq!(
            update(&mut app, Action::BeginPackageInventory),
            Some(Effect::GetPackageInventory(PackageInventoryRequest {
                generation: 1
            }))
        );
        let request = PackageInventoryRequest { generation: 1 };
        let _ = update(
            &mut app,
            Action::PackageInventoryLoaded {
                request: PackageInventoryRequest { generation: 99 },
                packages: vec![package_summary("stale", "stale")],
            },
        );
        assert_eq!(
            app.package_inventory,
            PackageInventoryState::Loading { request }
        );
        let mut invalid_field = package_summary("libc6", "glibc");
        invalid_field.provider = PackageField::Available("relative.bb".into());
        let _ = update(
            &mut app,
            Action::PackageInventoryLoaded {
                request,
                packages: vec![
                    package_summary("busybox", "busybox"),
                    invalid_field,
                    package_summary("busybox", "zzz"),
                ],
            },
        );
        assert!(matches!(
            app.package_inventory,
            PackageInventoryState::Partial { .. }
        ));
        assert_eq!(app.package_selection, Some(PackageIdentity::new("busybox")));

        let _ = update(&mut app, Action::BeginPackageSearch);
        let _ = update(&mut app, Action::AppendPackageQuery('G'));
        let _ = update(&mut app, Action::AppendPackageQuery('L'));
        assert_eq!(app.package_selection, Some(PackageIdentity::new("libc6")));
        assert_eq!(app.filtered_packages().len(), 1);
        let _ = update(&mut app, Action::BackspacePackageQuery);
        let _ = update(&mut app, Action::FinishPackageSearch);
        assert!(!app.package_searching);

        let _ = update(&mut app, Action::BeginPackageInventory);
        let request = PackageInventoryRequest { generation: 2 };
        let _ = update(
            &mut app,
            Action::PackageInventoryLoaded {
                request,
                packages: vec![package_summary("libc6", "glibc")],
            },
        );
        assert_eq!(app.package_selection, Some(PackageIdentity::new("libc6")));

        let _ = update(&mut app, Action::BeginPackageInventory);
        let request = PackageInventoryRequest { generation: 3 };
        let _ = update(
            &mut app,
            Action::PackageInventoryLoaded {
                request,
                packages: Vec::new(),
            },
        );
        assert_eq!(
            app.package_inventory,
            PackageInventoryState::AvailableEmpty { request }
        );
        assert_eq!(app.package_selection, None);

        let _ = update(&mut app, Action::BeginPackageInventory);
        let request = PackageInventoryRequest { generation: 4 };
        let _ = update(
            &mut app,
            Action::PackageInventoryFailed {
                request,
                message: "pkgdata missing".into(),
            },
        );
        assert_eq!(
            app.package_inventory,
            PackageInventoryState::Failed {
                request,
                message: "pkgdata missing".into()
            }
        );
    }

    #[test]
    fn pkgdata_model_detail_states_and_dependency_navigation_are_exact() {
        let mut app = App::new(10, 1_000);
        let inventory_request = PackageInventoryRequest { generation: 1 };
        app.package_inventory = PackageInventoryState::Loading {
            request: inventory_request,
        };
        let _ = update(
            &mut app,
            Action::PackageInventoryLoaded {
                request: inventory_request,
                packages: vec![
                    package_summary("busybox", "busybox"),
                    package_summary("libc6", "glibc"),
                    package_summary("init", "init"),
                ],
            },
        );
        assert_eq!(
            update(&mut app, Action::BeginSelectedPackageDetail),
            Some(Effect::GetPackageDetail(PackageDetailRequest {
                identity: PackageIdentity::new("busybox"),
                generation: 1,
            }))
        );
        let request = PackageDetailRequest {
            identity: PackageIdentity::new("busybox"),
            generation: 1,
        };
        let detail = PackageDetail {
            identity: request.identity.clone(),
            files: PackageField::Available(vec!["/bin/busybox".into()]),
            runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
            reverse_dependencies: PackageField::Available(vec![PackageIdentity::new("init")]),
        };
        let _ = update(
            &mut app,
            Action::PackageDetailLoaded {
                request: PackageDetailRequest {
                    generation: 99,
                    ..request.clone()
                },
                detail: detail.clone(),
            },
        );
        assert!(matches!(
            app.selected_package_detail(),
            Some(PackageDetailState::Loading { .. })
        ));
        let _ = update(
            &mut app,
            Action::PackageDetailPartial {
                request: request.clone(),
                detail,
                limitations: vec!["license unavailable".into()],
            },
        );
        assert!(matches!(
            app.selected_package_detail(),
            Some(PackageDetailState::Partial { .. })
        ));

        let _ = update(
            &mut app,
            Action::OpenPackageDependency {
                identity: PackageIdentity::new("libc6"),
                reverse: false,
            },
        );
        assert_eq!(app.package_selection, Some(PackageIdentity::new("libc6")));
        app.package_selection = Some(PackageIdentity::new("busybox"));
        let _ = update(
            &mut app,
            Action::OpenPackageDependency {
                identity: PackageIdentity::new("init"),
                reverse: true,
            },
        );
        assert_eq!(app.package_selection, Some(PackageIdentity::new("init")));
        app.package_selection = Some(PackageIdentity::new("busybox"));
        let _ = update(
            &mut app,
            Action::OpenPackageDependency {
                identity: PackageIdentity::new("not-present"),
                reverse: false,
            },
        );
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("not in the current typed detail"))
        );

        app.package_selection = Some(PackageIdentity::new("libc6"));
        let effect = update(&mut app, Action::BeginSelectedPackageDetail).unwrap();
        let Effect::GetPackageDetail(empty_request) = effect else {
            panic!("expected package detail effect");
        };
        let empty = PackageDetail {
            identity: empty_request.identity.clone(),
            files: PackageField::Available(Vec::new()),
            runtime_dependencies: PackageField::Available(Vec::new()),
            reverse_dependencies: PackageField::Available(Vec::new()),
        };
        let _ = update(
            &mut app,
            Action::PackageDetailLoaded {
                request: empty_request.clone(),
                detail: empty,
            },
        );
        assert_eq!(
            app.package_details.get(&empty_request.identity),
            Some(&PackageDetailState::AvailableEmpty {
                request: empty_request.clone()
            })
        );

        app.package_selection = Some(PackageIdentity::new("init"));
        let Effect::GetPackageDetail(failed_request) =
            update(&mut app, Action::BeginSelectedPackageDetail).unwrap()
        else {
            panic!("expected package detail effect");
        };
        let _ = update(
            &mut app,
            Action::PackageDetailFailed {
                request: failed_request.clone(),
                message: "tool failed".into(),
            },
        );
        assert_eq!(
            app.package_details.get(&failed_request.identity),
            Some(&PackageDetailState::Failed {
                request: failed_request,
                message: "tool failed".into()
            })
        );
    }

    #[test]
    fn pkgdata_workspace_routes_navigation_refresh_detail_and_contextual_actions() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes.push(Recipe {
            name: "busybox".into(),
            file: Some("/layers/meta/recipes-core/busybox.bb".into()),
            ..Recipe::default()
        });
        assert_eq!(
            update(&mut app, Action::Open(Screen::Packages)),
            Some(Effect::GetPackageInventory(PackageInventoryRequest {
                generation: 1
            }))
        );
        assert_eq!(app.screen, Screen::Packages);
        assert_eq!(NAVIGATOR_SCREENS[app.navigator_selection], Screen::Packages);
        assert_eq!(
            update(&mut app, Action::CancelPackageOperation),
            Some(Effect::CancelPackageOperation)
        );
        let request = PackageInventoryRequest { generation: 1 };
        let _ = update(
            &mut app,
            Action::PackageInventoryLoaded {
                request,
                packages: vec![
                    package_summary("busybox", "busybox"),
                    package_summary("init", "init"),
                    package_summary("libc6", "glibc"),
                ],
            },
        );
        assert_eq!(
            update(&mut app, Action::BeginSelectedPackageDetail),
            Some(Effect::GetPackageDetail(PackageDetailRequest {
                identity: PackageIdentity::new("busybox"),
                generation: 2,
            }))
        );
        let detail_request = PackageDetailRequest {
            identity: PackageIdentity::new("busybox"),
            generation: 2,
        };
        let _ = update(
            &mut app,
            Action::PackageDetailLoaded {
                request: detail_request.clone(),
                detail: PackageDetail {
                    identity: detail_request.identity,
                    files: PackageField::Available(vec!["/bin/busybox".into()]),
                    runtime_dependencies: PackageField::Available(vec![PackageIdentity::new(
                        "libc6",
                    )]),
                    reverse_dependencies: PackageField::Available(vec![PackageIdentity::new(
                        "init",
                    )]),
                },
            },
        );
        assert_eq!(
            app.selected_package_dependency(),
            Some(&PackageIdentity::new("libc6"))
        );
        let _ = update(&mut app, Action::TogglePackageDependencyKind);
        assert_eq!(
            app.selected_package_dependency(),
            Some(&PackageIdentity::new("init"))
        );
        assert_eq!(
            update(&mut app, Action::OpenSelectedPackageDependency),
            Some(Effect::GetPackageDetail(PackageDetailRequest {
                identity: PackageIdentity::new("init"),
                generation: 3,
            }))
        );
        assert_eq!(app.package_selection, Some(PackageIdentity::new("init")));
        let _ = update(&mut app, Action::BackPackageNavigation);
        assert_eq!(app.package_selection, Some(PackageIdentity::new("busybox")));

        assert_eq!(
            update(&mut app, Action::OpenSelectedPackageProvider),
            Some(Effect::OpenInEditor(
                "/layers/meta/recipes/busybox.bb".into()
            ))
        );
        let _ = update(&mut app, Action::OpenSelectedPackageRecipe);
        assert_eq!(app.screen, Screen::Recipes);
        assert_eq!(app.recipe_selection, 0);

        app.package_details.clear();
        app.screen = Screen::Packages;
        assert_eq!(
            update(&mut app, Action::RefreshPackageInventory),
            Some(Effect::GetPackageInventory(PackageInventoryRequest {
                generation: 4
            }))
        );
    }

    #[test]
    fn image_artifact_model_correlates_states_search_and_stable_selection() {
        let make_artifact = |image: &str, suffix: &str, kind| ImageArtifact {
            identity: ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: image.into(),
                path: format!("/build/tmp/deploy/images/qemux86-64/{image}.{suffix}").into(),
            },
            kind,
            size_bytes: ImageArtifactField::Available(4_096),
            modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
            checksums: ImageArtifactField::Available(Vec::new()),
            manifests: ImageArtifactField::Available(Vec::new()),
            licenses: ImageArtifactField::Unavailable,
            spdx: ImageArtifactField::Unavailable,
            wic_files: ImageArtifactField::Available(Vec::new()),
        };
        let inventory = |artifacts| ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: ImageArtifactField::Available(
                "/build/tmp/deploy/images/qemux86-64".into(),
            ),
            artifacts,
        };

        let mut app = App::new(20, 20_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let request = ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        };
        assert_eq!(
            update(&mut app, Action::BeginImageArtifactInventory),
            Some(Effect::GetImageArtifacts(request.clone()))
        );
        let minimal = make_artifact(
            "core-image-minimal",
            "rootfs.ext4",
            ImageArtifactKind::RootFilesystem,
        );
        let sato = make_artifact("core-image-sato", "wic", ImageArtifactKind::Wic);
        let _ = update(
            &mut app,
            Action::ImageArtifactInventoryLoaded {
                request: request.clone(),
                inventory: inventory(vec![sato.clone(), minimal.clone()]),
            },
        );
        assert!(matches!(
            app.image_artifacts,
            ImageArtifactInventoryState::Available { .. }
        ));
        assert_eq!(app.image_artifact_selection, Some(minimal.identity.clone()));
        let _ = update(&mut app, Action::SelectImageArtifact { delta: 1 });
        assert_eq!(app.image_artifact_selection, Some(sato.identity.clone()));

        assert_eq!(
            update(&mut app, Action::RefreshImageArtifactInventory),
            Some(Effect::GetImageArtifacts(ImageArtifactRequest {
                generation: 2,
                machine: "qemux86-64".into(),
            }))
        );
        let _ = update(
            &mut app,
            Action::ImageArtifactInventoryLoaded {
                request,
                inventory: inventory(vec![minimal.clone()]),
            },
        );
        assert!(matches!(
            app.image_artifacts,
            ImageArtifactInventoryState::Loading { .. }
        ));
        let request = ImageArtifactRequest {
            generation: 2,
            machine: "qemux86-64".into(),
        };
        let _ = update(
            &mut app,
            Action::ImageArtifactInventoryPartial {
                request: request.clone(),
                inventory: inventory(vec![minimal.clone(), sato.clone()]),
                limitations: vec!["checksum metadata unavailable".into()],
            },
        );
        assert!(matches!(
            app.image_artifacts,
            ImageArtifactInventoryState::Partial { .. }
        ));
        assert_eq!(app.image_artifact_selection, Some(sato.identity.clone()));

        let _ = update(&mut app, Action::BeginImageArtifactSearch);
        let _ = update(&mut app, Action::AppendImageArtifactQuery('m'));
        let _ = update(&mut app, Action::AppendImageArtifactQuery('i'));
        let _ = update(&mut app, Action::AppendImageArtifactQuery('n'));
        assert_eq!(app.filtered_image_artifacts(), vec![&minimal]);
        assert_eq!(app.image_artifact_selection, Some(minimal.identity.clone()));
        let _ = update(&mut app, Action::FinishImageArtifactSearch);

        app.image_artifact_query.clear();
        let failed_request = ImageArtifactRequest {
            generation: 3,
            machine: "qemux86-64".into(),
        };
        let _ = update(&mut app, Action::RefreshImageArtifactInventory);
        let _ = update(
            &mut app,
            Action::ImageArtifactInventoryFailed {
                request: failed_request.clone(),
                message: "deploy directory is unavailable".into(),
            },
        );
        assert!(matches!(
            app.image_artifacts,
            ImageArtifactInventoryState::Failed { ref request, .. } if request == &failed_request
        ));

        let empty_request = ImageArtifactRequest {
            generation: 4,
            machine: "qemux86-64".into(),
        };
        let _ = update(&mut app, Action::RefreshImageArtifactInventory);
        let _ = update(
            &mut app,
            Action::ImageArtifactInventoryLoaded {
                request: empty_request,
                inventory: inventory(Vec::new()),
            },
        );
        assert!(matches!(
            app.image_artifacts,
            ImageArtifactInventoryState::AvailableEmpty { .. }
        ));
        assert_eq!(app.image_artifact_selection, None);

        let invalid_request = ImageArtifactRequest {
            generation: 5,
            machine: "qemux86-64".into(),
        };
        let _ = update(&mut app, Action::RefreshImageArtifactInventory);
        let _ = update(
            &mut app,
            Action::ImageArtifactInventoryLoaded {
                request: invalid_request,
                inventory: ImageArtifactInventory {
                    machine: "qemuarm64".into(),
                    deploy_directory: ImageArtifactField::Unavailable,
                    artifacts: Vec::new(),
                },
            },
        );
        assert!(matches!(
            app.image_artifacts,
            ImageArtifactInventoryState::Failed { .. }
        ));
    }

    #[test]
    fn images_workspace_preserves_build_and_routes_exact_typed_paths() {
        let mut app = App::new(20, 20_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let request = ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        };
        assert_eq!(
            update(&mut app, Action::Open(Screen::Images)),
            Some(Effect::GetImageArtifacts(request.clone()))
        );
        let artifact_path =
            PathBuf::from("/build/tmp/deploy/images/qemux86-64/core-image-minimal.wic");
        let manifest_path =
            PathBuf::from("/build/tmp/deploy/images/qemux86-64/core-image-minimal.manifest");
        let artifact = ImageArtifact {
            identity: ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: "core-image-minimal".into(),
                path: artifact_path.clone(),
            },
            kind: ImageArtifactKind::Wic,
            size_bytes: ImageArtifactField::Available(42),
            modified_unix_seconds: ImageArtifactField::Available(10),
            checksums: ImageArtifactField::Unavailable,
            manifests: ImageArtifactField::Available(vec![manifest_path.clone()]),
            licenses: ImageArtifactField::Unavailable,
            spdx: ImageArtifactField::Unavailable,
            wic_files: ImageArtifactField::Available(vec![artifact_path.clone()]),
        };
        let _ = update(
            &mut app,
            Action::ImageArtifactInventoryLoaded {
                request,
                inventory: ImageArtifactInventory {
                    machine: "qemux86-64".into(),
                    deploy_directory: ImageArtifactField::Available(
                        "/build/tmp/deploy/images/qemux86-64".into(),
                    ),
                    artifacts: vec![artifact],
                },
            },
        );
        assert_eq!(
            update(&mut app, Action::OpenSelectedImageArtifact),
            Some(Effect::OpenInEditor(artifact_path.clone()))
        );
        assert_eq!(
            update(
                &mut app,
                Action::OpenSelectedImageArtifactAssociation(ImageArtifactAssociation::Manifest)
            ),
            Some(Effect::OpenInEditor(manifest_path))
        );
        let _ = update(&mut app, Action::BeginSelectedImageArtifactBuild);
        assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::RecipeTaskConfirmation(BuildRequest { targets, .. }))
                if targets == &vec!["core-image-minimal".to_owned()]
        ));
    }

    fn qemu_model_artifact() -> ImageArtifact {
        ImageArtifact {
            identity: ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: "core-image-minimal".into(),
                path: "/build/tmp/deploy/images/qemux86-64/core-image-minimal.wic".into(),
            },
            kind: ImageArtifactKind::Wic,
            size_bytes: ImageArtifactField::Available(42),
            modified_unix_seconds: ImageArtifactField::Available(10),
            checksums: ImageArtifactField::Unavailable,
            manifests: ImageArtifactField::Unavailable,
            licenses: ImageArtifactField::Unavailable,
            spdx: ImageArtifactField::Unavailable,
            wic_files: ImageArtifactField::Unavailable,
        }
    }

    fn qemu_model_app() -> App {
        let mut app = App::new(20, 20_000);
        let artifact = qemu_model_artifact();
        let request = ImageArtifactRequest {
            generation: 1,
            machine: artifact.identity.machine.clone(),
        };
        app.image_artifacts = ImageArtifactInventoryState::Available {
            request,
            inventory: ImageArtifactInventory {
                machine: artifact.identity.machine.clone(),
                deploy_directory: ImageArtifactField::Available(
                    "/build/tmp/deploy/images/qemux86-64".into(),
                ),
                artifacts: vec![artifact.clone()],
            },
        };
        app.image_artifact_selection = Some(artifact.identity.clone());
        app.qemu_capability = QemuCapability::Available {
            executable: "/opt/poky/scripts/runqemu".into(),
            compatible_images: vec![artifact.identity],
        };
        app
    }

    #[test]
    fn qemu_model_validates_exact_launch_identity_paths_and_options() {
        let artifact = qemu_model_artifact();
        let capability = QemuCapability::Available {
            executable: "/opt/poky/scripts/runqemu".into(),
            compatible_images: vec![artifact.identity.clone()],
        };
        let draft = QemuLaunchDraft::for_artifact(artifact.identity.clone(), artifact.kind);
        let first = draft.preview(&capability).expect("valid preview");
        let second = draft.preview(&capability).expect("deterministic preview");
        assert_eq!(first, second);
        assert_eq!(first.request.memory_mib, 1024);

        let mut invalid = draft.clone();
        invalid.machine = "other-machine".into();
        assert_eq!(
            invalid.preview(&capability),
            Err("runqemu machine and image identities must match")
        );
        invalid = draft.clone();
        invalid.rootfs = "relative/rootfs.ext4".into();
        assert!(invalid.preview(&capability).is_err());
        invalid = draft.clone();
        invalid.memory_mib = (MAX_QEMU_MEMORY_MIB + 1).to_string();
        assert!(invalid.preview(&capability).is_err());
        invalid = draft.clone();
        invalid.extra_arguments = "-- -display none".into();
        assert!(invalid.preview(&capability).is_err());
        invalid = draft;
        invalid.artifact_kind = ImageArtifactKind::Manifest;
        assert!(invalid.preview(&capability).is_err());
    }

    #[test]
    fn qemu_model_reducer_previews_confirms_and_bounds_session_output() {
        let mut app = qemu_model_app();
        let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
        assert!(matches!(app.active_dialog(), Some(Dialog::QemuLaunch(_))));
        assert_eq!(app.focus, FocusTarget::Dialog);
        let _ = update(&mut app, Action::PreviewQemuLaunch);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::QemuLaunchConfirmation(_))
        ));
        let effect = update(&mut app, Action::ConfirmQemuLaunch);
        let Some(Effect::StartQemuSession { id, request }) = effect else {
            panic!("expected typed runqemu start effect");
        };
        assert_eq!(request.image, qemu_model_artifact().identity);
        let session = app.qemu_session(id).expect("session");
        let job_id = session.background_job_id;
        assert_eq!(
            app.background_jobs.get(job_id).map(|job| job.status),
            Some(BackgroundJobStatus::Queued)
        );

        let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
        assert_eq!(
            app.notification.as_deref(),
            Some("A managed runqemu session is already active.")
        );
        let _ = update(
            &mut app,
            Action::QemuSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::QemuSessionRunning { id });
        for index in 0..600 {
            let _ = update(
                &mut app,
                Action::AppendQemuSessionOutput {
                    id,
                    stream: if index % 2 == 0 {
                        QemuOutputStream::Stdout
                    } else {
                        QemuOutputStream::Stderr
                    },
                    line: format!("line {index}"),
                    truncated: false,
                    timestamp: SystemTime::UNIX_EPOCH,
                },
            );
        }
        let job = app.background_jobs.get(job_id).expect("job");
        assert_eq!(job.status, BackgroundJobStatus::Running);
        assert_eq!(job.output.len(), MAX_BACKGROUND_JOB_OUTPUT_ENTRIES);
        assert_eq!(job.dropped_output_entries, 88);
        assert!(
            job.output
                .iter()
                .any(|entry| entry.source == BackgroundJobOutputSource::Stderr)
        );
    }

    #[test]
    fn qemu_model_requires_cancellation_confirmation_and_rejects_stale_events() {
        let mut app = qemu_model_app();
        let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
        let _ = update(&mut app, Action::PreviewQemuLaunch);
        let Some(Effect::StartQemuSession { id, .. }) = update(&mut app, Action::ConfirmQemuLaunch)
        else {
            panic!("expected start");
        };
        let _ = update(
            &mut app,
            Action::QemuSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::QemuSessionRunning { id });
        let _ = update(&mut app, Action::BeginQemuSessionCancellation { id });
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::QemuCancellationConfirmation(candidate)) if *candidate == id
        ));
        assert_eq!(
            update(&mut app, Action::ConfirmQemuSessionCancellation),
            Some(Effect::CancelQemuSession(id))
        );
        let job_id = app.qemu_session(id).expect("session").background_job_id;
        assert_eq!(
            app.background_jobs.get(job_id).map(|job| job.status),
            Some(BackgroundJobStatus::Cancelling)
        );
        let _ = update(
            &mut app,
            Action::RejectQemuSessionCancellation {
                id,
                message: "signal failed".into(),
            },
        );
        assert_eq!(
            app.background_jobs.get(job_id).map(|job| job.status),
            Some(BackgroundJobStatus::Running)
        );
        let _ = update(&mut app, Action::BeginQemuSessionCancellation { id });
        let _ = update(&mut app, Action::ConfirmQemuSessionCancellation);
        let _ = update(
            &mut app,
            Action::CancelQemuSession {
                id,
                exit_code: Some(130),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(
            app.background_jobs.get(job_id).map(|job| job.status),
            Some(BackgroundJobStatus::Cancelled)
        );
        assert_eq!(
            app.qemu_session(id).and_then(|session| session.exit_code),
            Some(130)
        );

        let ignored = app.background_jobs.ignored_transitions;
        let _ = update(
            &mut app,
            Action::QemuSessionRunning {
                id: QemuSessionId(99_999),
            },
        );
        assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
    }

    #[test]
    fn qemu_workspace_dialog_fields_are_bounded_modal_and_validation_aware() {
        let mut app = qemu_model_app();
        let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
        assert_eq!(app.focus, FocusTarget::Dialog);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::QemuLaunch(QemuLaunchDialog {
                selected_field: QemuLaunchField::Machine,
                editing: false,
                ..
            }))
        ));
        let _ = update(&mut app, Action::ActivateQemuLaunchField);
        assert_eq!(
            app.notification.as_deref(),
            Some("Image and machine identity are read-only.")
        );
        let _ = update(&mut app, Action::SelectQemuLaunchField { delta: 2 });
        let _ = update(&mut app, Action::ActivateQemuLaunchField);
        for _ in 0..(MAX_QEMU_PATH_INPUT_BYTES + 10) {
            let _ = update(&mut app, Action::AppendQemuLaunchField('x'));
        }
        let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog() else {
            panic!("launch dialog");
        };
        assert_eq!(dialog.draft.kernel.len(), MAX_QEMU_PATH_INPUT_BYTES);
        assert!(dialog.editing);
        let _ = update(&mut app, Action::FinishQemuLaunchFieldEdit);
        let _ = update(&mut app, Action::SelectQemuLaunchField { delta: 2 });
        let _ = update(&mut app, Action::CycleQemuLaunchChoice { backwards: false });
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::QemuLaunch(QemuLaunchDialog {
                draft: QemuLaunchDraft {
                    networking: QemuNetworkingMode::Tap,
                    ..
                },
                ..
            }))
        ));
        let _ = update(&mut app, Action::PreviewQemuLaunch);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::QemuLaunch(QemuLaunchDialog {
                validation_error: Some(message),
                ..
            })) if message.contains("paths must be normalized")
        ));
        let _ = update(&mut app, Action::CancelQemuLaunch);
        assert!(app.active_dialog().is_none());
        assert_eq!(app.focus, FocusTarget::Workspace);
    }

    #[test]
    fn qemu_workspace_availability_reasons_are_stable_and_cancellation_is_modal() {
        let mut app = qemu_model_app();
        let mut unsupported = qemu_model_app();
        if let ImageArtifactInventoryState::Available { inventory, .. } =
            &mut unsupported.image_artifacts
        {
            inventory.artifacts[0].kind = ImageArtifactKind::Manifest;
        }
        assert_eq!(
            unsupported.qemu_launch_unavailable_reason().as_deref(),
            Some("runqemu requires a root filesystem or Wic artifact.")
        );
        app.qemu_capability = QemuCapability::NotInspected;
        assert_eq!(
            app.qemu_launch_unavailable_reason().as_deref(),
            Some("runqemu capability has not been inspected.")
        );
        app.qemu_capability = QemuCapability::MissingTool;
        assert_eq!(
            app.qemu_launch_unavailable_reason().as_deref(),
            Some("runqemu is not available.")
        );
        app.qemu_capability = QemuCapability::MissingCompatibleImage;
        assert_eq!(
            app.qemu_launch_unavailable_reason().as_deref(),
            Some("No compatible deployed runqemu image is available.")
        );
        app.qemu_capability = QemuCapability::Failed {
            message: "inspection denied".into(),
        };
        assert_eq!(
            app.qemu_launch_unavailable_reason().as_deref(),
            Some("runqemu capability inspection failed: inspection denied")
        );
        app.qemu_capability = QemuCapability::Available {
            executable: "/opt/poky/scripts/runqemu".into(),
            compatible_images: Vec::new(),
        };
        assert_eq!(
            app.qemu_launch_unavailable_reason().as_deref(),
            Some("The selected artifact is not in the inspected runqemu capability.")
        );
        app.qemu_capability = QemuCapability::Available {
            executable: "/opt/poky/scripts/runqemu".into(),
            compatible_images: vec![qemu_model_artifact().identity],
        };
        let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
        let _ = update(&mut app, Action::PreviewQemuLaunch);
        let Some(Effect::StartQemuSession { id, .. }) = update(&mut app, Action::ConfirmQemuLaunch)
        else {
            panic!("start effect");
        };
        let _ = update(
            &mut app,
            Action::QemuSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::QemuSessionRunning { id });
        let _ = update(&mut app, Action::BeginActiveQemuSessionCancellation);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::QemuCancellationConfirmation(candidate)) if *candidate == id
        ));
        let _ = update(&mut app, Action::CancelQemuSessionCancellation);
        assert!(app.active_dialog().is_none());
        assert_eq!(
            app.background_jobs
                .get(app.qemu_session(id).expect("session").background_job_id)
                .map(|job| job.status),
            Some(BackgroundJobStatus::Running)
        );
    }

    fn wic_model_capability() -> WicCapability {
        WicCapability::Available {
            executable: "/opt/poky/scripts/wic".into(),
            kickstarts: vec![WicKickstart {
                identity: WicKickstartIdentity {
                    name: "directdisk".into(),
                    path: Some("/layers/meta/wic/directdisk.wks".into()),
                },
                source: "part / --source rootfs --fstype=ext4 --size=64".into(),
                partitions: vec![WicPartitionSummary {
                    mount_point: Some("/".into()),
                    filesystem: Some("ext4".into()),
                    source_plugin: Some("rootfs".into()),
                    size_mib: Some(64),
                    alignment_kib: None,
                }],
                limitations: Vec::new(),
            }],
            image_targets: vec!["core-image-minimal".into()],
        }
    }

    fn wic_model_device(path: &str, major_minor: &str) -> WicDevice {
        WicDevice {
            identity: WicDeviceIdentity {
                path: path.into(),
                major_minor: major_minor.into(),
                size_bytes: 2048,
                model: Some("test".into()),
                serial: Some(format!("serial-{major_minor}")),
                transport: Some("usb".into()),
            },
            removable: true,
            writable: true,
            read_only: false,
            descendant_mounts: Vec::new(),
            unavailable_reason: None,
        }
    }

    #[test]
    fn wic_model_reducer_correlates_creation_inventory_and_lifecycle() {
        let mut app = App::new(20, 20_000);
        app.wic_capability = wic_model_capability();
        let draft = WicCreateDraft {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            kickstart: WicKickstartIdentity {
                name: "directdisk".into(),
                path: Some("/layers/meta/wic/directdisk.wks".into()),
            },
            output_directory: "/build/wic-output".into(),
            generate_bmap: true,
            compression: WicCompression::None,
        };
        let preview = draft.preview(&app.wic_capability).unwrap();
        let Some(Effect::StartWicSession { id, operation }) =
            update(&mut app, Action::StartConfirmedWicCreate(preview))
        else {
            panic!("Wic start effect");
        };
        assert!(matches!(operation, WicOperation::Create(_)));
        let background_job_id = app.wic_session(id).unwrap().background_job_id;
        assert_eq!(
            background_job_id,
            BackgroundJobId(WIC_BACKGROUND_JOB_NAMESPACE | id.0)
        );
        assert_ne!(
            background_job_id,
            qemu_background_job_id(QemuSessionId(id.0))
        );
        let _ = update(
            &mut app,
            Action::WicSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::WicSessionRunning { id });
        let _ = update(
            &mut app,
            Action::AppendWicSessionOutput {
                id,
                stream: WicOutputStream::Stdout,
                line: "creating".into(),
                truncated: false,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        );
        let output = WicOutput {
            identity: WicOutputIdentity {
                path: "/build/wic-output/image.wic".into(),
                size_bytes: 1024,
                modified_unix_seconds: 1,
            },
            kind: WicOutputKind::Wic,
        };
        let _ = update(
            &mut app,
            Action::CompleteWicSession {
                id,
                exit_code: 0,
                outputs: vec![output.clone()],
                limitations: vec!["dynamic partition size".into()],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(
            app.background_jobs
                .get(background_job_id)
                .map(|job| job.status),
            Some(BackgroundJobStatus::Succeeded)
        );
        assert!(matches!(
            &app.wic_outputs,
            WicOutputInventoryState::Partial { outputs, .. } if outputs == &vec![output]
        ));

        let ignored = app.background_jobs.ignored_transitions;
        let _ = update(
            &mut app,
            Action::WicSessionRunning {
                id: WicSessionId(99_999),
            },
        );
        assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
    }

    #[test]
    fn wic_device_write_requires_exact_phrase_and_cancellation_warning() {
        let mut app = App::new(20, 20_000);
        app.wic_capability = wic_model_capability();
        let image = WicOutputIdentity {
            path: "/build/wic-output/image.wic".into(),
            size_bytes: 1024,
            modified_unix_seconds: 1,
        };
        app.wic_outputs = WicOutputInventoryState::Available {
            request: WicOutputInventoryRequest {
                generation: 1,
                output_directory: "/build/wic-output".into(),
            },
            outputs: vec![WicOutput {
                identity: image.clone(),
                kind: WicOutputKind::Wic,
            }],
        };
        app.wic_output_selection = Some(image.clone());
        let Some(Effect::GetWicDevices(request)) =
            update(&mut app, Action::BeginSelectedWicDeviceWrite)
        else {
            panic!("Wic device discovery effect");
        };
        assert_eq!(request.image, image);
        assert_eq!(app.focus, FocusTarget::Dialog);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::WicDevicePicker(dialog)) if dialog.request == request
        ));
        let ignored = app.background_jobs.ignored_transitions;
        let mut stale = request.clone();
        stale.generation += 1;
        let _ = update(
            &mut app,
            Action::WicDeviceInventoryLoaded {
                request: stale,
                devices: vec![wic_model_device("/dev/sdy", "8:239")],
                limitations: Vec::new(),
            },
        );
        assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
        let first = wic_model_device("/dev/sdy", "8:239");
        let selected = wic_model_device("/dev/sdz", "8:240");
        let _ = update(
            &mut app,
            Action::WicDeviceInventoryLoaded {
                request: request.clone(),
                devices: vec![first, selected.clone()],
                limitations: vec!["system device excluded".into()],
            },
        );
        let _ = update(&mut app, Action::SelectWicDevice { delta: 1 });
        assert_eq!(app.wic_device_selection, Some(selected.identity.clone()));
        let _ = update(&mut app, Action::CancelWicDevicePicker);
        let Some(Effect::GetWicDevices(refreshed_request)) =
            update(&mut app, Action::BeginSelectedWicDeviceWrite)
        else {
            panic!("refreshed Wic device discovery effect");
        };
        assert!(refreshed_request.generation > request.generation);
        let _ = update(
            &mut app,
            Action::WicDeviceInventoryLoaded {
                request: refreshed_request,
                devices: vec![selected.clone(), wic_model_device("/dev/sdy", "8:239")],
                limitations: Vec::new(),
            },
        );
        assert_eq!(app.wic_device_selection, Some(selected.identity.clone()));
        let _ = update(&mut app, Action::ConfirmWicDeviceSelection);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::WicWritePhrase(dialog)) if dialog.device == selected.identity
        ));
        for character in "WRITE /dev/sdy".chars() {
            let _ = update(&mut app, Action::AppendWicWritePhrase(character));
        }
        assert!(update(&mut app, Action::PreviewWicDeviceWrite).is_none());
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::WicWritePhrase(WicWritePhraseDialog {
                validation_error: Some(_),
                ..
            }))
        ));
        if let Some(Dialog::WicWritePhrase(dialog)) = app.active_dialog_mut() {
            dialog.input = "WRITE /dev/sdz".into();
        }
        let _ = update(&mut app, Action::PreviewWicDeviceWrite);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::WicWriteConfirmation(_))
        ));
        app.wic_capability = WicCapability::MissingTool;
        assert!(update(&mut app, Action::ConfirmWicDeviceWrite).is_none());
        assert!(app.active_wic_session().is_none());
        app.wic_capability = WicCapability::MissingKickstarts {
            executable: "/opt/poky/scripts/wic".into(),
        };
        let Some(Effect::StartWicSession { id, operation }) =
            update(&mut app, Action::ConfirmWicDeviceWrite)
        else {
            panic!("Wic write start effect");
        };
        assert!(matches!(operation, WicOperation::Write(_)));
        assert!(app.active_dialog().is_none());
        let _ = update(
            &mut app,
            Action::WicSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::WicSessionRunning { id });
        let _ = update(&mut app, Action::BeginActiveWicSessionCancellation);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::WicCancellationConfirmation {
                id: candidate,
                incomplete_device_warning: true,
            }) if *candidate == id
        ));
        assert!(
            update(
                &mut app,
                Action::ConfirmWicSessionCancellation {
                    id,
                    acknowledge_incomplete_device: false,
                },
            )
            .is_none()
        );
        assert_eq!(
            app.background_jobs
                .get(app.wic_session(id).unwrap().background_job_id)
                .map(|job| job.status),
            Some(BackgroundJobStatus::Running)
        );
        assert_eq!(
            update(
                &mut app,
                Action::ConfirmWicSessionCancellation {
                    id,
                    acknowledge_incomplete_device: true,
                },
            ),
            Some(Effect::CancelWicSession(id))
        );
        let _ = update(
            &mut app,
            Action::CancelWicSession {
                id,
                exit_code: Some(130),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let job = app
            .background_jobs
            .get(app.wic_session(id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(job.status, BackgroundJobStatus::Cancelled);
        assert!(
            job.error
                .as_ref()
                .and_then(|error| error.detail.as_deref())
                .is_some_and(|detail| detail.contains("incomplete"))
        );
    }

    #[test]
    fn wic_device_write_discovers_only_exact_images_and_keeps_empty_failure_typed() {
        let mut app = qemu_model_app();
        app.wic_capability = wic_model_capability();
        let Some(Effect::GetWicDevices(request)) =
            update(&mut app, Action::BeginSelectedWicDeviceWrite)
        else {
            panic!("deployed Wic discovery");
        };
        assert!(request.image.path.ends_with("core-image-minimal.wic"));
        let _ = update(
            &mut app,
            Action::WicDeviceInventoryLoaded {
                request: request.clone(),
                devices: Vec::new(),
                limitations: Vec::new(),
            },
        );
        assert!(matches!(
            app.wic_devices,
            WicDeviceInventoryState::Available {
                ref devices,
                ..
            } if devices.is_empty()
        ));
        assert!(app.wic_device_selection.is_none());
        assert!(update(&mut app, Action::ConfirmWicDeviceSelection).is_none());
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::WicDevicePicker(_))
        ));
        let _ = update(&mut app, Action::CancelWicDevicePicker);
        let Some(Effect::GetWicDevices(second_request)) =
            update(&mut app, Action::BeginSelectedWicDeviceWrite)
        else {
            panic!("second discovery");
        };
        assert!(second_request.generation > request.generation);
        let _ = update(
            &mut app,
            Action::WicDeviceInventoryFailed {
                request: second_request.clone(),
                message: "root identity ambiguous".into(),
            },
        );
        assert!(matches!(
            &app.wic_devices,
            WicDeviceInventoryState::Failed { request, message }
                if request == &second_request && message == "root identity ambiguous"
        ));

        let mut compressed = qemu_model_app();
        compressed.wic_capability = wic_model_capability();
        if let ImageArtifactInventoryState::Available { inventory, .. } =
            &mut compressed.image_artifacts
        {
            inventory.artifacts[0].identity.path =
                "/build/tmp/deploy/images/qemux86-64/core-image-minimal.wic.gz".into();
            compressed.image_artifact_selection = Some(inventory.artifacts[0].identity.clone());
        }
        assert!(update(&mut compressed, Action::BeginSelectedWicDeviceWrite).is_none());

        let mut direct = qemu_model_app();
        direct.wic_capability = wic_model_capability();
        if let ImageArtifactInventoryState::Available { inventory, .. } =
            &mut direct.image_artifacts
        {
            inventory.artifacts[0].identity.path =
                "/build/tmp/deploy/images/qemux86-64/core-image-minimal.direct".into();
            direct.image_artifact_selection = Some(inventory.artifacts[0].identity.clone());
        }
        assert!(matches!(
            update(&mut direct, Action::BeginSelectedWicDeviceWrite),
            Some(Effect::GetWicDevices(WicDeviceInventoryRequest {
                image: WicOutputIdentity { ref path, .. },
                ..
            })) if path.ends_with("core-image-minimal.direct")
        ));
    }

    #[test]
    fn wic_workspace_dialog_is_bounded_modal_and_stale_safe() {
        let mut app = qemu_model_app();
        app.wic_capability = wic_model_capability();
        if let WicCapability::Available { kickstarts, .. } = &mut app.wic_capability {
            kickstarts.push(WicKickstart {
                identity: WicKickstartIdentity {
                    name: "configured".into(),
                    path: Some("/layers/custom/configured.wks".into()),
                },
                source: "part /boot --source=bootimg-partition".into(),
                partitions: Vec::new(),
                limitations: Vec::new(),
            });
        }
        app.workspace
            .variables
            .insert("WKS_FILE".into(), "/layers/custom/configured.wks".into());
        let _ = update(&mut app, Action::BeginSelectedWicCreate);
        assert_eq!(app.focus, FocusTarget::Dialog);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::WicCreateTomlEditor { editor, .. })
                if !editor.editing
                    && editor.text.contains("kickstart = \"configured\"")
                    && editor.selected_text() == Some("/build/tmp/deploy/images/qemux86-64")
        ));
        if let Some(Dialog::WicCreateTomlEditor { editor, .. }) = app.active_dialog_mut() {
            editor.text = "machine = \"qemux86-64\"\nimage = \"core-image-minimal\"\nkickstart = \"configured\"\noutput_directory = \"relative/output\"\ngenerate_bmap = true\ncompression = \"none\"\n".into();
            editor.cursor = editor.text.len();
        }
        let _ = update(&mut app, Action::PreviewWicCreate);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::WicCreateTomlEditor {
                validation_error: Some(_),
                ..
            })
        ));
        let _ = update(&mut app, Action::CancelWicCreate);
        assert!(app.active_dialog().is_none());
        assert_eq!(app.focus, FocusTarget::Workspace);

        let _ = update(&mut app, Action::BeginSelectedWicCreate);
        let _ = update(&mut app, Action::PreviewWicCreate);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::WicCreateConfirmation(_))
        ));
        app.wic_capability = WicCapability::MissingTool;
        assert!(update(&mut app, Action::ConfirmWicCreate).is_none());
        assert!(app.active_wic_session().is_none());
    }

    #[test]
    fn wic_workspace_output_selection_and_creation_cancellation_are_typed() {
        let mut app = qemu_model_app();
        app.wic_capability = wic_model_capability();
        let _ = update(&mut app, Action::BeginSelectedWicCreate);
        let _ = update(&mut app, Action::PreviewWicCreate);
        let Some(Effect::StartWicSession { id, .. }) = update(&mut app, Action::ConfirmWicCreate)
        else {
            panic!("Wic start");
        };
        let _ = update(
            &mut app,
            Action::WicSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::WicSessionRunning { id });
        let _ = update(&mut app, Action::BeginActiveImageRuntimeCancellation);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::WicCancellationConfirmation {
                id: candidate,
                incomplete_device_warning: false,
            }) if *candidate == id
        ));
        let _ = update(&mut app, Action::CancelWicSessionCancellation);
        assert!(app.active_dialog().is_none());

        let output = WicOutput {
            identity: WicOutputIdentity {
                path: "/build/out/image.wic".into(),
                size_bytes: 1,
                modified_unix_seconds: 1,
            },
            kind: WicOutputKind::Wic,
        };
        app.wic_outputs = WicOutputInventoryState::Available {
            request: WicOutputInventoryRequest {
                generation: 1,
                output_directory: "/build/out".into(),
            },
            outputs: vec![output.clone()],
        };
        let _ = update(&mut app, Action::SelectWicOutput { delta: 1 });
        assert_eq!(app.wic_output_selection, Some(output.identity.clone()));
        assert_eq!(
            update(&mut app, Action::OpenSelectedWicOutput),
            Some(Effect::OpenInEditor(output.identity.path))
        );
    }

    fn sdk_workflow_app() -> App {
        let mut app = App::new(20, 20_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace
            .variables
            .insert("DISTRO".into(), "poky".into());
        app.workspace
            .variables
            .insert("SDK_DEPLOY".into(), "/build/deploy/sdk".into());
        app.build.target = Some("core-image-minimal".into());
        app.sdk_tool_capability = SdkToolCapability::Available {
            publish: Some("/opt/poky/oe-publish-sdk".into()),
            find_sysroot: Some("/opt/poky/oe-find-native-sysroot".into()),
            run_native: Some("/opt/poky/oe-run-native".into()),
        };
        app
    }

    #[test]
    fn sdk_workflow_navigates_and_previews_exact_managed_builds() {
        let mut app = sdk_workflow_app();
        let sdk_index = NAVIGATOR_SCREENS
            .iter()
            .position(|screen| *screen == Screen::Sdk)
            .unwrap();
        app.focus = FocusTarget::Navigator;
        app.navigator_selection = sdk_index;
        app.sdk_tool_capability = SdkToolCapability::NotInspected;
        assert_eq!(
            update(&mut app, Action::ActivateNavigator),
            Some(Effect::InspectSdkTools)
        );
        assert_eq!(app.screen, Screen::Sdk);
        let _ = update(
            &mut app,
            Action::BeginSdkBuild(SdkBuildAction::Populate(SdkKind::Extensible)),
        );
        let Some(Dialog::SdkBuildConfirmation(preview)) = app.active_dialog() else {
            panic!("SDK build confirmation");
        };
        assert_eq!(preview.machine, "qemux86-64");
        assert_eq!(preview.distro, "poky");
        assert_eq!(preview.request.task.as_deref(), Some("populate_sdk_ext"));
        assert_eq!(
            update(&mut app, Action::ConfirmSdkBuild),
            Some(Effect::Start(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: Some("populate_sdk_ext".into()),
                force: false,
            }))
        );
        assert!(app.active_dialog().is_none());
    }

    #[test]
    fn sdk_workflow_inventory_publication_native_and_lifecycle_are_correlated() {
        let mut app = sdk_workflow_app();
        let Some(Effect::GetSdkArtifacts(request)) =
            update(&mut app, Action::BeginSdkArtifactInventory)
        else {
            panic!("SDK inventory effect");
        };
        assert_eq!(
            update(&mut app, Action::BeginActiveSdkSessionCancellation),
            Some(Effect::CancelSdkArtifactOperation)
        );
        let stale = SdkArtifactInventoryRequest {
            generation: request.generation + 1,
            ..request.clone()
        };
        let ignored = app.background_jobs.ignored_transitions;
        let _ = update(
            &mut app,
            Action::SdkArtifactInventoryFailed {
                request: stale,
                message: "stale".into(),
            },
        );
        assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
        let artifact = SdkArtifact {
            identity: SdkArtifactIdentity {
                path: "/build/deploy/sdk/poky.sh".into(),
                size_bytes: 42,
                modified_unix_seconds: 7,
            },
            kind: SdkArtifactKind::Installer,
            sdk_kind: Some(SdkKind::Standard),
            machine: Some("qemux86-64".into()),
            host_tuple: Some("x86_64-pokysdk-linux".into()),
            target_tuple: Some("x86_64-poky-linux".into()),
            checksums: Vec::new(),
            manifests: Vec::new(),
            published: None,
        };
        let _ = update(
            &mut app,
            Action::SdkArtifactInventoryLoaded {
                request,
                artifacts: vec![artifact.clone()],
                limitations: vec!["one unrelated record skipped".into()],
            },
        );
        assert_eq!(app.sdk_artifact_selection, Some(artifact.identity.clone()));
        assert!(matches!(
            app.sdk_artifacts,
            SdkArtifactInventoryState::Partial { .. }
        ));
        assert_eq!(
            update(&mut app, Action::OpenSelectedSdkArtifact),
            Some(Effect::OpenInEditor(artifact.identity.path.clone()))
        );

        let _ = update(&mut app, Action::BeginSelectedSdkPublish);
        if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut() {
            editor.text = "destination = \"/srv/sdk\"\n".into();
            editor.cursor = editor.text.len();
        }
        let _ = update(&mut app, Action::PreviewSdkPublish);
        let Some(Effect::StartSdkSession { id, operation }) =
            update(&mut app, Action::ConfirmSdkPublish)
        else {
            panic!("SDK publication effect");
        };
        assert!(matches!(
            operation,
            SdkOperation::Publish(SdkPublishRequest { destination, .. })
                if destination == Path::new("/srv/sdk")
        ));
        let _ = update(
            &mut app,
            Action::SdkSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::SdkSessionRunning { id });
        let _ = update(
            &mut app,
            Action::AppendSdkSessionOutput {
                id,
                stream: SdkOutputStream::Stderr,
                line: "publication warning".into(),
                truncated: true,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(
            &mut app,
            Action::CompleteSdkSession {
                id,
                exit_code: 0,
                artifacts: vec!["/srv/sdk/poky.sh".into()],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let session = app.sdk_session(id).unwrap();
        let job = app.background_jobs.get(session.background_job_id).unwrap();
        assert_eq!(job.status, BackgroundJobStatus::Succeeded);
        assert!(job.output[0].truncated);

        let _ = update(&mut app, Action::BeginSdkNative);
        let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut() else {
            panic!("SDK native dialog");
        };
        editor.text = "mode = \"run-native\"\nworkspace = \"/opt/sdk\"\nrecipe = \"cmake-native\"\ntool = \"cmake\"\narguments = \"--version\"\n".into();
        editor.cursor = editor.text.len();
        let _ = update(&mut app, Action::PreviewSdkNative);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::SdkNativeConfirmation(_))
        ));
        let _ = update(&mut app, Action::CancelSdkNativePreview);

        let _ = update(&mut app, Action::BeginSdkNative);
        if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut() {
            editor.text = "mode = \"run-native\"\nworkspace = \"/opt/sdk\"\nrecipe = \"cmake-native\"\ntool = \"cmake\"\narguments = \"--version\"\n".into();
            editor.cursor = editor.text.len();
        }
        let _ = update(&mut app, Action::PreviewSdkNative);
        let Some(Effect::StartSdkSession { id: native_id, .. }) =
            update(&mut app, Action::ConfirmSdkNative)
        else {
            panic!("SDK native effect");
        };
        let _ = update(
            &mut app,
            Action::SdkSessionStarting {
                id: native_id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::SdkSessionRunning { id: native_id });
        let _ = update(&mut app, Action::BeginActiveSdkSessionCancellation);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::SdkCancellationConfirmation(id)) if *id == native_id
        ));
        assert_eq!(
            update(&mut app, Action::ConfirmSdkSessionCancellation),
            Some(Effect::CancelSdkSession(native_id))
        );
        let _ = update(
            &mut app,
            Action::CancelSdkSession {
                id: native_id,
                exit_code: Some(130),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(
            app.background_jobs
                .get(app.sdk_session(native_id).unwrap().background_job_id)
                .unwrap()
                .status,
            BackgroundJobStatus::Cancelled
        );
    }

    fn test_workflow_app() -> App {
        let mut app = App::new(20, 20_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace
            .variables
            .insert("DISTRO".into(), "poky".into());
        app.build.target = Some("core-image-minimal".into());
        app.test_capability = TestCapability {
            oe_selftest: TestExecutableCapability::Available("/workspace/oe-selftest".into()),
            bitbake_selftest: TestExecutableCapability::Available(
                "/workspace/bitbake-selftest".into(),
            ),
            ptest: PtestCapability::Configured,
        };
        app
    }

    #[test]
    fn test_workflow_model_navigates_previews_and_runs_bounded_selftests() {
        let mut app = test_workflow_app();
        let testing_index = NAVIGATOR_SCREENS
            .iter()
            .position(|screen| *screen == Screen::Testing)
            .unwrap();
        app.focus = FocusTarget::Navigator;
        app.navigator_selection = testing_index;
        app.test_capability = TestCapability::default();
        assert_eq!(
            update(&mut app, Action::ActivateNavigator),
            Some(Effect::InspectTestCapability)
        );
        assert_eq!(app.screen, Screen::Testing);
        let capability = test_workflow_app().test_capability;
        let _ = update(&mut app, Action::TestCapabilityLoaded(capability));
        let _ = update(&mut app, Action::BeginSelectedTestLaunch);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::TestLaunchTomlEditor { editor, .. })
                if editor.selected_text() == Some("all")
        ));
        if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog_mut() {
            editor.text = "family = \"OE selftest\"\nmachine = \"qemux86-64\"\ndistro = \"poky\"\nimage = \"core-image-minimal\"\nscope = \"selected\"\nselector = \"tinfoil.TinfoilTests.test_getvar\"\nparallelism = 8\nverbose = false\nskip_network = false\n".into();
        }
        let _ = update(&mut app, Action::PreviewTestLaunch);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::TestLaunchConfirmation(
                TestLaunchPreview::Selftest(request)
            )) if request.parallelism == 8
                && request.selector.as_deref() == Some("tinfoil.TinfoilTests.test_getvar")
        ));
        let Some(Effect::StartTestSession { id, operation }) =
            update(&mut app, Action::ConfirmTestLaunch)
        else {
            panic!("selftest effect");
        };
        assert!(matches!(
            operation,
            TestOperation::Selftest(TestSelftestRequest {
                family: TestFamily::OeSelftest,
                ..
            })
        ));
        let job_id = app.test_session(id).unwrap().background_job_id.unwrap();
        assert_eq!(
            app.background_jobs.get(job_id).unwrap().kind,
            BackgroundJobKind::Test
        );
        let _ = update(
            &mut app,
            Action::TestSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::TestSessionRunning { id });
        let _ = update(
            &mut app,
            Action::AppendTestSessionOutput {
                id,
                stream: TestOutputStream::Stderr,
                line: "one warning".into(),
                truncated: true,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::BeginActiveTestSessionCancellation);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::TestCancellationConfirmation(candidate)) if *candidate == id
        ));
        assert_eq!(
            update(&mut app, Action::ConfirmTestSessionCancellation),
            Some(Effect::CancelTestSession(id))
        );
        let _ = update(
            &mut app,
            Action::RejectTestSessionCancellation {
                id,
                message: "still stopping".into(),
            },
        );
        assert_eq!(
            app.background_jobs.get(job_id).unwrap().status,
            BackgroundJobStatus::Running
        );
        let _ = update(&mut app, Action::BeginActiveTestSessionCancellation);
        let _ = update(&mut app, Action::ConfirmTestSessionCancellation);
        let _ = update(
            &mut app,
            Action::CancelTestSession {
                id,
                exit_code: Some(130),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let job = app.background_jobs.get(job_id).unwrap();
        assert_eq!(job.status, BackgroundJobStatus::Cancelled);
        assert!(job.output[0].truncated);
    }

    #[test]
    fn test_workflow_launch_editor_rejects_changed_authoritative_context() {
        let mut app = test_workflow_app();
        let _ = update(&mut app, Action::BeginSelectedTestLaunch);
        let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog_mut() else {
            panic!("test launch TOML editor");
        };
        editor.text = editor.text.replace(
            "machine = \"qemux86-64\"",
            "machine = \"untrusted-machine\"",
        );

        let _ = update(&mut app, Action::PreviewTestLaunch);

        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::TestLaunchTomlEditor {
                validation_error: Some(message),
                ..
            }) if message.contains("must match the current Testing context")
        ));
    }

    #[test]
    fn test_workflow_model_attaches_managed_builds_and_rejects_stale_results() {
        let mut app = test_workflow_app();
        let _ = update(&mut app, Action::SelectTestFamily { delta: 2 });
        let _ = update(&mut app, Action::BeginSelectedTestLaunch);
        let _ = update(&mut app, Action::PreviewTestLaunch);
        let Some(Effect::StartTestBuildSession { id, request }) =
            update(&mut app, Action::ConfirmTestLaunch)
        else {
            panic!("test build effect");
        };
        assert_eq!(request.task.as_deref(), Some("testimage"));
        assert!(app.test_session(id).unwrap().background_job_id.is_none());
        let job_id = BackgroundJobId(44);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(BackgroundJobSpec {
                id: job_id,
                kind: BackgroundJobKind::Test,
                title: "Image runtime test".into(),
                context: BackgroundJobContext {
                    workspace: Some(Screen::Testing),
                    image: Some("core-image-minimal".into()),
                    task: Some("testimage".into()),
                    ..BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at: SystemTime::UNIX_EPOCH,
            }),
        );
        let _ = update(
            &mut app,
            Action::AttachTestBuildSession {
                id,
                background_job_id: job_id,
            },
        );
        assert_eq!(
            app.test_session(id).unwrap().background_job_id,
            Some(job_id)
        );
        let ignored = app.background_jobs.ignored_transitions;
        let _ = update(
            &mut app,
            Action::CompleteTestSession {
                id,
                exit_code: 0,
                result_paths: vec!["relative/testresults.json".into()],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
        let _ = update(
            &mut app,
            Action::TestSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::TestSessionRunning { id });
        let _ = update(
            &mut app,
            Action::CompleteTestSession {
                id,
                exit_code: 0,
                result_paths: vec!["/build/testresults.json".into()],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(
            app.background_jobs.get(job_id).unwrap().status,
            BackgroundJobStatus::Succeeded
        );
        assert_eq!(
            app.test_session(id).unwrap().result_paths,
            [PathBuf::from("/build/testresults.json")]
        );
    }

    #[test]
    fn test_workflow_model_records_failure_loss_and_stale_terminal_events() {
        fn queue_selftest(app: &mut App) -> TestSessionId {
            let request = TestSelftestRequest::new(
                "/workspace/oe-selftest".into(),
                TestFamily::OeSelftest,
                None,
                1,
                false,
                false,
            )
            .unwrap();
            let Some(Effect::StartTestSession { id, .. }) =
                queue_test_session(app, TestOperation::Selftest(request))
            else {
                panic!("selftest session");
            };
            id
        }

        let mut failed = test_workflow_app();
        let failed_id = queue_selftest(&mut failed);
        let failed_job = failed
            .test_session(failed_id)
            .unwrap()
            .background_job_id
            .unwrap();
        let _ = update(
            &mut failed,
            Action::FailTestSession {
                id: failed_id,
                message: "runner timed out after forced termination".into(),
                exit_code: None,
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let job = failed.background_jobs.get(failed_job).unwrap();
        assert_eq!(job.status, BackgroundJobStatus::Failed);
        assert_eq!(
            job.error.as_ref().and_then(|error| error.detail.as_deref()),
            Some("runner timed out after forced termination")
        );

        let mut lost = test_workflow_app();
        let lost_id = queue_selftest(&mut lost);
        let lost_job = lost
            .test_session(lost_id)
            .unwrap()
            .background_job_id
            .unwrap();
        let _ = update(
            &mut lost,
            Action::LoseTestSession {
                id: lost_id,
                message: "runner event channel closed".into(),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(
            lost.background_jobs.get(lost_job).unwrap().status,
            BackgroundJobStatus::Lost
        );

        let mut timed_out = test_workflow_app();
        let timed_out_id = queue_selftest(&mut timed_out);
        let _ = update(
            &mut timed_out,
            Action::TimeoutTestSession {
                id: timed_out_id,
                forced: true,
                exit_code: None,
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(
            timed_out.test_session(timed_out_id).unwrap().outcome,
            Some(TestSessionOutcome::TimedOut)
        );

        let ignored = lost.background_jobs.ignored_transitions;
        let _ = update(
            &mut lost,
            Action::FailTestSession {
                id: TestSessionId(u64::MAX),
                message: "stale".into(),
                exit_code: Some(1),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(lost.background_jobs.ignored_transitions, ignored + 1);
    }

    fn test_results_record(
        name: &str,
        fingerprint: &str,
        outcomes: &[(&str, TestCaseOutcome)],
    ) -> TestResultRecord {
        let cases = outcomes
            .iter()
            .map(|(case_name, outcome)| {
                TestCaseRecord::new(
                    TestCaseIdentity::new("suite".into(), (*case_name).into()).unwrap(),
                    *outcome,
                    Some(Duration::from_millis(10)),
                    Vec::new(),
                    Some(format!("/build/logs/{name}-{case_name}.log").into()),
                )
                .unwrap()
                .0
            })
            .collect();
        let suite = TestSuiteRecord::new("suite".into(), None, Vec::new(), cases)
            .unwrap()
            .0;
        TestResultRecord::new(
            TestResultIdentity::new(
                format!("/build/results/{name}/testresults.json").into(),
                2_048,
                SystemTime::UNIX_EPOCH,
                fingerprint.into(),
            )
            .unwrap(),
            Some(TestFamily::TestImage),
            Some("qemux86-64".into()),
            Some("core-image-minimal".into()),
            Some("rev-1".into()),
            Some(Duration::from_secs(1)),
            Vec::new(),
            vec![suite],
            Some(TestSessionId(1)),
            Vec::new(),
        )
        .0
    }

    fn load_test_results(
        app: &mut App,
        records: Vec<TestResultRecord>,
        limitations: Vec<String>,
    ) -> TestResultImportRequest {
        let Some(Effect::ImportTestResults(request)) =
            begin_test_result_import(app, vec!["/build/results".into()])
        else {
            panic!("import effect");
        };
        let _ = update(
            app,
            Action::TestResultsLoaded {
                request: request.clone(),
                records,
                limitations,
            },
        );
        request
    }

    #[test]
    fn test_results_reducer_correlates_empty_partial_search_drill_and_stale_data() {
        let mut empty = test_workflow_app();
        let request = load_test_results(&mut empty, Vec::new(), Vec::new());
        assert!(matches!(
            empty.test_results,
            TestResultInventoryState::AvailableEmpty { .. }
        ));
        let ignored = empty.background_jobs.ignored_transitions;
        let _ = update(
            &mut empty,
            Action::TestResultsLost {
                request,
                message: "late worker loss".into(),
            },
        );
        assert_eq!(empty.background_jobs.ignored_transitions, ignored + 1);

        let baseline =
            test_results_record("baseline", "base", &[("case", TestCaseOutcome::Passed)]);
        let candidate = test_results_record(
            "candidate",
            "candidate",
            &[("case", TestCaseOutcome::Failed)],
        );
        let mut app = test_workflow_app();
        let old_request = load_test_results(
            &mut app,
            vec![candidate.clone(), baseline.clone(), candidate],
            vec!["one malformed adapter record was skipped".into()],
        );
        let TestResultInventoryState::Partial {
            records,
            limitations,
            ..
        } = &app.test_results
        else {
            panic!("partial inventory");
        };
        assert_eq!(records.len(), 2);
        assert!(limitations.iter().any(|value| value.contains("duplicate")));
        assert_eq!(app.test_result_selection.as_ref(), Some(&baseline.identity));
        let _ = update(&mut app, Action::BeginTestResultSearch);
        for character in "candidate".chars() {
            let _ = update(&mut app, Action::AppendTestResultQuery(character));
        }
        assert_eq!(
            app.test_result_selection.as_ref(),
            Some(
                &test_results_record(
                    "candidate",
                    "candidate",
                    &[("case", TestCaseOutcome::Failed)]
                )
                .identity
            ),
            "search falls back to the first visible exact result"
        );
        assert_eq!(app.filtered_test_results().len(), 1);
        let _ = update(&mut app, Action::SelectTestResult { delta: 1 });
        assert_eq!(
            app.test_result_selection.as_ref(),
            Some(
                &test_results_record(
                    "candidate",
                    "candidate",
                    &[("case", TestCaseOutcome::Failed)]
                )
                .identity
            )
        );
        let _ = update(&mut app, Action::DrillIntoSelectedTestResult);
        assert!(app.test_result_drilled);
        assert!(matches!(
            update(&mut app, Action::OpenSelectedTestCaseLog),
            Some(Effect::OpenInEditor(path))
                if path == Path::new("/build/logs/candidate-case.log")
        ));
        assert!(matches!(
            update(&mut app, Action::OpenSelectedTestResult),
            Some(Effect::OpenInEditor(path))
                if path == Path::new("/build/results/candidate/testresults.json")
        ));

        let Some(Effect::ImportTestResults(new_request)) =
            update(&mut app, Action::RefreshTestResults)
        else {
            panic!("refresh effect");
        };
        assert_ne!(old_request, new_request);
        let mut malformed = baseline;
        malformed.identity.path = "relative.json".into();
        let ignored = app.background_jobs.ignored_transitions;
        let _ = update(
            &mut app,
            Action::TestResultsLoaded {
                request: new_request,
                records: vec![malformed],
                limitations: Vec::new(),
            },
        );
        assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
        assert!(matches!(
            app.test_results,
            TestResultInventoryState::Loading { .. }
        ));
    }

    #[test]
    fn test_results_reducer_previews_exact_comparison_and_rejects_inconsistent_results() {
        let baseline =
            test_results_record("baseline", "base", &[("case", TestCaseOutcome::Passed)]);
        let candidate = test_results_record(
            "candidate",
            "candidate",
            &[("case", TestCaseOutcome::Failed)],
        );
        let mut app = test_workflow_app();
        app.result_tool_capability =
            ResultToolCapability::Available("/workspace/resulttool".into());
        load_test_results(
            &mut app,
            vec![baseline.clone(), candidate.clone()],
            Vec::new(),
        );
        let _ = update(&mut app, Action::BeginTestComparison);
        let _ = update(&mut app, Action::PreviewTestComparison);
        let Some(Dialog::TestComparisonConfirmation(preview)) = app.active_dialog().cloned() else {
            panic!("comparison preview");
        };
        assert_eq!(
            preview.argv,
            [
                PathBuf::from("/workspace/resulttool"),
                "regression-file".into(),
                baseline.identity.path.clone(),
                candidate.identity.path.clone(),
            ]
        );
        let Some(Effect::CompareTestResults(request)) =
            update(&mut app, Action::ConfirmTestComparison)
        else {
            panic!("comparison effect");
        };
        let comparison = TestComparison::between(&baseline, &candidate).unwrap();
        let _ = update(
            &mut app,
            Action::TestComparisonLoaded {
                request: request.clone(),
                comparison,
                limitations: vec!["resulttool omitted optional metadata".into()],
            },
        );
        assert!(matches!(
            app.test_comparison,
            TestComparisonState::Partial { .. }
        ));
        assert_eq!(
            app.selected_test_transition().unwrap().category,
            TestComparisonCategory::Regression
        );
        assert!(matches!(
            update(&mut app, Action::OpenSelectedTestTransitionLog),
            Some(Effect::OpenInEditor(path))
                if path == Path::new("/build/logs/candidate-case.log")
        ));

        let ignored = app.background_jobs.ignored_transitions;
        let _ = update(
            &mut app,
            Action::TestComparisonFailed {
                request,
                message: "late failure".into(),
            },
        );
        assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
    }

    #[test]
    fn test_results_popup_editors_share_selection_navigation_and_clipboard() {
        let mut app = test_workflow_app();
        let _ = update(&mut app, Action::BeginTestResultImport);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::TestResultImportTomlEditor { editor, .. })
                if editor.selected_text() == Some("")
        ));
        assert!(matches!(
            update(
                &mut app,
                Action::EditActivePopup(PopupEditorCommand::Copy)
            ),
            Some(Effect::CopyToClipboard(value)) if value.is_empty()
        ));

        app.dialogs.clear();
        let baseline =
            test_results_record("baseline", "base", &[("case", TestCaseOutcome::Passed)]);
        let candidate = test_results_record(
            "candidate",
            "candidate",
            &[("case", TestCaseOutcome::Failed)],
        );
        load_test_results(&mut app, vec![baseline, candidate], Vec::new());
        let _ = update(&mut app, Action::BeginTestComparison);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::TestComparisonTomlEditor { editor, .. })
                if editor.selected_text().is_some_and(|value| value.starts_with("/build/results/"))
        ));
        assert!(matches!(
            update(
                &mut app,
                Action::EditActivePopup(PopupEditorCommand::Copy)
            ),
            Some(Effect::CopyToClipboard(value)) if value.starts_with("/build/results/")
        ));
    }

    #[test]
    fn test_results_reducer_validates_junit_destination_and_correlates_failure() {
        let result = test_results_record(
            "candidate",
            "candidate",
            &[("case", TestCaseOutcome::Failed)],
        );
        let mut app = test_workflow_app();
        app.result_tool_capability =
            ResultToolCapability::Available("/workspace/resulttool".into());
        load_test_results(&mut app, vec![result.clone()], Vec::new());
        let _ = update(&mut app, Action::BeginTestJunitExport);
        if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
            editor.text = "destination = \"/exports/candidate.xml\"\n".into();
            editor.cursor = editor.text.len();
        }
        let Some(Effect::InspectTestJunitDestination {
            result: inspected_result,
            destination,
        }) = update(&mut app, Action::PreviewTestJunitExport)
        else {
            panic!("destination inspection effect");
        };
        assert_eq!(inspected_result, result.identity);
        assert_eq!(destination, PathBuf::from("/exports/candidate.xml"));
        let _ = update(
            &mut app,
            Action::TestJunitDestinationInspected {
                result: result.identity.clone(),
                inspection: TestJunitDestinationInspection {
                    requested: destination.clone(),
                    canonical_parent: Some("/exports".into()),
                    parent_exists: true,
                    parent_is_directory: true,
                    destination_exists: true,
                    destination_is_symlink: false,
                },
            },
        );
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::TestJunitTomlEditor {
                validation_error: Some(_),
                ..
            })
        ));
        assert!(
            update(&mut app, Action::PreviewTestJunitExport).is_some(),
            "the corrected retry requests fresh filesystem validation"
        );
        let _ = update(
            &mut app,
            Action::TestJunitDestinationInspected {
                result: result.identity.clone(),
                inspection: TestJunitDestinationInspection {
                    requested: destination,
                    canonical_parent: Some("/exports".into()),
                    parent_exists: true,
                    parent_is_directory: true,
                    destination_exists: false,
                    destination_is_symlink: false,
                },
            },
        );
        let Some(Dialog::TestJunitExportConfirmation(preview)) = app.active_dialog().cloned()
        else {
            panic!("JUnit confirmation");
        };
        assert_eq!(
            preview.argv,
            [
                PathBuf::from("/workspace/resulttool"),
                "junit".into(),
                result.identity.path,
                "-j".into(),
                "/exports/candidate.xml".into(),
            ]
        );
        let Some(Effect::ExportTestJunit(request)) =
            update(&mut app, Action::ConfirmTestJunitExport)
        else {
            panic!("JUnit export effect");
        };
        let stale = TestJunitExportRequest {
            generation: request.generation + 1,
            ..request.clone()
        };
        let ignored = app.background_jobs.ignored_transitions;
        let _ = update(
            &mut app,
            Action::TestJunitExportSucceeded { request: stale },
        );
        assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
        let _ = update(
            &mut app,
            Action::TestJunitExportFailed {
                request: request.clone(),
                message: "resulttool exited 1".into(),
            },
        );
        assert!(matches!(
            app.test_junit_export,
            TestJunitExportState::Failed { .. }
        ));
        let ignored = app.background_jobs.ignored_transitions;
        let _ = update(&mut app, Action::TestJunitExportSucceeded { request });
        assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
    }

    #[test]
    fn build_environment_requires_a_verified_correlated_connection() {
        let profile = BuildEnvironmentProfile {
            source_dir: PathBuf::from("/workspace/poky"),
            build_dir: PathBuf::from("/workspace/build"),
            init_script: PathBuf::from("/workspace/poky/oe-init-build-env"),
        };
        let mut app = App::new_unconfigured(16, 4096);
        assert_eq!(app.screen, Screen::BuildEnvironment);
        assert_eq!(app.focus, FocusTarget::Navigator);
        let request = BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        };
        assert_eq!(update(&mut app, Action::Start(request.clone())), None);
        assert_eq!(
            app.notification.as_deref(),
            Some("Configure and verify a BitBake environment first")
        );

        assert_eq!(
            update(&mut app, Action::ConfigureBuildEnvironment(profile.clone())),
            None
        );
        let Some(Effect::VerifyBuildEnvironment { generation, .. }) =
            update(&mut app, Action::BeginBuildEnvironmentVerification)
        else {
            panic!("verification effect");
        };
        let _ = update(
            &mut app,
            Action::BuildEnvironmentVerified {
                generation: generation + 1,
            },
        );
        assert!(matches!(
            app.build_environment,
            BuildEnvironmentState::Verifying { .. }
        ));
        let _ = update(&mut app, Action::BuildEnvironmentVerified { generation });
        assert_eq!(
            app.build_environment,
            BuildEnvironmentState::Connected(profile)
        );
        assert_eq!(
            update(&mut app, Action::Start(request.clone())),
            Some(Effect::Start(request))
        );
    }

    #[test]
    fn build_environment_form_edits_typed_profile_and_clears_inventory() {
        let mut app = App::new_unconfigured(8, 512);
        app.available_images = vec!["core-image-minimal".into()];
        let _ = update(&mut app, Action::BeginBuildEnvironmentEdit);
        let _ = update(&mut app, Action::AppendBuildEnvironmentField('/'));
        for c in "src".chars() {
            let _ = update(&mut app, Action::AppendBuildEnvironmentField(c));
        }
        let _ = update(&mut app, Action::SelectBuildEnvironmentField { delta: 1 });
        for c in "/build".chars() {
            let _ = update(&mut app, Action::AppendBuildEnvironmentField(c));
        }
        let _ = update(&mut app, Action::SelectBuildEnvironmentField { delta: 1 });
        for c in "/env".chars() {
            let _ = update(&mut app, Action::AppendBuildEnvironmentField(c));
        }
        let _ = update(&mut app, Action::ApplyBuildEnvironmentProfile);
        assert!(matches!(
            app.build_environment,
            BuildEnvironmentState::Configured(_)
        ));
        assert!(app.available_images.is_empty());
    }

    #[test]
    fn recipe_refresh_rebuilds_authoritative_image_inventory() {
        let mut app = App::new(16, 4096);
        let _ = update(
            &mut app,
            Action::RecipesLoaded(vec![
                Recipe {
                    name: "base-files".into(),
                    ..Recipe::default()
                },
                Recipe {
                    name: "core-image-minimal".into(),
                    ..Recipe::default()
                },
            ]),
        );
        assert_eq!(app.available_images, vec!["core-image-minimal"]);
    }

    #[test]
    fn build_environment_toml_editor_applies_profile_from_popup() {
        let mut app = App::new_unconfigured(8, 512);
        let _ = update(&mut app, Action::OpenBuildEnvironmentEditor);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::BuildEnvironmentEditor(editor)) if !editor.editing
        ));
        if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog_mut() {
            editor.editing = true;
            editor.text = "source = \"/src/poky\"\nbuild = \"/src/build\"\nscript = \"/src/poky/oe-init-build-env\"\n".into();
            editor.cursor = editor.text.len();
        }
        let _ = update(&mut app, Action::ApplyBuildEnvironmentEditor);
        assert!(matches!(
            app.build_environment,
            BuildEnvironmentState::Configured(_)
        ));
    }

    #[test]
    fn build_environment_popup_uses_shared_selection_navigation_and_clipboard() {
        let mut app = App::new_unconfigured(8, 512);
        let _ = update(
            &mut app,
            Action::ConfigureBuildEnvironment(BuildEnvironmentProfile {
                source_dir: "/old/source".into(),
                build_dir: "/old/build".into(),
                init_script: "/old/source/oe-init-build-env".into(),
            }),
        );
        let _ = update(&mut app, Action::OpenBuildEnvironmentEditor);
        assert!(matches!(
            update(
                &mut app,
                Action::EditActivePopup(PopupEditorCommand::Copy)
            ),
            Some(Effect::CopyToClipboard(value)) if value == "/old/source"
        ));
        let _ = update(
            &mut app,
            Action::EditActivePopup(PopupEditorCommand::ToggleInsert),
        );
        for character in "/new/source".chars() {
            let _ = update(
                &mut app,
                Action::EditActivePopup(PopupEditorCommand::Insert(character)),
            );
        }
        let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog() else {
            panic!("build environment editor");
        };
        assert!(editor.text.starts_with("source = \"/new/source\""));
    }

    #[test]
    fn build_environment_rejects_relative_profiles_and_preserves_unconfigured_state() {
        let mut app = App::new_unconfigured(16, 4096);
        let _ = update(
            &mut app,
            Action::ConfigureBuildEnvironment(BuildEnvironmentProfile {
                source_dir: PathBuf::from("poky"),
                build_dir: PathBuf::from("/workspace/build"),
                init_script: PathBuf::from("/workspace/poky/oe-init-build-env"),
            }),
        );
        assert_eq!(app.build_environment, BuildEnvironmentState::Unconfigured);
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("absolute"))
        );
    }

    #[test]
    fn build_environment_clone_request_rejects_unsafe_revision_and_destination() {
        let mut request = BuildEnvironmentCloneRequest {
            repository: "https://example.invalid/poky".into(),
            destination: PathBuf::from("/workspace/poky"),
            revision: Some("main;rm -rf".into()),
        };
        assert!(request.validate().is_err());
        request.revision = Some("scarthgap".into());
        request.destination = PathBuf::from("relative/poky");
        assert!(request.validate().is_err());
    }

    #[test]
    fn build_environment_clone_editor_requires_review_before_emitting_clone_effect() {
        let mut app = App::new_unconfigured(8, 512);
        let _ = update(&mut app, Action::OpenBuildEnvironmentCloneEditor);
        if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog_mut() {
            editor.text = "repository = \"https://git.yoctoproject.org/poky\"\ndestination = \"/tmp/poky\"\nrevision = \"\"\nbuild = \"/tmp/poky/build\"\n".into();
            editor.cursor = editor.text.len();
        }
        let _ = update(&mut app, Action::ReviewBuildEnvironmentClone);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::BuildEnvironmentCloneReview(_))
        ));
        assert!(matches!(
            update(&mut app, Action::ConfirmBuildEnvironmentClone),
            Some(Effect::CloneBuildEnvironment(_))
        ));
    }

    #[test]
    fn popup_editor_replaces_selection_and_moves_to_line_bounds() {
        let mut editor = PopupEditor::new("path = \"old\"\nnext = \"value\"".into());
        editor.select_range(8, 11);
        assert_eq!(editor.selected_text(), Some("old"));
        editor.insert("/home/user/poky");
        assert!(editor.text.contains("/home/user/poky"));
        editor.end();
        assert_eq!(editor.cursor, editor.text.find('\n').unwrap());
        editor.home();
        assert_eq!(editor.cursor, 0);
    }

    #[test]
    fn popup_editor_selects_a_toml_value_and_undoes_replacement() {
        let mut editor = PopupEditor::new("path = \"/old\"\nmode = \"safe\"\n".into());
        editor.select_toml_value("path").unwrap();
        assert_eq!(editor.selected_text(), Some("/old"));
        editor.insert("/new");
        assert_eq!(editor.text, "path = \"/new\"\nmode = \"safe\"\n");
        assert!(editor.undo());
        assert_eq!(editor.text, "path = \"/old\"\nmode = \"safe\"\n");
        assert!(!editor.undo());
    }

    #[test]
    fn popup_editor_selects_native_toml_boolean_and_integer_values() {
        let mut editor = PopupEditor::new("enabled = true\njobs = 12 # bounded\n".into());
        editor.select_toml_value("enabled").unwrap();
        assert_eq!(editor.selected_text(), Some("true"));
        editor.cursor = editor.text.find("jobs").unwrap();
        editor.select_toml_value_at_cursor().unwrap();
        assert_eq!(editor.selected_text(), Some("12"));
    }

    #[test]
    fn popup_editor_supports_unicode_navigation_copy_paste_and_backspace() {
        let mut editor = PopupEditor::new("path = \"hé\"\n".into());
        editor.select_toml_value("path").unwrap();
        assert_eq!(editor.copy_selection_or_line(), "hé");
        editor.editing = true;
        editor.insert("x");
        editor.paste();
        assert_eq!(editor.text, "path = \"xhé\"\n");
        editor.left();
        editor.backspace();
        assert_eq!(editor.text, "path = \"xé\"\n");
        editor.home();
        assert_eq!(editor.cursor, 0);
        editor.end();
        assert_eq!(editor.cursor, "path = \"xé\"".len());
    }

    #[test]
    fn junit_popup_editor_routes_selection_navigation_and_clipboard_actions() {
        let result = test_results_record("candidate", "candidate", &[]);
        let mut app = test_workflow_app();
        app.result_tool_capability =
            ResultToolCapability::Available("/workspace/resulttool".into());
        load_test_results(&mut app, vec![result], Vec::new());
        let _ = update(&mut app, Action::BeginTestJunitExport);
        let _ = update(&mut app, Action::SelectTestJunitDestination);
        let _ = update(&mut app, Action::AppendTestJunitTomlEditor('x'));
        assert!(matches!(
            update(&mut app, Action::CopyTestJunitTomlEditor),
            Some(Effect::CopyToClipboard(value)) if value.contains("destination")
        ));
        let _ = update(&mut app, Action::MoveTestJunitTomlEditorHome);
        let _ = update(&mut app, Action::MoveTestJunitTomlEditorEnd);
        let _ = update(&mut app, Action::PasteTestJunitTomlEditor);
        let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog() else {
            panic!("JUnit editor");
        };
        assert!(
            editor
                .text
                .contains("destination = \"x\"destination = \"x\"")
        );
    }
}
