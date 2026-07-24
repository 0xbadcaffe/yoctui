//! Domain model and pure state transitions. BitBake remains authoritative.
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    path::PathBuf,
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
}
impl BuildRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.targets.is_empty()
            || self.targets.iter().any(|x| {
                x.is_empty()
                    || matches!(x.as_str(), "." | "..")
                    || !x
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'))
            })
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
pub struct DevtoolDeployRequest {
    pub recipe: String,
    pub target: String,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub version: Option<String>,
    pub layer: Option<String>,
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
pub enum Dialog {
    BuildOptions,
    BuildCompletion,
    BuildTarget { input: String, task: Option<String> },
    ImagePicker(ImagePicker),
    RecipeTaskConfirmation(BuildRequest),
    DevtoolResetConfirmation(String),
    DevtoolUpdateConfirmation(String),
    DevtoolFinish { recipe: String, destination: String },
    DevtoolFinishConfirmation(DevtoolFinishRequest),
    DevtoolDeploy { recipe: String, target: String },
    DevtoolDeployConfirmation(DevtoolDeployRequest),
    BbmaskEdit { input: String },
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
pub struct BackgroundJobOutputEntry {
    pub severity: Severity,
    pub message: String,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerBrowserEntry {
    pub path: PathBuf,
    pub is_dir: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerBrowser {
    pub layer: String,
    pub root: PathBuf,
    pub directory: PathBuf,
    pub entries: Vec<LayerBrowserEntry>,
    pub selection: usize,
    pub preview: String,
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
    pub layer_relationships: Option<LayerRelationships>,
    pub recipe_sources: HashMap<String, Vec<PathBuf>>,
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
            layer_relationships: None,
            recipe_sources: HashMap::new(),
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
    BeginSelectedRecipeDevtoolModify,
    BeginSelectedRecipeDevtoolReset,
    BeginSelectedRecipeDevtoolUpdateRecipe,
    BeginSelectedRecipeDevtoolFinish,
    BeginSelectedRecipeDevtoolDeploy,
    BeginSelectedRecipeDependencies,
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
    CloseRecipeEditor,
    ConfirmRecipeTask,
    CancelRecipeTask,
    ConfirmDevtoolReset,
    CancelDevtoolReset,
    ConfirmDevtoolUpdateRecipe,
    CancelDevtoolUpdateRecipe,
    AppendDevtoolFinishDestination(char),
    BackspaceDevtoolFinishDestination,
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
    LayerBrowserEnter,
    LayerBrowserUp,
    RefreshLayerBrowser,
    LoadLayerBrowserPreview(String),
    EditSelectedLayerBrowserFile,
    BeginLayerRelationships,
    LayerRelationshipsLoaded(LayerRelationships),
    SelectConfigVariable {
        delta: isize,
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
    VariableLoaded {
        name: String,
        value: Option<String>,
        provenance: Option<String>,
    },
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
            app.recipe_selection = if delta.is_negative() {
                app.recipe_selection.saturating_sub(delta.unsigned_abs())
            } else {
                app.recipe_selection
                    .saturating_add(delta as usize)
                    .min(app.workspace.recipes.len().saturating_sub(1))
            };
        }
        Action::BeginSelectedRecipeBuild => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                open_dialog(
                    app,
                    Dialog::RecipeTaskConfirmation(BuildRequest {
                        targets: vec![recipe.name.clone()],
                        task: None,
                    }),
                );
            } else {
                app.notification = Some("No recipe is selected to build.".into());
            }
        }
        Action::BeginSelectedRecipeClean => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                open_dialog(
                    app,
                    Dialog::BuildTarget {
                        input: recipe.name.clone(),
                        task: Some("clean".into()),
                    },
                );
            } else {
                app.notification = Some("No recipe is selected to clean.".into());
            }
        }
        Action::BeginSelectedRecipeMenuConfig => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                open_dialog(
                    app,
                    Dialog::BuildTarget {
                        input: recipe.name.clone(),
                        task: Some("menuconfig".into()),
                    },
                );
            } else {
                app.notification = Some("No recipe is selected for menuconfig.".into());
            }
        }
        Action::BeginSelectedRecipeCleanState => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                open_dialog(
                    app,
                    Dialog::RecipeTaskConfirmation(BuildRequest {
                        targets: vec![recipe.name.clone()],
                        task: Some("cleansstate".into()),
                    }),
                );
            } else {
                app.notification = Some("No recipe is selected to clean state.".into());
            }
        }
        Action::BeginSelectedRecipeDevtoolModify => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                let request = BuildRequest {
                    targets: vec![recipe.name.clone()],
                    task: None,
                };
                if let Err(error) = request.validate() {
                    app.notification = Some(error.to_string());
                } else {
                    return Some(Effect::DevtoolModify(recipe.name.clone()));
                }
            } else {
                app.notification = Some("No recipe is selected for devtool modification.".into());
            }
        }
        Action::BeginSelectedRecipeDevtoolReset => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                let request = BuildRequest {
                    targets: vec![recipe.name.clone()],
                    task: None,
                };
                if let Err(error) = request.validate() {
                    app.notification = Some(error.to_string());
                } else {
                    open_dialog(app, Dialog::DevtoolResetConfirmation(recipe.name.clone()));
                }
            } else {
                app.notification = Some("No recipe is selected for devtool reset.".into());
            }
        }
        Action::BeginSelectedRecipeDevtoolUpdateRecipe => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                open_dialog(app, Dialog::DevtoolUpdateConfirmation(recipe.name.clone()));
            } else {
                app.notification = Some("No recipe is selected for devtool update-recipe.".into());
            }
        }
        Action::BeginSelectedRecipeDevtoolFinish => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected for devtool finish.".into());
                return None;
            };
            let recipe_name = recipe.name.clone();
            let layer_name = recipe.layer.clone();
            let destination = layer_name
                .as_deref()
                .and_then(|layer| {
                    app.workspace
                        .layers
                        .iter()
                        .find(|candidate| candidate.name == layer)
                })
                .map_or_else(String::new, |layer| layer.path.display().to_string());
            open_dialog(
                app,
                Dialog::DevtoolFinish {
                    recipe: recipe_name,
                    destination,
                },
            );
        }
        Action::BeginSelectedRecipeDevtoolDeploy => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                open_dialog(
                    app,
                    Dialog::DevtoolDeploy {
                        recipe: recipe.name.clone(),
                        target: String::new(),
                    },
                );
            } else {
                app.notification = Some("No recipe is selected for devtool deploy-target.".into());
            }
        }
        Action::BeginSelectedRecipeDependencies => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                return Some(Effect::GetDependencies(recipe.name.clone()));
            }
            app.notification = Some("No recipe is selected for dependency inspection.".into());
        }
        Action::DependenciesLoaded(dependencies) => {
            app.screen = Screen::Dependencies;
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
        Action::ConfirmDevtoolReset => {
            if let Some(Dialog::DevtoolResetConfirmation(recipe)) = app.active_dialog().cloned() {
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolReset(recipe));
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
            if let Some(Dialog::DevtoolUpdateConfirmation(recipe)) = app.active_dialog().cloned() {
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolUpdateRecipe(recipe));
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
        Action::AppendDevtoolFinishDestination(character) => {
            if let Some(Dialog::DevtoolFinish { destination, .. }) = app.active_dialog_mut() {
                destination.push(character);
            }
        }
        Action::BackspaceDevtoolFinishDestination => {
            if let Some(Dialog::DevtoolFinish { destination, .. }) = app.active_dialog_mut() {
                destination.pop();
            }
        }
        Action::PreviewDevtoolFinish => {
            if let Some(Dialog::DevtoolFinish {
                recipe,
                destination,
            }) = app.active_dialog()
            {
                let recipe = recipe.clone();
                let destination = destination.trim().to_owned();
                if destination.is_empty() {
                    app.notification =
                        Some("Enter a destination layer directory for devtool finish.".into());
                } else {
                    replace_dialog(
                        app,
                        Dialog::DevtoolFinishConfirmation(DevtoolFinishRequest {
                            recipe,
                            destination: PathBuf::from(destination),
                        }),
                    );
                }
            }
        }
        Action::CancelDevtoolFinish => {
            if matches!(app.active_dialog(), Some(Dialog::DevtoolFinish { .. })) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolFinish => {
            if let Some(Dialog::DevtoolFinishConfirmation(request)) = app.active_dialog().cloned() {
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolFinish(request));
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
            if let Some(Dialog::DevtoolDeploy { target, .. }) = app.active_dialog_mut() {
                target.push(character);
            }
        }
        Action::BackspaceDevtoolDeployTarget => {
            if let Some(Dialog::DevtoolDeploy { target, .. }) = app.active_dialog_mut() {
                target.pop();
            }
        }
        Action::PreviewDevtoolDeploy => {
            if let Some(Dialog::DevtoolDeploy { recipe, target }) = app.active_dialog() {
                let recipe = recipe.clone();
                let target = target.trim().to_owned();
                if target.is_empty() || target.contains(char::is_whitespace) {
                    app.notification =
                        Some("Enter one deployment target without whitespace.".into());
                } else {
                    replace_dialog(
                        app,
                        Dialog::DevtoolDeployConfirmation(DevtoolDeployRequest { recipe, target }),
                    );
                }
            }
        }
        Action::CancelDevtoolDeploy => {
            if matches!(app.active_dialog(), Some(Dialog::DevtoolDeploy { .. })) {
                close_dialog(app);
            }
        }
        Action::ConfirmDevtoolDeploy => {
            if let Some(Dialog::DevtoolDeployConfirmation(request)) = app.active_dialog().cloned() {
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolDeploy(request));
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
            entries,
        } => {
            app.layer_browser = Some(LayerBrowser {
                layer,
                root,
                directory,
                entries,
                selection: 0,
                preview: String::new(),
            });
            if let Some(path) = app
                .layer_browser
                .as_ref()
                .and_then(|browser| browser.entries.first())
                .filter(|entry| !entry.is_dir)
                .map(|entry| entry.path.clone())
            {
                return Some(Effect::LoadLayerBrowserPreview(path));
            }
        }
        Action::SelectLayerBrowserEntry { delta } => {
            let path = if let Some(browser) = app.layer_browser.as_mut() {
                browser.selection = if delta.is_negative() {
                    browser.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    browser
                        .selection
                        .saturating_add(delta as usize)
                        .min(browser.entries.len().saturating_sub(1))
                };
                browser
                    .entries
                    .get(browser.selection)
                    .filter(|entry| !entry.is_dir)
                    .map(|entry| entry.path.clone())
            } else {
                None
            };
            if let Some(path) = path {
                return Some(Effect::LoadLayerBrowserPreview(path));
            }
        }
        Action::LayerBrowserEnter => {
            if let Some(browser) = app.layer_browser.as_ref()
                && let Some(entry) = browser.entries.get(browser.selection)
                && entry.is_dir
            {
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    directory: entry.path.clone(),
                });
            }
        }
        Action::LayerBrowserUp => {
            if let Some(browser) = app.layer_browser.as_ref()
                && browser.directory != browser.root
            {
                let directory = browser
                    .directory
                    .parent()
                    .unwrap_or(&browser.root)
                    .to_path_buf();
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    directory,
                });
            }
            app.layer_browser = None;
        }
        Action::RefreshLayerBrowser => {
            if let Some(browser) = app.layer_browser.as_ref() {
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    directory: browser.directory.clone(),
                });
            }
        }
        Action::LoadLayerBrowserPreview(preview) => {
            if let Some(browser) = app.layer_browser.as_mut() {
                browser.preview = preview;
            }
        }
        Action::EditSelectedLayerBrowserFile => {
            if let Some(browser) = app.layer_browser.as_ref()
                && let Some(entry) = browser.entries.get(browser.selection)
                && !entry.is_dir
                && let Some(name) = entry.path.file_name()
            {
                return Some(Effect::OpenLayerBrowserEditor {
                    layer: browser.layer.clone(),
                    root: browser.directory.clone(),
                    file: PathBuf::from(name),
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
            app.config_selection = if delta.is_negative() {
                app.config_selection.saturating_sub(delta.unsigned_abs())
            } else {
                app.config_selection
                    .saturating_add(delta as usize)
                    .min(app.workspace.variables.len().saturating_sub(1))
            };
        }
        Action::OpenSelectedConfigSource => {
            let mut variables = app.workspace.variables.iter().collect::<Vec<_>>();
            variables.sort_by_key(|(name, _)| *name);
            let Some((name, _)) = variables.get(app.config_selection) else {
                app.notification = Some("No configuration variable is selected to open.".into());
                return None;
            };
            let Some(provenance) = app.workspace.variable_provenance.get(*name) else {
                app.notification =
                    Some("The selected variable has no file-backed provenance.".into());
                return None;
            };
            let source = provenance
                .rsplit_once(':')
                .filter(|(_, line)| line.chars().all(|character| character.is_ascii_digit()))
                .map_or(provenance.as_str(), |(path, _)| path);
            let path = PathBuf::from(source);
            if path.as_os_str().is_empty() {
                app.notification =
                    Some("The selected variable has no file-backed provenance.".into());
            } else {
                let path = if path.is_relative() {
                    app.workspace
                        .build_dir
                        .as_ref()
                        .map_or(path.clone(), |build_dir| build_dir.join(path))
                } else {
                    path
                };
                return Some(Effect::OpenInEditor(path));
            }
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
        }
        Action::BackspaceMetadataQuery if app.metadata_searching => {
            app.metadata_query.pop();
            app.recipe_selection = 0;
            app.layer_selection = 0;
            app.config_selection = 0;
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
        Action::WorkspaceLoaded(w) => app.workspace = w,
        Action::RecipesLoaded(mut recipes) => {
            recipes.sort_by(|left, right| left.name.cmp(&right.name));
            app.workspace.recipes = recipes;
            app.recipe_selection = app
                .recipe_selection
                .min(app.workspace.recipes.len().saturating_sub(1));
        }
        Action::LayersLoaded(mut layers) => {
            layers.sort_by(|left, right| left.name.cmp(&right.name));
            app.workspace.layers = layers;
            app.layer_selection = app
                .layer_selection
                .min(app.workspace.layers.len().saturating_sub(1));
        }
        Action::VariableLoaded {
            name,
            value,
            provenance,
        } => {
            if let Some(value) = value {
                app.workspace.variables.insert(name.clone(), value);
            } else {
                app.workspace.variables.remove(&name);
            }
            if let Some(provenance) = provenance {
                app.workspace.variable_provenance.insert(name, provenance);
            } else {
                app.workspace.variable_provenance.remove(&name);
            }
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
    DevtoolModify(String),
    DevtoolReset(String),
    DevtoolUpdateRecipe(String),
    DevtoolFinish(DevtoolFinishRequest),
    DevtoolDeploy(DevtoolDeployRequest),
    GetDependencies(String),
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
                }),
            )
            .is_none()
        );
        assert!(app.notification.is_some());
        let request = BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("compile".into()),
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
            },
            Recipe {
                name: "beta".into(),
                version: None,
                layer: None,
            },
        ];
        let _ = update(&mut app, Action::SelectRecipe { delta: 8 });
        assert_eq!(app.recipe_selection, 1);
        let _ = update(&mut app, Action::SelectRecipe { delta: -8 });
        assert_eq!(app.recipe_selection, 0);
    }
    #[test]
    fn selected_recipe_build_requires_confirmation() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
        }];
        let _ = update(&mut app, Action::BeginSelectedRecipeBuild);
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["busybox".into()],
                task: None,
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
        }];
        let _ = update(&mut app, Action::BeginSelectedRecipeClean);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::BuildTarget { input, task })
                if input == "busybox" && task.as_deref() == Some("clean")
        ));
    }
    #[test]
    fn selected_recipe_menuconfig_prefills_the_menuconfig_task() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
        }];
        let _ = update(&mut app, Action::BeginSelectedRecipeMenuConfig);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::BuildTarget { input, task })
                if input == "busybox" && task.as_deref() == Some("menuconfig")
        ));
    }
    #[test]
    fn selected_recipe_requests_devtool_modification() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
        }];
        assert_eq!(
            update(&mut app, Action::BeginSelectedRecipeDevtoolModify),
            Some(Effect::DevtoolModify("busybox".into()))
        );
    }
    #[test]
    fn selected_recipe_requests_authoritative_dependencies() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
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
        });
        let _ = update(&mut app, Action::SelectDependency { delta: 1 });
        let _ = update(&mut app, Action::OpenSelectedDependency);
        assert_eq!(app.screen, Screen::Recipes);
        assert_eq!(app.recipe_selection, 1);
    }
    #[test]
    fn selected_recipe_requires_confirmation_before_devtool_reset() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
        }];
        assert_eq!(
            update(&mut app, Action::BeginSelectedRecipeDevtoolReset),
            None
        );
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::DevtoolResetConfirmation("busybox".into()))
        );
        assert_eq!(
            update(&mut app, Action::ConfirmDevtoolReset),
            Some(Effect::DevtoolReset("busybox".into()))
        );
    }
    #[test]
    fn selected_recipe_requires_confirmation_before_devtool_update_recipe() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
        }];
        assert_eq!(
            update(&mut app, Action::BeginSelectedRecipeDevtoolUpdateRecipe),
            None
        );
        assert_eq!(
            app.active_dialog(),
            Some(&Dialog::DevtoolUpdateConfirmation("busybox".into()))
        );
        assert_eq!(
            update(&mut app, Action::ConfirmDevtoolUpdateRecipe),
            Some(Effect::DevtoolUpdateRecipe("busybox".into()))
        );
    }
    #[test]
    fn devtool_finish_prefills_the_selected_recipe_layer_and_requires_confirmation() {
        let mut app = App::new(10, 1_000);
        app.workspace.layers = vec![Layer {
            name: "meta-demo".into(),
            path: PathBuf::from("/layers/meta-demo"),
            priority: None,
        }];
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: Some("meta-demo".into()),
        }];
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolFinish);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::DevtoolFinish {
                recipe,
                destination
            }) if recipe == "busybox" && destination == "/layers/meta-demo"
        ));
        let _ = update(&mut app, Action::PreviewDevtoolFinish);
        assert_eq!(
            update(&mut app, Action::ConfirmDevtoolFinish),
            Some(Effect::DevtoolFinish(DevtoolFinishRequest {
                recipe: "busybox".into(),
                destination: PathBuf::from("/layers/meta-demo"),
            }))
        );
    }
    #[test]
    fn devtool_deploy_requires_a_target_and_confirmation() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
        }];
        let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolDeploy);
        let _ = update(&mut app, Action::AppendDevtoolDeployTarget('q'));
        let _ = update(&mut app, Action::AppendDevtoolDeployTarget('e'));
        let _ = update(&mut app, Action::AppendDevtoolDeployTarget('m'));
        let _ = update(&mut app, Action::AppendDevtoolDeployTarget('u'));
        let _ = update(&mut app, Action::PreviewDevtoolDeploy);
        assert_eq!(
            update(&mut app, Action::ConfirmDevtoolDeploy),
            Some(Effect::DevtoolDeploy(DevtoolDeployRequest {
                recipe: "busybox".into(),
                target: "qemu".into(),
            }))
        );
    }
    #[test]
    fn recipe_editor_loads_edits_and_saves_selected_file() {
        let mut app = App::new(10, 1_000);
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
        assert_eq!(
            update(&mut app, Action::SaveRecipeEditor),
            Some(Effect::SaveRecipeEditorFile {
                path: root.join("main.c"),
                content: "int main() {}\n".into(),
            })
        );
    }
    #[test]
    fn clean_state_requires_confirmation_before_starting() {
        let mut app = App::new(10, 1_000);
        app.workspace.recipes = vec![Recipe {
            name: "busybox".into(),
            version: None,
            layer: None,
        }];
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
    fn layer_browser_descends_and_returns_to_the_parent_directory() {
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
                entries: vec![],
            },
        );
        assert_eq!(
            update(&mut app, Action::LayerBrowserUp),
            Some(Effect::LoadLayerBrowserDirectory {
                layer: "meta-demo".into(),
                root: "/layers/meta-demo".into(),
                directory: "/layers/meta-demo".into(),
            })
        );
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
    fn selected_configuration_source_opens_relative_provenance_path() {
        let mut app = App::new(10, 1_000);
        app.workspace.build_dir = Some(PathBuf::from("/build"));
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemuarm".into());
        app.workspace
            .variable_provenance
            .insert("MACHINE".into(), "conf/local.conf:12".into());
        assert_eq!(
            update(&mut app, Action::OpenSelectedConfigSource),
            Some(Effect::OpenInEditor(PathBuf::from(
                "/build/conf/local.conf"
            )))
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
                },
                Recipe {
                    name: "base-files".into(),
                    version: None,
                    layer: Some("core".into()),
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
            Action::VariableLoaded {
                name: "MACHINE".into(),
                value: Some("qemux86-64".into()),
                provenance: Some("/build/conf/local.conf:1".into()),
            },
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
                task: None
            }
            .validate()
            .is_ok()
        );
        assert!(
            BuildRequest {
                targets: vec!["bad target".into()],
                task: None
            }
            .validate()
            .is_err()
        );
        assert!(
            BuildRequest {
                targets: vec!["..".into()],
                task: None
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
}
