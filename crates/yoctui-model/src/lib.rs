//! Domain model and pure state transitions. BitBake remains authoritative.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    fmt,
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime},
};
use thiserror::Error;

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
    LayerRelationships,
    Recipes,
    Images,
    Layers,
    Configuration,
    Bbmask,
    Logs,
    Errors,
    Help,
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
    Dark,
    Light,
    MatrixGreen,
    HighContrast,
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
const NAVIGATOR_SCREENS: [Screen; 12] = [
    Screen::Dashboard,
    Screen::Layers,
    Screen::Recipes,
    Screen::Images,
    Screen::Tasks,
    Screen::Logs,
    Screen::Errors,
    Screen::Configuration,
    Screen::Dependencies,
    Screen::Recipes,
    Screen::Bbmask,
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
            | Self::Reset { recipe } => recipe,
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
    pub disk_available_bytes: Option<u64>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevtoolCapability {
    Available,
    MissingExecutable,
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
        if self.capability == DevtoolCapability::MissingExecutable {
            return Some("Devtool executable is missing.".into());
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
pub enum Dialog {
    BuildOptions,
    BuildCompletion,
    BuildTarget {
        input: String,
        task: Option<String>,
    },
    ImagePicker(ImagePicker),
    RecipeTaskConfirmation(BuildRequest),
    RecipeTaskPicker(RecipeTaskPicker),
    RecipeTaskLogPicker(RecipeTaskLogPicker),
    RecipePatchPicker(RecipePatchPicker),
    ConfigSourcePicker(ConfigSourcePicker),
    ConfigScopePicker(ConfigScopePicker),
    ConfigComparison(ConfigComparison),
    ConfigEdit {
        identity: VariableIdentity,
        input: String,
    },
    ConfigEditConfirmation(ConfigEditRequest),
    DevtoolModifyConfirmation(RecipeIdentity),
    DevtoolResetConfirmation(DevtoolResetPlan),
    DevtoolUpdateConfirmation(RecipeIdentity),
    DevtoolFinishPicker(DevtoolFinishPicker),
    DevtoolFinishConfirmation(DevtoolFinishPlan),
    DevtoolDeploy(DevtoolDeployDraft),
    DevtoolDeployConfirmation(DevtoolDeployPlan),
    BbmaskEdit {
        input: String,
    },
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
    pub screen: Screen,
    pub focus: FocusTarget,
    pub focus_return: Option<FocusTarget>,
    pub navigator_selection: usize,
    pub backend: String,
    pub color_enabled: bool,
    pub theme: Theme,
    pub animation_speed: AnimationSpeed,
    pub reduced_motion: bool,
    pub settings_selection: usize,
    pub settings_dirty: bool,
    pub animation_frame: u64,
    pub workspace: Workspace,
    pub host_telemetry: HostTelemetry,
    pub build: BuildState,
    pub background_jobs: BackgroundJobs,
    pub build_history: VecDeque<BuildRecord>,
    pub build_history_selection: usize,
    pub dependencies: Option<RecipeDependencies>,
    pub dependency_selection: usize,
    pub dependency_graph: DependencyGraphState,
    pub dependency_graph_selection: Option<DependencyNodeId>,
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
            screen: Screen::Dashboard,
            focus: FocusTarget::Workspace,
            focus_return: None,
            navigator_selection: 0,
            backend: "unknown".into(),
            color_enabled: true,
            theme: Theme::Dark,
            animation_speed: AnimationSpeed::Fast,
            reduced_motion: false,
            settings_selection: 0,
            settings_dirty: false,
            animation_frame: 0,
            workspace: Workspace::default(),
            host_telemetry: HostTelemetry::default(),
            build: BuildState::default(),
            background_jobs: BackgroundJobs::default(),
            build_history: VecDeque::new(),
            build_history_selection: 0,
            dependencies: None,
            dependency_selection: 0,
            dependency_graph: DependencyGraphState::NotLoaded,
            dependency_graph_selection: None,
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
    pub fn elapsed(&self) -> Option<Duration> {
        self.build
            .started
            .and_then(|s| SystemTime::now().duration_since(s).ok())
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
                task_state_order(left.state),
                left.recipe.as_str(),
                left.task.as_str(),
                left.id.0.as_str(),
            )
                .cmp(&(
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
    Open(Screen),
    SelectNavigator {
        delta: isize,
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
    OpenBuildOptions,
    CloseBuildOptions,
    OpenImagePicker(Vec<String>),
    SelectImage {
        delta: isize,
    },
    ConfirmImagePicker,
    CancelImagePicker,
    BeginCurrentImageBuild,
    BeginBuildTargetEdit,
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
    const THEMES: [Theme; 5] = [
        Theme::Dark,
        Theme::Light,
        Theme::MatrixGreen,
        Theme::HighContrast,
        Theme::Monochrome,
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
    app.screen = Screen::Dependencies;
    app.dependency_graph = if let Some(limitations) = limitations {
        DependencyGraphState::Partial { graph, limitations }
    } else if graph.edges.is_empty() {
        DependencyGraphState::AvailableEmpty { root: graph.root }
    } else {
        DependencyGraphState::Available(graph)
    };
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
        Action::ActivateNavigator => {
            app.screen = NAVIGATOR_SCREENS[app.navigator_selection];
            app.focus = FocusTarget::Workspace;
            app.focus_return = None;
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
            match SETTINGS[app.settings_selection.min(SETTINGS.len() - 1)] {
                Setting::Theme => app.theme = cycle_theme(app.theme, backwards),
                Setting::AnimationSpeed => {
                    app.animation_speed = match app.animation_speed {
                        AnimationSpeed::Slow => AnimationSpeed::Fast,
                        AnimationSpeed::Fast => AnimationSpeed::Slow,
                    }
                }
                Setting::ReducedMotion => app.reduced_motion = !app.reduced_motion,
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
            open_dialog(app, Dialog::BuildOptions);
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
        Action::BeginBuildTargetEdit => {
            replace_dialog(
                app,
                Dialog::BuildTarget {
                    input: app.build.target.clone().unwrap_or_default(),
                    task: None,
                },
            );
        }
        Action::BeginBuildTargetTask(task) => {
            replace_dialog(
                app,
                Dialog::BuildTarget {
                    input: app.build.target.clone().unwrap_or_default(),
                    task,
                },
            );
        }
        Action::AppendBuildTarget(character) => {
            if let Some(Dialog::BuildTarget { input, .. }) = app.active_dialog_mut() {
                input.push(character);
            }
        }
        Action::BackspaceBuildTarget => {
            if let Some(Dialog::BuildTarget { input, .. }) = app.active_dialog_mut() {
                input.pop();
            }
        }
        Action::CancelBuildTargetEdit => {
            if matches!(app.active_dialog(), Some(Dialog::BuildTarget { .. })) {
                close_dialog(app);
            }
        }
        Action::ConfirmBuildTarget => {
            if let Some(Dialog::BuildTarget { input, task }) = app.active_dialog() {
                let request = BuildRequest {
                    targets: vec![input.clone()],
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
            if let Err(e) = r.validate() {
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
            Ok((identity, value, _)) => open_dialog(
                app,
                Dialog::ConfigEdit {
                    identity,
                    input: value,
                },
            ),
            Err(reason) => app.notification = Some(reason),
        },
        Action::AppendConfigEdit(character) => {
            if character.is_control() {
                app.notification =
                    Some("Configuration values cannot contain control characters.".into());
            } else if let Some(Dialog::ConfigEdit { input, .. }) = app.active_dialog_mut() {
                input.push(character);
            }
        }
        Action::BackspaceConfigEdit => {
            if let Some(Dialog::ConfigEdit { input, .. }) = app.active_dialog_mut() {
                input.pop();
            }
        }
        Action::PreviewConfigEdit => {
            let edit = app.active_dialog().and_then(|dialog| match dialog {
                Dialog::ConfigEdit { identity, input } => Some((identity.clone(), input.clone())),
                _ => None,
            });
            let (identity, value) = edit?;
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
            open_dialog(app, Dialog::BbmaskEdit { input });
        }
        Action::AppendBbmask(character) => {
            if let Some(Dialog::BbmaskEdit { input }) = app.active_dialog_mut() {
                input.push(character);
            }
        }
        Action::BackspaceBbmask => {
            if let Some(Dialog::BbmaskEdit { input }) = app.active_dialog_mut() {
                input.pop();
            }
        }
        Action::PreviewBbmaskEdit => {
            if let Some(Dialog::BbmaskEdit { input }) = app.active_dialog() {
                if input.contains(['\n', '\r']) {
                    app.notification = Some("BBMASK must be entered on one line.".into());
                } else {
                    replace_dialog(app, Dialog::BbmaskConfirmation(input.clone()));
                }
            }
        }
        Action::CancelBbmaskEdit => {
            if matches!(app.active_dialog(), Some(Dialog::BbmaskEdit { .. })) {
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
        Action::HostTelemetryUpdated(telemetry) => app.host_telemetry = telemetry,
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
        for character in "core-image-minimal".chars() {
            let _ = update(&mut app, Action::AppendBuildTarget(character));
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
            Some(Dialog::BuildTarget { input, task })
                if input == "core-image-minimal" && task.as_deref() == Some("clean")
        ));
    }
    #[test]
    fn updates_host_telemetry() {
        let mut app = App::new(10, 1_000);
        let telemetry = HostTelemetry {
            cpu_utilization_percent: Some(42),
            disk_available_bytes: Some(8 * 1024 * 1024 * 1024),
        };
        let _ = update(&mut app, Action::HostTelemetryUpdated(telemetry.clone()));
        assert_eq!(app.host_telemetry, telemetry);
    }
    #[test]
    fn settings_selection_and_changes_are_typed_and_persisted() {
        let mut app = App::new(10, 1_000);
        assert_eq!(SETTINGS[app.settings_selection], Setting::Theme);
        assert_eq!(
            update(&mut app, Action::ChangeSelectedSetting { backwards: false }),
            Some(Effect::PersistSettings)
        );
        assert_eq!(app.theme, Theme::Light);
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
        assert_eq!(app.theme, Theme::Monochrome);

        let _ = update(
            &mut app,
            Action::SettingsPersistenceFailed("read-only filesystem".into()),
        );
        assert_eq!(app.theme, Theme::Monochrome);
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
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::BbmaskEdit {
                input: "meta-old/.*".into()
            })
        );
        let _ = update(&mut app, Action::AppendBbmask(' '));
        let _ = update(&mut app, Action::AppendBbmask('x'));
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
            Some(Dialog::ConfigEdit { identity: selected, input })
                if selected == &identity && input == "qemux86-64"
        ));
        let _ = update(&mut app, Action::AppendConfigEdit('"'));
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
}
