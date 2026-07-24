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
    BuildHistory,
    Dependencies,
    LayerRelationships,
    Recipes,
    Layers,
    Configuration,
    Bbmask,
    Logs,
    Errors,
    Help,
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
const NAVIGATOR_SCREENS: [Screen; 11] = [
    Screen::Dashboard,
    Screen::Layers,
    Screen::Recipes,
    Screen::Dashboard,
    Screen::Logs,
    Screen::Errors,
    Screen::Configuration,
    Screen::Dependencies,
    Screen::Recipes,
    Screen::Bbmask,
    Screen::Help,
];
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildStatus {
    Idle,
    LoadingWorkspace,
    Parsing,
    Running,
    Cancelling,
    Completed,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: TaskId,
    pub recipe: String,
    pub task: String,
    pub progress: Option<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedTask {
    pub task: TaskInfo,
    pub success: bool,
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
pub struct LogEntry {
    pub severity: Severity,
    pub message: String,
    pub recipe: Option<String>,
    pub task: Option<String>,
    pub path: Option<PathBuf>,
    pub timestamp: SystemTime,
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
    pub follow: bool,
    pub paused_len: Option<usize>,
    pub wrap: bool,
    pub filter: Option<Severity>,
    pub recipe_filter: Option<String>,
    pub task_filter: Option<String>,
    pub query: String,
    pub searching: bool,
    pub scroll_offset: usize,
    pub horizontal_offset: usize,
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
            follow: true,
            paused_len: None,
            wrap: false,
            filter: None,
            recipe_filter: None,
            task_filter: None,
            query: String::new(),
            searching: false,
            scroll_offset: 0,
            horizontal_offset: 0,
        }
    }
    pub fn insert(&mut self, entry: LogEntry) {
        let bytes = entry.message.len();
        self.retained_bytes += bytes;
        self.entries.push_back(entry);
        while self.entries.len() > self.max_entries || self.retained_bytes > self.max_bytes {
            if let Some(old) = self.entries.pop_front() {
                self.retained_bytes = self.retained_bytes.saturating_sub(old.message.len());
                self.dropped += 1;
                match old.severity {
                    Severity::Warning => self.dropped_warnings += 1,
                    Severity::Error => self.dropped_errors += 1,
                    Severity::Trace | Severity::Info => {}
                }
            } else {
                break;
            }
        }
    }
    pub fn filtered(&self) -> impl Iterator<Item = &LogEntry> {
        let query = self.query.to_lowercase();
        let visible_len = self.paused_len.unwrap_or(self.entries.len());
        self.entries.iter().take(visible_len).filter(move |e| {
            self.filter.is_none_or(|s| s == e.severity)
                && self
                    .recipe_filter
                    .as_ref()
                    .is_none_or(|recipe| e.recipe.as_ref() == Some(recipe))
                && self
                    .task_filter
                    .as_ref()
                    .is_none_or(|task| e.task.as_ref() == Some(task))
                && (query.is_empty() || e.message.to_lowercase().contains(&query))
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub screen: Screen,
    pub focus: FocusTarget,
    pub navigator_selection: usize,
    pub backend: String,
    pub color_enabled: bool,
    pub workspace: Workspace,
    pub host_telemetry: HostTelemetry,
    pub build: BuildState,
    pub build_history: VecDeque<BuildRecord>,
    pub build_history_selection: usize,
    pub dependencies: Option<RecipeDependencies>,
    pub dependency_selection: usize,
    pub layer_relationships: Option<LayerRelationships>,
    pub layer_browser: Option<LayerBrowser>,
    pub image_picker: Option<ImagePicker>,
    pub tasks: HashMap<TaskId, TaskInfo>,
    pub completed_tasks: VecDeque<CompletedTask>,
    pub task_progress_scroll: usize,
    pub logs: LogState,
    pub should_quit: bool,
    pub quit_confirm: bool,
    pub notification: Option<String>,
    pub build_options_open: bool,
    pub build_completion_open: bool,
    pub build_target_editing: bool,
    pub build_target_input: String,
    pub build_task: Option<String>,
    pub recipe_task_confirmation: Option<BuildRequest>,
    pub devtool_reset_confirmation: Option<String>,
    pub devtool_update_confirmation: Option<String>,
    pub devtool_finish_recipe: Option<String>,
    pub devtool_finish_destination: String,
    pub devtool_finish_confirmation: Option<DevtoolFinishRequest>,
    pub devtool_deploy_recipe: Option<String>,
    pub devtool_deploy_target: String,
    pub devtool_deploy_confirmation: Option<DevtoolDeployRequest>,
    pub bbmask_editing: bool,
    pub bbmask_input: String,
    pub bbmask_confirmation: Option<String>,
    pub recipe_editor: Option<RecipeEditor>,
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
            navigator_selection: 0,
            backend: "unknown".into(),
            color_enabled: true,
            workspace: Workspace::default(),
            host_telemetry: HostTelemetry::default(),
            build: BuildState::default(),
            build_history: VecDeque::new(),
            build_history_selection: 0,
            dependencies: None,
            dependency_selection: 0,
            layer_relationships: None,
            layer_browser: None,
            image_picker: None,
            tasks: HashMap::new(),
            completed_tasks: VecDeque::new(),
            task_progress_scroll: 0,
            logs: LogState::new(max_entries, max_bytes),
            should_quit: false,
            quit_confirm: false,
            notification: None,
            build_options_open: false,
            build_completion_open: false,
            build_target_editing: false,
            build_target_input: String::new(),
            build_task: None,
            recipe_task_confirmation: None,
            devtool_reset_confirmation: None,
            devtool_update_confirmation: None,
            devtool_finish_recipe: None,
            devtool_finish_destination: String::new(),
            devtool_finish_confirmation: None,
            devtool_deploy_recipe: None,
            devtool_deploy_target: String::new(),
            devtool_deploy_confirmation: None,
            bbmask_editing: false,
            bbmask_input: String::new(),
            bbmask_confirmation: None,
            recipe_editor: None,
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
    BuildStarted,
    ParseProgress {
        current: Option<u64>,
        total: Option<u64>,
    },
    TaskStarted(TaskInfo),
    TaskProgress {
        id: TaskId,
        progress: u8,
    },
    TaskCompleted {
        id: TaskId,
        success: bool,
    },
    ScrollBuildTasks {
        delta: isize,
    },
    Log(LogEntry),
    BuildCompleted {
        success: bool,
        exit_code: Option<i32>,
    },
    DismissBuildCompletion,
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
    DismissNotification,
    Quit,
    ConfirmQuit,
    WorkspaceLoaded(Workspace),
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
    app.build_completion_open = false;
    app.tasks.clear();
    app.completed_tasks.clear();
    app.task_progress_scroll = 0;
}

pub fn update(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::Open(s) => {
            app.screen = s;
            app.focus = FocusTarget::Workspace;
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
        }
        Action::Focus(target) => app.focus = target,
        Action::CycleFocus { backwards } => {
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
        Action::OpenBuildOptions => app.build_options_open = true,
        Action::CloseBuildOptions => app.build_options_open = false,
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
                app.image_picker = Some(ImagePicker { images, selection });
            }
        }
        Action::SelectImage { delta } => {
            if let Some(picker) = app.image_picker.as_mut() {
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
            if let Some(picker) = app.image_picker.take()
                && let Some(image) = picker.images.get(picker.selection)
            {
                app.build.target = Some(image.clone());
            }
        }
        Action::CancelImagePicker => app.image_picker = None,
        Action::BeginCurrentImageBuild => {
            if app
                .recipe_editor
                .as_ref()
                .is_some_and(|editor| editor.dirty)
            {
                app.notification = Some("Save the edited file with Ctrl+S before building.".into());
            } else if let Some(target) = app.build.target.clone() {
                app.recipe_editor = None;
                app.recipe_task_confirmation = Some(BuildRequest {
                    targets: vec![target],
                    task: None,
                });
            } else {
                app.notification = Some("Select an image first with i.".into());
            }
        }
        Action::BeginBuildTargetEdit => {
            app.build_target_input = app.build.target.clone().unwrap_or_default();
            app.build_task = None;
            app.build_options_open = false;
            app.build_target_editing = true;
        }
        Action::BeginBuildTargetTask(task) => {
            app.build_target_input = app.build.target.clone().unwrap_or_default();
            app.build_task = task;
            app.build_options_open = false;
            app.build_target_editing = true;
        }
        Action::AppendBuildTarget(character) if app.build_target_editing => {
            app.build_target_input.push(character);
        }
        Action::BackspaceBuildTarget if app.build_target_editing => {
            app.build_target_input.pop();
        }
        Action::CancelBuildTargetEdit => app.build_target_editing = false,
        Action::ConfirmBuildTarget if app.build_target_editing => {
            let request = BuildRequest {
                targets: vec![app.build_target_input.clone()],
                task: app.build_task.clone(),
            };
            if let Err(error) = request.validate() {
                app.notification = Some(error.to_string());
            } else {
                app.build_target_editing = false;
                app.recipe_task_confirmation = Some(request);
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
            app.tasks.insert(t.id.clone(), t);
        }
        Action::TaskProgress { id, progress } => {
            if let Some(t) = app.tasks.get_mut(&id) {
                t.progress = Some(progress)
            }
        }
        Action::TaskCompleted { id, success } => {
            if let Some(mut task) = app.tasks.remove(&id) {
                task.progress = Some(100);
                app.completed_tasks
                    .push_back(CompletedTask { task, success });
                if app.completed_tasks.len() > MAX_COMPLETED_TASKS {
                    app.completed_tasks.pop_front();
                }
                app.build.completed += 1;
            }
        }
        Action::ScrollBuildTasks { delta } => {
            let task_count = app.tasks.len() + app.completed_tasks.len();
            app.task_progress_scroll = if delta.is_negative() {
                app.task_progress_scroll
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.task_progress_scroll
                    .saturating_add(delta as usize)
                    .min(task_count.saturating_sub(1))
            };
        }
        Action::Log(l) => {
            match l.severity {
                Severity::Warning => app.build.warnings += 1,
                Severity::Error => app.build.errors += 1,
                _ => {}
            }
            app.logs.insert(l);
            if app.logs.follow {
                app.logs.scroll_offset = 0;
            }
        }
        Action::BuildCompleted { success, exit_code } => {
            app.build.status = if success {
                BuildStatus::Completed
            } else {
                BuildStatus::Failed
            };
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
            app.build_completion_open = true;
        }
        Action::DismissBuildCompletion => app.build_completion_open = false,
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
                return Some(Effect::Cancel);
            }
        }
        Action::ToggleLogFollow => {
            app.logs.follow = !app.logs.follow;
            app.logs.paused_len = (!app.logs.follow).then_some(app.logs.entries.len());
        }
        Action::ToggleLogWrap => app.logs.wrap = !app.logs.wrap,
        Action::CycleLogSeverity => {
            app.logs.filter = match app.logs.filter {
                None => Some(Severity::Info),
                Some(Severity::Info) => Some(Severity::Warning),
                Some(Severity::Warning) => Some(Severity::Error),
                Some(Severity::Error) | Some(Severity::Trace) => None,
            };
        }
        Action::ScrollLogs { delta } => {
            app.logs.follow = false;
            app.logs.paused_len = Some(app.logs.entries.len());
            app.logs.scroll_offset = if delta.is_negative() {
                app.logs.scroll_offset.saturating_sub(delta.unsigned_abs())
            } else {
                app.logs
                    .scroll_offset
                    .saturating_add(delta as usize)
                    .min(app.logs.entries.len())
            };
        }
        Action::BeginLogSearch => {
            app.logs.searching = true;
            app.logs.follow = false;
        }
        Action::AppendLogQuery(character) if app.logs.searching => app.logs.query.push(character),
        Action::BackspaceLogQuery if app.logs.searching => {
            app.logs.query.pop();
        }
        Action::FinishLogSearch => app.logs.searching = false,
        Action::NextLogMatch if !app.logs.query.is_empty() => {
            let count = app.logs.filtered().count();
            app.logs.follow = false;
            app.logs.paused_len = Some(app.logs.entries.len());
            app.logs.scroll_offset = app
                .logs
                .scroll_offset
                .saturating_add(1)
                .min(count.saturating_sub(1));
        }
        Action::PreviousLogMatch if !app.logs.query.is_empty() => {
            app.logs.follow = false;
            app.logs.paused_len = Some(app.logs.entries.len());
            app.logs.scroll_offset = app.logs.scroll_offset.saturating_sub(1);
        }
        Action::ScrollLogsHorizontally { delta } => {
            app.logs.horizontal_offset = if delta.is_negative() {
                app.logs
                    .horizontal_offset
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.logs.horizontal_offset.saturating_add(delta as usize)
            };
        }
        Action::CycleLogRecipeFilter => {
            let mut values = app
                .logs
                .entries
                .iter()
                .filter_map(|entry| entry.recipe.clone())
                .collect::<Vec<_>>();
            values.sort();
            values.dedup();
            app.logs.recipe_filter = next_filter(&values, app.logs.recipe_filter.take());
        }
        Action::CycleLogTaskFilter => {
            let mut values = app
                .logs
                .entries
                .iter()
                .filter_map(|entry| entry.task.clone())
                .collect::<Vec<_>>();
            values.sort();
            values.dedup();
            app.logs.task_filter = next_filter(&values, app.logs.task_filter.take());
        }
        Action::SelectError { delta } => {
            let count = app
                .logs
                .entries
                .iter()
                .filter(|entry| matches!(entry.severity, Severity::Warning | Severity::Error))
                .count();
            app.error_selection = if delta.is_negative() {
                app.error_selection.saturating_sub(delta.unsigned_abs())
            } else {
                app.error_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::JumpToSelectedError => {
            if let Some(entry) = app
                .logs
                .entries
                .iter()
                .filter(|entry| matches!(entry.severity, Severity::Warning | Severity::Error))
                .nth(app.error_selection)
            {
                app.logs.query = entry.message.clone();
                app.logs.filter = Some(entry.severity);
                app.logs.follow = false;
                app.screen = Screen::Logs;
            }
        }
        Action::OpenSelectedErrorSource => {
            let selected = app
                .logs
                .entries
                .iter()
                .filter(|entry| matches!(entry.severity, Severity::Warning | Severity::Error))
                .nth(app.error_selection);
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
                app.recipe_task_confirmation = Some(BuildRequest {
                    targets: vec![recipe.name.clone()],
                    task: None,
                });
            } else {
                app.notification = Some("No recipe is selected to build.".into());
            }
        }
        Action::BeginSelectedRecipeClean => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                app.build_target_input = recipe.name.clone();
                app.build_task = Some("clean".into());
                app.build_target_editing = true;
            } else {
                app.notification = Some("No recipe is selected to clean.".into());
            }
        }
        Action::BeginSelectedRecipeMenuConfig => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                app.build_target_input = recipe.name.clone();
                app.build_task = Some("menuconfig".into());
                app.build_target_editing = true;
            } else {
                app.notification = Some("No recipe is selected for menuconfig.".into());
            }
        }
        Action::BeginSelectedRecipeCleanState => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                app.recipe_task_confirmation = Some(BuildRequest {
                    targets: vec![recipe.name.clone()],
                    task: Some("cleansstate".into()),
                });
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
                    app.devtool_reset_confirmation = Some(recipe.name.clone());
                }
            } else {
                app.notification = Some("No recipe is selected for devtool reset.".into());
            }
        }
        Action::BeginSelectedRecipeDevtoolUpdateRecipe => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                app.devtool_update_confirmation = Some(recipe.name.clone());
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
            app.devtool_finish_recipe = Some(recipe_name);
            app.devtool_finish_destination = layer_name
                .as_deref()
                .and_then(|layer| {
                    app.workspace
                        .layers
                        .iter()
                        .find(|candidate| candidate.name == layer)
                })
                .map_or_else(String::new, |layer| layer.path.display().to_string());
        }
        Action::BeginSelectedRecipeDevtoolDeploy => {
            if let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) {
                app.devtool_deploy_recipe = Some(recipe.name.clone());
                app.devtool_deploy_target.clear();
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
            if let Some(request) = app.recipe_task_confirmation.take() {
                prepare_build(app, request.targets.first().cloned());
                return Some(Effect::Start(request));
            }
        }
        Action::CancelRecipeTask => app.recipe_task_confirmation = None,
        Action::ConfirmDevtoolReset => {
            if let Some(recipe) = app.devtool_reset_confirmation.take() {
                return Some(Effect::DevtoolReset(recipe));
            }
        }
        Action::CancelDevtoolReset => app.devtool_reset_confirmation = None,
        Action::ConfirmDevtoolUpdateRecipe => {
            if let Some(recipe) = app.devtool_update_confirmation.take() {
                return Some(Effect::DevtoolUpdateRecipe(recipe));
            }
        }
        Action::CancelDevtoolUpdateRecipe => app.devtool_update_confirmation = None,
        Action::AppendDevtoolFinishDestination(character)
            if app.devtool_finish_recipe.is_some() =>
        {
            app.devtool_finish_destination.push(character);
        }
        Action::BackspaceDevtoolFinishDestination if app.devtool_finish_recipe.is_some() => {
            app.devtool_finish_destination.pop();
        }
        Action::PreviewDevtoolFinish if app.devtool_finish_recipe.is_some() => {
            let destination = app.devtool_finish_destination.trim();
            if destination.is_empty() {
                app.notification =
                    Some("Enter a destination layer directory for devtool finish.".into());
            } else if let Some(recipe) = app.devtool_finish_recipe.take() {
                app.devtool_finish_confirmation = Some(DevtoolFinishRequest {
                    recipe,
                    destination: PathBuf::from(destination),
                });
            }
        }
        Action::CancelDevtoolFinish => {
            app.devtool_finish_recipe = None;
            app.devtool_finish_destination.clear();
        }
        Action::ConfirmDevtoolFinish => {
            if let Some(request) = app.devtool_finish_confirmation.take() {
                return Some(Effect::DevtoolFinish(request));
            }
        }
        Action::CancelDevtoolFinishConfirmation => app.devtool_finish_confirmation = None,
        Action::AppendDevtoolDeployTarget(character) if app.devtool_deploy_recipe.is_some() => {
            app.devtool_deploy_target.push(character);
        }
        Action::BackspaceDevtoolDeployTarget if app.devtool_deploy_recipe.is_some() => {
            app.devtool_deploy_target.pop();
        }
        Action::PreviewDevtoolDeploy if app.devtool_deploy_recipe.is_some() => {
            let target = app.devtool_deploy_target.trim();
            if target.is_empty() || target.contains(char::is_whitespace) {
                app.notification = Some("Enter one deployment target without whitespace.".into());
            } else if let Some(recipe) = app.devtool_deploy_recipe.take() {
                app.devtool_deploy_confirmation = Some(DevtoolDeployRequest {
                    recipe,
                    target: target.into(),
                });
            }
        }
        Action::CancelDevtoolDeploy => {
            app.devtool_deploy_recipe = None;
            app.devtool_deploy_target.clear();
        }
        Action::ConfirmDevtoolDeploy => {
            if let Some(request) = app.devtool_deploy_confirmation.take() {
                return Some(Effect::DevtoolDeploy(request));
            }
        }
        Action::CancelDevtoolDeployConfirmation => app.devtool_deploy_confirmation = None,
        Action::OpenRecipeEditor {
            recipe,
            root,
            files,
        } => {
            app.recipe_editor = Some(RecipeEditor {
                recipe,
                root,
                files,
                selection: 0,
                content: String::new(),
                editing: false,
                dirty: false,
            });
            if let Some(path) = app
                .recipe_editor
                .as_ref()
                .and_then(RecipeEditor::selected_path)
            {
                return Some(Effect::LoadRecipeEditorFile(path));
            }
            app.notification = Some("The Devtool workspace contains no editable files.".into());
        }
        Action::SelectRecipeEditorFile { delta } => {
            let path = if let Some(editor) = app.recipe_editor.as_mut() {
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
            if let Some(editor) = app.recipe_editor.as_mut() {
                editor.content = content;
                editor.editing = false;
                editor.dirty = false;
            }
        }
        Action::ToggleRecipeEditorEditing => {
            if let Some(editor) = app.recipe_editor.as_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendRecipeEditor(character) => {
            if let Some(editor) = app.recipe_editor.as_mut()
                && editor.editing
            {
                editor.content.push(character);
                editor.dirty = true;
            }
        }
        Action::BackspaceRecipeEditor => {
            if let Some(editor) = app.recipe_editor.as_mut()
                && editor.editing
            {
                editor.content.pop();
                editor.dirty = true;
            }
        }
        Action::SaveRecipeEditor => {
            if let Some(editor) = app.recipe_editor.as_ref()
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
            if let Some(editor) = app.recipe_editor.as_mut() {
                editor.dirty = false;
                app.notification = Some("Recipe file saved. Press Esc to return to Yoctui.".into());
            }
        }
        Action::CloseRecipeEditor => app.recipe_editor = None,
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
            app.bbmask_input = app
                .workspace
                .variables
                .get("BBMASK")
                .cloned()
                .unwrap_or_default();
            app.bbmask_editing = true;
        }
        Action::AppendBbmask(character) if app.bbmask_editing => app.bbmask_input.push(character),
        Action::BackspaceBbmask if app.bbmask_editing => {
            app.bbmask_input.pop();
        }
        Action::PreviewBbmaskEdit if app.bbmask_editing => {
            if app.bbmask_input.contains(['\n', '\r']) {
                app.notification = Some("BBMASK must be entered on one line.".into());
            } else {
                app.bbmask_confirmation = Some(app.bbmask_input.clone());
                app.bbmask_editing = false;
            }
        }
        Action::CancelBbmaskEdit => app.bbmask_editing = false,
        Action::ConfirmBbmaskWrite => {
            if let Some(value) = app.bbmask_confirmation.take() {
                return Some(Effect::WriteBbmask(value));
            }
        }
        Action::CancelBbmaskWrite => app.bbmask_confirmation = None,
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
        | Action::AppendBuildTarget(_)
        | Action::BackspaceBuildTarget
        | Action::ConfirmBuildTarget
        | Action::AppendMetadataQuery(_)
        | Action::BackspaceMetadataQuery
        | Action::AppendBbmask(_)
        | Action::BackspaceBbmask
        | Action::PreviewBbmaskEdit
        | Action::AppendDevtoolFinishDestination(_)
        | Action::BackspaceDevtoolFinishDestination
        | Action::PreviewDevtoolFinish
        | Action::AppendDevtoolDeployTarget(_)
        | Action::BackspaceDevtoolDeployTarget
        | Action::PreviewDevtoolDeploy => {}
        Action::DismissNotification => app.notification = None,
        Action::Quit => {
            if matches!(
                app.build.status,
                BuildStatus::Running | BuildStatus::Parsing | BuildStatus::Cancelling
            ) {
                app.quit_confirm = true
            } else {
                app.should_quit = true
            }
        }
        Action::ConfirmQuit => app.should_quit = true,
        Action::WorkspaceLoaded(w) => app.workspace = w,
        Action::HostTelemetryUpdated(telemetry) => app.host_telemetry = telemetry,
        Action::Failure(e) => {
            app.notification = Some(e.to_string());
            app.build.status = BuildStatus::Failed
        }
        Action::Tick => {}
    }
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
    Start(BuildRequest),
    Cancel,
    OpenInEditor(PathBuf),
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
            severity: Severity::Info,
            message: message.into(),
            recipe: None,
            task: None,
            path: None,
            timestamp: SystemTime::now(),
        }
    }
    fn tagged_log(recipe: &str, task: &str, severity: Severity, message: &str) -> LogEntry {
        LogEntry {
            severity,
            message: message.into(),
            recipe: Some(recipe.into()),
            task: Some(task.into()),
            path: None,
            timestamp: SystemTime::now(),
        }
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
        assert_eq!(app.screen, Screen::Help);
        assert_eq!(app.focus, FocusTarget::Workspace);
        let _ = update(&mut app, Action::CycleFocus { backwards: false });
        assert_eq!(app.focus, FocusTarget::Inspector);
        let _ = update(&mut app, Action::CycleFocus { backwards: true });
        assert_eq!(app.focus, FocusTarget::Workspace);
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
        assert_eq!(logs.dropped_errors, 1);
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
            }),
        );
        let _ = update(
            &mut app,
            Action::TaskProgress {
                id: id.clone(),
                progress: 50,
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
    fn selected_error_jumps_to_filtered_logs() {
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
        let _ = update(&mut app, Action::Open(Screen::Errors));
        let _ = update(&mut app, Action::JumpToSelectedError);
        assert_eq!(app.screen, Screen::Logs);
        assert_eq!(app.logs.query, "compile failed");
        assert_eq!(app.logs.filter, Some(Severity::Error));
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
            app.recipe_task_confirmation,
            Some(BuildRequest {
                targets: vec!["busybox".into()],
                task: None,
            })
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
        assert_eq!(app.build_target_input, "busybox");
        assert_eq!(app.build_task.as_deref(), Some("clean"));
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
        assert_eq!(app.build_target_input, "busybox");
        assert_eq!(app.build_task.as_deref(), Some("menuconfig"));
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
        assert_eq!(app.devtool_reset_confirmation.as_deref(), Some("busybox"));
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
        assert_eq!(app.devtool_update_confirmation.as_deref(), Some("busybox"));
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
        assert_eq!(app.devtool_finish_recipe.as_deref(), Some("busybox"));
        assert_eq!(app.devtool_finish_destination, "/layers/meta-demo");
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
        assert!(app.recipe_task_confirmation.is_some());
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
        assert_eq!(app.logs.scroll_offset, 1);
        assert!(!app.logs.follow);

        let _ = update(&mut app, Action::NextLogMatch);
        assert_eq!(app.logs.scroll_offset, 1);
        let _ = update(&mut app, Action::PreviousLogMatch);
        assert_eq!(app.logs.scroll_offset, 0);
    }
    #[test]
    fn build_target_editor_requires_confirmation_before_starting() {
        let mut app = App::new(10, 1_000);
        let _ = update(&mut app, Action::BeginBuildTargetEdit);
        for character in "core-image-minimal".chars() {
            let _ = update(&mut app, Action::AppendBuildTarget(character));
        }
        let effect = update(&mut app, Action::ConfirmBuildTarget);

        assert!(!app.build_target_editing);
        assert_eq!(effect, None);
        assert_eq!(
            app.recipe_task_confirmation,
            Some(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
            })
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
            app.recipe_task_confirmation,
            Some(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
            })
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
        assert!(app.build_completion_open);
        let _ = update(&mut app, Action::DismissBuildCompletion);
        assert!(!app.build_completion_open);
    }
    #[test]
    fn build_options_prefill_the_current_target_and_requested_task() {
        let mut app = App::new(10, 1_000);
        app.build.target = Some("core-image-minimal".into());

        let _ = update(&mut app, Action::OpenBuildOptions);
        assert!(app.build_options_open);
        let _ = update(&mut app, Action::BeginBuildTargetTask(Some("clean".into())));

        assert!(!app.build_options_open);
        assert!(app.build_target_editing);
        assert_eq!(app.build_target_input, "core-image-minimal");
        assert_eq!(app.build_task.as_deref(), Some("clean"));
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
    fn bbmask_editing_requires_a_preview_and_confirmation() {
        let mut app = App::new(10, 1_000);
        app.workspace
            .variables
            .insert("BBMASK".into(), "meta-old/.*".into());
        let _ = update(&mut app, Action::BeginBbmaskEdit);
        assert!(app.bbmask_editing);
        assert_eq!(app.bbmask_input, "meta-old/.*");
        let _ = update(&mut app, Action::AppendBbmask(' '));
        let _ = update(&mut app, Action::AppendBbmask('x'));
        let _ = update(&mut app, Action::PreviewBbmaskEdit);
        assert!(!app.bbmask_editing);
        assert_eq!(app.bbmask_confirmation.as_deref(), Some("meta-old/.* x"));
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
        assert!(a.quit_confirm);
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
        assert_eq!(app.logs.scroll_offset, 2);
        let _ = update(&mut app, Action::ScrollLogs { delta: -9 });
        assert_eq!(app.logs.scroll_offset, 0);
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
}
