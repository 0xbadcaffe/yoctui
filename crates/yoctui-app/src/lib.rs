//! Application-owned input mapping, keeping terminal concerns outside the reducer.
use std::time::SystemTime;
use yoctui_bitbake::BackendEvent;
use yoctui_model::{
    Action, AppError, BackgroundJobContext, BackgroundJobError, BackgroundJobId, BackgroundJobKind,
    BackgroundJobOutputEntry, BackgroundJobProgress, BackgroundJobResult, BackgroundJobSpec,
    BuildRequest, FocusTarget, LayerInspectorMode, LayerRelationship, LayerRelationships,
    RecipeDependencies, Screen, Severity, TaskId, TaskInfo, VariableDetail, VariableIdentity,
};

#[derive(Debug)]
pub struct BuildJobCoordinator {
    next_job_id: u64,
    active_job: Option<BackgroundJobId>,
    active_kind: Option<BackgroundJobKind>,
    cancellation_requested: bool,
}
impl Default for BuildJobCoordinator {
    fn default() -> Self {
        Self {
            next_job_id: 1,
            active_job: None,
            active_kind: None,
            cancellation_requested: false,
        }
    }
}
impl BuildJobCoordinator {
    pub fn active_job_id(&self) -> Option<BackgroundJobId> {
        self.active_job
    }

    pub fn queue_build(
        &mut self,
        request: &BuildRequest,
        queued_at: SystemTime,
    ) -> Option<Vec<Action>> {
        if self.active_job.is_some() || request.validate().is_err() {
            return None;
        }
        let id = BackgroundJobId(self.next_job_id);
        self.next_job_id = self.next_job_id.checked_add(1).unwrap_or(1);
        self.active_job = Some(id);
        self.cancellation_requested = false;
        let target = request.targets.first().cloned();
        let (kind, title, workspace, recipe) = match request.task.as_deref() {
            Some("cve_check") => (
                BackgroundJobKind::CveCheck,
                format!("CVE check {}", request.targets.join(" ")),
                Screen::Recipes,
                target.clone(),
            ),
            Some("create_spdx") => (
                BackgroundJobKind::Spdx,
                format!("SPDX generation {}", request.targets.join(" ")),
                Screen::Recipes,
                target.clone(),
            ),
            Some(task) => (
                BackgroundJobKind::Build,
                format!("Build {}:{task}", request.targets.join(" ")),
                Screen::Tasks,
                None,
            ),
            None => (
                BackgroundJobKind::Build,
                format!("Build {}", request.targets.join(" ")),
                Screen::Tasks,
                None,
            ),
        };
        self.active_kind = Some(kind);
        Some(vec![
            Action::QueueBackgroundJob(BackgroundJobSpec {
                id,
                kind,
                title,
                context: BackgroundJobContext {
                    workspace: Some(workspace),
                    target,
                    recipe,
                    task: request.task.clone(),
                    ..BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at,
            }),
            Action::StartBackgroundJob {
                id,
                started_at: queued_at,
            },
        ])
    }

    pub fn start_failed(&mut self, message: String, finished_at: SystemTime) -> Vec<Action> {
        self.active_job.take().map_or_else(Vec::new, |id| {
            self.active_kind = None;
            self.cancellation_requested = false;
            vec![Action::FailBackgroundJob {
                id,
                error: BackgroundJobError {
                    summary: "could not start BitBake".into(),
                    detail: Some(message),
                },
                finished_at,
            }]
        })
    }

    pub fn request_cancellation(&mut self) -> Option<Action> {
        let id = self.active_job?;
        if self.cancellation_requested {
            return None;
        }
        self.cancellation_requested = true;
        Some(Action::RequestBackgroundJobCancellation { id })
    }

    pub fn cancellation_failed(&mut self, message: String, timestamp: SystemTime) -> Vec<Action> {
        let Some(id) = self.active_job else {
            return Vec::new();
        };
        self.cancellation_requested = false;
        vec![
            Action::AppendBackgroundJobOutput {
                id,
                entry: BackgroundJobOutputEntry {
                    severity: Severity::Error,
                    message: format!("Cancellation request failed: {message}"),
                    timestamp,
                },
            },
            Action::RejectBackgroundJobCancellation { id },
            Action::BuildCancellationRejected(message),
        ]
    }

    pub fn backend_lost(&mut self, message: String, timestamp: SystemTime) -> Vec<Action> {
        let Some(id) = self.active_job.take() else {
            return Vec::new();
        };
        self.active_kind = None;
        self.cancellation_requested = false;
        vec![
            Action::Failure(AppError::new(
                "Backend",
                message.clone(),
                "inspect backend diagnostics and restart the build",
            )),
            Action::LoseBackgroundJob {
                id,
                error: BackgroundJobError {
                    summary: "BitBake backend lost".into(),
                    detail: Some(message),
                },
                finished_at: timestamp,
            },
        ]
    }

    pub fn job_actions_for_event(
        &mut self,
        event: &BackendEvent,
        timestamp: SystemTime,
    ) -> Vec<Action> {
        let Some(id) = self.active_job else {
            return Vec::new();
        };
        match event {
            BackendEvent::BuildStarted => vec![Action::RunBackgroundJob { id }],
            BackendEvent::ParseProgress {
                current: Some(completed),
                total: Some(total),
            } if *total > 0 && completed <= total => {
                vec![Action::UpdateBackgroundJobProgress {
                    id,
                    progress: BackgroundJobProgress::Units {
                        completed: *completed,
                        total: *total,
                    },
                }]
            }
            BackendEvent::Log(entry) => vec![Action::AppendBackgroundJobOutput {
                id,
                entry: BackgroundJobOutputEntry {
                    severity: entry.severity,
                    message: entry.message.clone(),
                    timestamp: entry.timestamp,
                },
            }],
            BackendEvent::BuildCompleted { success, exit_code } => {
                self.active_job = None;
                let kind = self.active_kind.take().unwrap_or(BackgroundJobKind::Build);
                let cancellation_requested = self.cancellation_requested;
                self.cancellation_requested = false;
                if cancellation_requested && !success {
                    vec![Action::CancelBackgroundJob {
                        id,
                        finished_at: timestamp,
                    }]
                } else if *success {
                    vec![Action::SucceedBackgroundJob {
                        id,
                        result: BackgroundJobResult {
                            summary: match kind {
                                BackgroundJobKind::CveCheck => {
                                    "CVE check completed; BitBake reported no result path".into()
                                }
                                BackgroundJobKind::Spdx => {
                                    "SPDX generation completed; BitBake reported no result path"
                                        .into()
                                }
                                _ => "BitBake build completed successfully".into(),
                            },
                            artifacts: Vec::new(),
                        },
                        finished_at: timestamp,
                    }]
                } else {
                    vec![Action::FailBackgroundJob {
                        id,
                        error: BackgroundJobError {
                            summary: "BitBake build failed".into(),
                            detail: exit_code.map(|code| format!("exit code {code}")),
                        },
                        finished_at: timestamp,
                    }]
                }
            }
            BackendEvent::CommandFailed { code, message } => {
                self.active_job = None;
                self.active_kind = None;
                self.cancellation_requested = false;
                vec![Action::FailBackgroundJob {
                    id,
                    error: BackgroundJobError {
                        summary: format!("BitBake command failed: {code}"),
                        detail: Some(message.clone()),
                    },
                    finished_at: timestamp,
                }]
            }
            BackendEvent::Disconnected => {
                self.active_job = None;
                self.active_kind = None;
                self.cancellation_requested = false;
                vec![Action::LoseBackgroundJob {
                    id,
                    error: BackgroundJobError {
                        summary: "BitBake backend disconnected".into(),
                        detail: None,
                    },
                    finished_at: timestamp,
                }]
            }
            BackendEvent::Workspace(_)
            | BackendEvent::Recipes(_)
            | BackendEvent::Layers(_)
            | BackendEvent::Variable { .. }
            | BackendEvent::Dependencies { .. }
            | BackendEvent::RecipeSources { .. }
            | BackendEvent::RecipeMetadata(_)
            | BackendEvent::LayerRelationships(_)
            | BackendEvent::ParseProgress { .. }
            | BackendEvent::TaskQueued { .. }
            | BackendEvent::TaskStarted { .. }
            | BackendEvent::TaskProgress { .. }
            | BackendEvent::TaskCompleted { .. }
            | BackendEvent::Ignored => Vec::new(),
        }
    }

    pub fn actions_for_backend_event(
        &mut self,
        event: BackendEvent,
        timestamp: SystemTime,
    ) -> Vec<Action> {
        let cancellation_acknowledged = self.cancellation_requested
            && matches!(&event, BackendEvent::BuildCompleted { success: false, .. });
        let mut actions = if cancellation_acknowledged {
            let exit_code = match &event {
                BackendEvent::BuildCompleted { exit_code, .. } => *exit_code,
                _ => None,
            };
            vec![Action::BuildCancelled { exit_code }]
        } else {
            model_action_from_backend_event(event.clone())
                .into_iter()
                .collect()
        };
        actions.extend(self.job_actions_for_event(&event, timestamp));
        actions
    }
}

pub fn model_action_from_backend_event(event: BackendEvent) -> Option<Action> {
    match event {
        BackendEvent::Workspace(workspace) => Some(Action::WorkspaceLoaded(workspace)),
        BackendEvent::BuildStarted => Some(Action::BuildStarted),
        BackendEvent::ParseProgress { current, total } => {
            Some(Action::ParseProgress { current, total })
        }
        BackendEvent::Log(entry) => Some(Action::Log(entry)),
        BackendEvent::TaskQueued {
            recipe,
            task,
            worker,
            stats,
        } => {
            let id = TaskId(format!("{recipe}:{task}"));
            let mut info = TaskInfo::active(id, recipe, task);
            info.worker = worker;
            info.stats = stats;
            Some(Action::TaskQueued(info))
        }
        BackendEvent::TaskStarted {
            recipe,
            task,
            pid,
            worker,
            log_path,
            stats,
        } => {
            let id = TaskId(format!("{recipe}:{task}"));
            let mut info = TaskInfo::active(id, recipe, task);
            info.pid = pid;
            info.worker = worker;
            info.log_path = log_path;
            info.stats = stats;
            Some(Action::TaskStarted(info))
        }
        BackendEvent::TaskProgress {
            recipe,
            task,
            progress,
        } => Some(Action::TaskProgress {
            id: TaskId(format!("{recipe}:{task}")),
            progress,
        }),
        BackendEvent::TaskCompleted {
            recipe,
            task,
            success,
        } => Some(Action::TaskCompleted {
            id: TaskId(format!("{recipe}:{task}")),
            success,
        }),
        BackendEvent::BuildCompleted { success, exit_code } => {
            Some(Action::BuildCompleted { success, exit_code })
        }
        BackendEvent::CommandFailed { code, message } => Some(Action::Failure(AppError::new(
            "BitBake",
            format!("{code}: {message}"),
            "inspect the bridge or BitBake diagnostics",
        ))),
        BackendEvent::Disconnected => Some(Action::Failure(AppError::new(
            "Bridge",
            "backend disconnected",
            "restart Yoctui and inspect the backend diagnostics",
        ))),
        BackendEvent::Recipes(recipes) => Some(Action::RecipesLoaded(recipes)),
        BackendEvent::Layers(layers) => Some(Action::LayersLoaded(layers)),
        BackendEvent::Variable {
            name,
            recipe,
            value,
            provenance,
            unexpanded_value,
            operations,
            active_overrides,
        } => Some(Action::VariableLoaded(VariableDetail {
            identity: VariableIdentity { name, recipe },
            effective_value: value,
            unexpanded_value,
            provenance,
            operations,
            active_overrides,
        })),
        BackendEvent::Dependencies {
            recipe,
            build,
            runtime,
        } => Some(Action::DependenciesLoaded(RecipeDependencies {
            recipe,
            build,
            runtime,
        })),
        BackendEvent::RecipeSources { recipe, paths } => {
            Some(Action::RecipeSourcesLoaded { recipe, paths })
        }
        BackendEvent::RecipeMetadata(metadata) => Some(Action::RecipeMetadataLoaded(metadata)),
        BackendEvent::LayerRelationships(layers) => {
            Some(Action::LayerRelationshipsLoaded(LayerRelationships {
                layers: layers
                    .into_iter()
                    .map(|layer| LayerRelationship {
                        name: layer.name,
                        priority: layer.priority,
                        compatible: layer.compatible,
                        depends: layer.depends,
                        overlays: layer.overlays,
                        appends: layer.appends,
                    })
                    .collect(),
            }))
        }
        BackendEvent::Ignored => None,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Char(char),
    Esc,
    Enter,
    CtrlC,
    CtrlB,
    CtrlP,
    F5,
    Tab,
    BackTab,
    CtrlS,
    Up,
    Down,
    Backspace,
    Left,
    Right,
}
pub fn key_action(key: Input) -> Option<Action> {
    match key {
        Input::Char('b') => None,
        Input::Char('c') => Some(Action::Cancel),
        Input::Char('f') => Some(Action::ToggleLogFollow),
        Input::Char('w') => Some(Action::ToggleLogWrap),
        Input::Char('s') => Some(Action::CycleLogSeverity),
        Input::Char('/') => Some(Action::BeginLogSearch),
        Input::Char('n') => Some(Action::NextLogMatch),
        Input::Char('N') => Some(Action::PreviousLogMatch),
        Input::Char('R') => Some(Action::CycleLogRecipeFilter),
        Input::Char('T') => Some(Action::CycleLogTaskFilter),
        Input::Backspace => Some(Action::BackspaceLogQuery),
        Input::Up => Some(Action::ScrollLogs { delta: 1 }),
        Input::Down => Some(Action::ScrollLogs { delta: -1 }),
        Input::Left => Some(Action::ScrollLogsHorizontally { delta: -8 }),
        Input::Right => Some(Action::ScrollLogsHorizontally { delta: 8 }),
        Input::Char('l') => Some(Action::Open(Screen::Logs)),
        Input::Char('h') => Some(Action::Open(Screen::BuildHistory)),
        Input::Char('e') => Some(Action::Open(Screen::Errors)),
        Input::Char('r') => Some(Action::Open(Screen::Recipes)),
        Input::Char('y') => Some(Action::Open(Screen::Layers)),
        Input::Char('v') => Some(Action::Open(Screen::Configuration)),
        Input::Char('x') => Some(Action::Open(Screen::Bbmask)),
        Input::Char('?') => Some(Action::Open(Screen::Help)),
        Input::Char('q') | Input::CtrlC => Some(Action::Quit),
        Input::CtrlP => Some(Action::OpenCommandPalette),
        Input::F5 => Some(Action::OpenBuildOptions),
        Input::Tab => Some(Action::CycleFocus { backwards: false }),
        Input::BackTab => Some(Action::CycleFocus { backwards: true }),
        Input::Char('Y') => Some(Action::ConfirmQuit),
        Input::Enter => Some(Action::ActivateNotification),
        Input::Esc => Some(Action::Open(Screen::Dashboard)),
        _ => None,
    }
}

pub fn focus_action(focus: FocusTarget, key: Input) -> Option<Action> {
    match (focus, key) {
        (FocusTarget::Navigator, Input::Up | Input::Char('k')) => {
            Some(Action::SelectNavigator { delta: -1 })
        }
        (FocusTarget::Navigator, Input::Down | Input::Char('j')) => {
            Some(Action::SelectNavigator { delta: 1 })
        }
        (FocusTarget::Navigator, Input::Enter) => Some(Action::ActivateNavigator),
        (FocusTarget::Navigator | FocusTarget::Workspace | FocusTarget::Inspector, Input::Tab) => {
            Some(Action::CycleFocus { backwards: false })
        }
        (
            FocusTarget::Navigator | FocusTarget::Workspace | FocusTarget::Inspector,
            Input::BackTab,
        ) => Some(Action::CycleFocus { backwards: true }),
        (FocusTarget::Navigator | FocusTarget::Inspector, Input::Esc) => {
            Some(Action::Focus(FocusTarget::Workspace))
        }
        _ => None,
    }
}

pub fn settings_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSetting { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSetting { delta: 1 }),
        Input::Left => Some(Action::ChangeSelectedSetting { backwards: true }),
        Input::Right | Input::Enter => Some(Action::ChangeSelectedSetting { backwards: false }),
        Input::Char('r') => Some(Action::RetrySettingsPersistence),
        _ => None,
    }
}
pub fn tasks_action(editing: bool, key: Input) -> Option<Action> {
    if editing {
        return match key {
            Input::Char(character) => Some(Action::AppendTaskFilter(character)),
            Input::Backspace => Some(Action::BackspaceTaskFilter),
            Input::Enter | Input::Esc => Some(Action::FinishTaskFilterEdit),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::ScrollBuildTasks { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::ScrollBuildTasks { delta: 1 }),
        Input::Char('f') => Some(Action::CycleTaskStateFilter),
        Input::Char('F') => Some(Action::CycleTaskFilterField),
        Input::Char('/') => Some(Action::BeginTaskFilterEdit),
        Input::Char('d') => Some(Action::CycleTaskDurationFilter),
        _ => None,
    }
}
pub fn logs_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendLogQuery(character)),
            Input::Backspace => Some(Action::BackspaceLogQuery),
            Input::Enter | Input::Esc => Some(Action::FinishLogSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::ScrollLogs { delta: 1 }),
        Input::Down | Input::Char('j') => Some(Action::ScrollLogs { delta: -1 }),
        Input::Left => Some(Action::ScrollLogsHorizontally { delta: -8 }),
        Input::Right => Some(Action::ScrollLogsHorizontally { delta: 8 }),
        Input::Char('f') => Some(Action::ToggleLogFollow),
        Input::Char('w') => Some(Action::ToggleLogWrap),
        Input::Char('s') => Some(Action::CycleLogSeverity),
        Input::Char('/') => Some(Action::BeginLogSearch),
        Input::Char('n') => Some(Action::NextLogMatch),
        Input::Char('N') => Some(Action::PreviousLogMatch),
        Input::Char('R') => Some(Action::CycleLogRecipeFilter),
        Input::Char('T') => Some(Action::CycleLogTaskFilter),
        Input::Char('B') => Some(Action::CycleLogBuildFilter),
        Input::Char('o') => Some(Action::OpenSelectedLogSource),
        Input::Char('C') => Some(Action::CopySelectedLog),
        _ => None,
    }
}
pub fn errors_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectError { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectError { delta: 1 }),
        Input::Enter => Some(Action::JumpToSelectedError),
        Input::Char('o') => Some(Action::OpenSelectedErrorSource),
        _ => None,
    }
}
pub fn layer_tree_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectLayerBrowserEntry { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectLayerBrowserEntry { delta: 1 }),
        Input::Enter => Some(Action::LayerBrowserEnter),
        Input::Right | Input::Char('l') => Some(Action::LayerBrowserExpand),
        Input::Left | Input::Char('h') => Some(Action::LayerBrowserUp),
        Input::Esc => Some(Action::CloseLayerBrowser),
        Input::Char('r') => Some(Action::RefreshLayerBrowser),
        Input::Char('e') => Some(Action::EditSelectedLayerBrowserFile),
        Input::Char('.') => Some(Action::ToggleLayerBrowserHidden),
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::Char('g') => Some(Action::SetLayerInspectorMode(LayerInspectorMode::Git)),
        Input::Char('m') => Some(Action::SetLayerInspectorMode(LayerInspectorMode::Metadata)),
        Input::Char('d') => Some(Action::SetLayerInspectorMode(
            LayerInspectorMode::Dependencies,
        )),
        _ => None,
    }
}
pub fn recipes_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectRecipe { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectRecipe { delta: 1 }),
        Input::Enter => Some(Action::BeginSelectedRecipeMetadata),
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::Char('e') => Some(Action::OpenSelectedRecipeProvider),
        Input::Char('o') => Some(Action::BeginSelectedRecipeTaskLog),
        Input::Char('p') => Some(Action::BeginSelectedRecipePatchReview),
        Input::Char('g') => Some(Action::BeginSelectedRecipeDependencies),
        Input::Char('f') => Some(Action::BeginSelectedRecipeForceTask),
        Input::Char('v') => Some(Action::BeginSelectedRecipeDevshell),
        Input::Char('K') => Some(Action::BeginSelectedRecipeDiffconfig),
        Input::Char('z') => Some(Action::BeginSelectedRecipeDiffsigs),
        Input::Char('V') => Some(Action::BeginSelectedRecipeCveCheck),
        Input::Char('X') => Some(Action::BeginSelectedRecipeSpdx),
        Input::Char('d') => Some(Action::BeginSelectedRecipeDevtoolModify),
        Input::Char('t') => Some(Action::BeginSelectedRecipeDevtoolStatus),
        Input::Char('u') => Some(Action::BeginSelectedRecipeDevtoolUpdateRecipe),
        Input::Char('F') => Some(Action::BeginSelectedRecipeDevtoolFinish),
        Input::Char('P') => Some(Action::BeginSelectedRecipeDevtoolDeploy),
        Input::Char('D') => Some(Action::BeginSelectedRecipeDevtoolReset),
        _ => None,
    }
}

pub fn config_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectConfigVariable { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectConfigVariable { delta: 1 }),
        Input::Enter => Some(Action::BeginSelectedConfigDetail),
        Input::Char('C') => Some(Action::CopySelectedConfigEffective),
        Input::Char('U') => Some(Action::CopySelectedConfigUnexpanded),
        Input::Char('s') => Some(Action::OpenConfigScopePicker),
        Input::Char('c') => Some(Action::OpenConfigComparison),
        Input::Char('E') => Some(Action::BeginConfigEdit),
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::Char('o') => Some(Action::OpenSelectedConfigSource),
        _ => None,
    }
}

pub fn config_source_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectConfigSource { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectConfigSource { delta: 1 }),
        Input::Enter => Some(Action::OpenSelectedConfigSourceChoice),
        Input::Esc => Some(Action::CancelConfigSourcePicker),
        _ => None,
    }
}

pub fn config_scope_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectConfigScope { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectConfigScope { delta: 1 }),
        Input::Enter => Some(Action::ConfirmConfigScope),
        Input::Esc => Some(Action::CancelConfigScopePicker),
        _ => None,
    }
}

pub fn config_compare_dialog_action(key: Input) -> Option<Action> {
    matches!(key, Input::Enter | Input::Esc).then_some(Action::CloseConfigComparison)
}

pub fn config_edit_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendConfigEdit(character)),
        Input::Backspace => Some(Action::BackspaceConfigEdit),
        Input::Enter => Some(Action::PreviewConfigEdit),
        Input::Esc => Some(Action::CancelConfigEdit),
        _ => None,
    }
}

pub fn config_edit_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmConfigEdit),
        Input::Esc => Some(Action::CancelConfigEditConfirmation),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use yoctui_model::{App, BackgroundJobStatus, BuildStatus, update};

    fn apply_actions(app: &mut App, actions: Vec<Action>) {
        for action in actions {
            let _ = update(app, action);
        }
    }

    fn request() -> BuildRequest {
        BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        }
    }

    #[test]
    fn background_job_build_events_survive_navigation_and_complete() {
        let mut coordinator = BuildJobCoordinator::default();
        let mut app = App::new(10, 1_000);
        apply_actions(
            &mut app,
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        let id = coordinator.active_job_id().unwrap();
        assert_eq!(
            app.background_jobs.get(id).unwrap().status,
            BackgroundJobStatus::Starting
        );
        apply_actions(
            &mut app,
            coordinator.actions_for_backend_event(
                BackendEvent::BuildStarted,
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            ),
        );
        let _ = update(&mut app, Action::Open(Screen::Layers));
        let log = yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Warning,
            message: "cache miss".into(),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: None,
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            build: None,
            protected: false,
            diagnostic: None,
        };
        apply_actions(
            &mut app,
            coordinator.actions_for_backend_event(
                BackendEvent::Log(log),
                SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            ),
        );
        apply_actions(
            &mut app,
            coordinator.actions_for_backend_event(
                BackendEvent::BuildCompleted {
                    success: true,
                    exit_code: Some(0),
                },
                SystemTime::UNIX_EPOCH + Duration::from_secs(3),
            ),
        );

        let job = app.background_jobs.get(id).unwrap();
        assert_eq!(app.screen, Screen::Layers);
        assert_eq!(app.build.status, BuildStatus::Completed);
        assert_eq!(job.status, BackgroundJobStatus::Succeeded);
        assert_eq!(job.output.len(), 1);
        assert_eq!(job.warnings, 1);
        assert_eq!(coordinator.active_job_id(), None);
    }

    #[test]
    fn typed_event_maps_every_metadata_family_and_ignores_future_events() {
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::Recipes(vec![])),
            Some(Action::RecipesLoaded(_))
        ));
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::Layers(vec![])),
            Some(Action::LayersLoaded(_))
        ));
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::Variable {
                name: "MACHINE".into(),
                recipe: None,
                value: Some("qemux86-64".into()),
                provenance: Some("conf/local.conf:1".into()),
                unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
                operations: vec![],
                active_overrides: vec![],
            }),
            Some(Action::VariableLoaded(_))
        ));
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::Dependencies {
                recipe: "busybox".into(),
                build: vec![],
                runtime: vec![],
            }),
            Some(Action::DependenciesLoaded(_))
        ));
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::RecipeSources {
                recipe: "busybox".into(),
                paths: vec!["/workspace/busybox".into()],
            }),
            Some(Action::RecipeSourcesLoaded { .. })
        ));
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::LayerRelationships(vec![])),
            Some(Action::LayerRelationshipsLoaded(_))
        ));
        assert_eq!(model_action_from_backend_event(BackendEvent::Ignored), None);
    }

    #[test]
    fn typed_event_terminal_events_emit_primary_and_job_actions_once() {
        let mut coordinator = BuildJobCoordinator::default();
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap();
        let completed = coordinator.actions_for_backend_event(
            BackendEvent::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
            SystemTime::UNIX_EPOCH,
        );
        assert_eq!(
            completed
                .iter()
                .filter(|action| matches!(action, Action::BuildCompleted { .. }))
                .count(),
            1
        );
        assert_eq!(
            completed
                .iter()
                .filter(|action| matches!(action, Action::SucceedBackgroundJob { .. }))
                .count(),
            1
        );
        assert_eq!(completed.len(), 2);

        let mut coordinator = BuildJobCoordinator::default();
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap();
        let failed = coordinator.actions_for_backend_event(
            BackendEvent::CommandFailed {
                code: "parse".into(),
                message: "bad metadata".into(),
            },
            SystemTime::UNIX_EPOCH,
        );
        assert_eq!(
            failed
                .iter()
                .filter(|action| matches!(action, Action::Failure(_)))
                .count(),
            1
        );
        assert_eq!(
            failed
                .iter()
                .filter(|action| matches!(action, Action::FailBackgroundJob { .. }))
                .count(),
            1
        );
        assert_eq!(failed.len(), 2);
    }

    #[test]
    fn background_job_command_failure_and_disconnect_are_terminal() {
        let mut coordinator = BuildJobCoordinator::default();
        let mut app = App::new(10, 1_000);
        apply_actions(
            &mut app,
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        let failed_id = coordinator.active_job_id().unwrap();
        apply_actions(
            &mut app,
            coordinator.actions_for_backend_event(
                BackendEvent::CommandFailed {
                    code: "start_failed".into(),
                    message: "server rejected build".into(),
                },
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            ),
        );
        assert_eq!(
            app.background_jobs.get(failed_id).unwrap().status,
            BackgroundJobStatus::Failed
        );

        apply_actions(
            &mut app,
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        let lost_id = coordinator.active_job_id().unwrap();
        apply_actions(
            &mut app,
            coordinator.actions_for_backend_event(
                BackendEvent::Disconnected,
                SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            ),
        );
        assert_eq!(
            app.background_jobs.get(lost_id).unwrap().status,
            BackgroundJobStatus::Lost
        );
    }

    #[test]
    fn background_job_start_failure_finishes_the_queued_job() {
        let mut coordinator = BuildJobCoordinator::default();
        let mut app = App::new(10, 1_000);
        apply_actions(
            &mut app,
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        let id = coordinator.active_job_id().unwrap();
        apply_actions(
            &mut app,
            coordinator.start_failed(
                "executable not found".into(),
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            ),
        );
        let job = app.background_jobs.get(id).unwrap();
        assert_eq!(job.status, BackgroundJobStatus::Failed);
        assert_eq!(
            job.error.as_ref().and_then(|error| error.detail.as_deref()),
            Some("executable not found")
        );
        assert_eq!(coordinator.active_job_id(), None);
    }

    #[test]
    fn background_job_backend_error_marks_the_active_job_lost() {
        let mut coordinator = BuildJobCoordinator::default();
        let mut app = App::new(10, 1_000);
        apply_actions(
            &mut app,
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        let id = coordinator.active_job_id().unwrap();
        apply_actions(
            &mut app,
            coordinator.backend_lost(
                "protocol framing failed".into(),
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            ),
        );
        let job = app.background_jobs.get(id).unwrap();
        assert_eq!(job.status, BackgroundJobStatus::Lost);
        assert_eq!(
            job.error.as_ref().and_then(|error| error.detail.as_deref()),
            Some("protocol framing failed")
        );
        assert_eq!(coordinator.active_job_id(), None);
    }

    #[test]
    fn background_job_cancellation_failure_recovers_then_acknowledges() {
        let mut coordinator = BuildJobCoordinator::default();
        let mut app = App::new(10, 1_000);
        apply_actions(
            &mut app,
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        let id = coordinator.active_job_id().unwrap();
        apply_actions(
            &mut app,
            coordinator
                .actions_for_backend_event(BackendEvent::BuildStarted, SystemTime::UNIX_EPOCH),
        );
        assert!(matches!(
            update(&mut app, Action::Cancel),
            Some(yoctui_model::Effect::Cancel)
        ));
        apply_actions(&mut app, vec![coordinator.request_cancellation().unwrap()]);
        apply_actions(
            &mut app,
            coordinator.cancellation_failed(
                "backend refused".into(),
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            ),
        );
        assert_eq!(
            app.background_jobs.get(id).unwrap().status,
            BackgroundJobStatus::Running
        );
        assert_eq!(app.build.status, BuildStatus::Running);
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("may still be running")
        );

        assert!(matches!(
            update(&mut app, Action::Cancel),
            Some(yoctui_model::Effect::Cancel)
        ));
        apply_actions(&mut app, vec![coordinator.request_cancellation().unwrap()]);
        apply_actions(
            &mut app,
            coordinator.actions_for_backend_event(
                BackendEvent::BuildCompleted {
                    success: false,
                    exit_code: None,
                },
                SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            ),
        );
        assert_eq!(app.build.status, BuildStatus::Cancelled);
        assert_eq!(
            app.background_jobs.get(id).unwrap().status,
            BackgroundJobStatus::Cancelled
        );
    }

    #[test]
    fn background_job_coordinator_prevents_duplicate_active_builds() {
        let mut coordinator = BuildJobCoordinator::default();
        assert!(
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .is_some()
        );
        assert!(
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .is_none()
        );
        assert_eq!(coordinator.active_job_id(), Some(BackgroundJobId(1)));
    }

    #[test]
    fn maps_navigation() {
        assert_eq!(
            key_action(Input::Char('l')),
            Some(Action::Open(Screen::Logs))
        );
        assert_eq!(
            key_action(Input::Tab),
            Some(Action::CycleFocus { backwards: false })
        );
        assert_eq!(key_action(Input::F5), Some(Action::OpenBuildOptions));
        assert_eq!(
            key_action(Input::Char('x')),
            Some(Action::Open(Screen::Bbmask))
        );
    }
    #[test]
    fn responsive_pane_shortcuts_map_to_focus_cycle() {
        assert_eq!(
            key_action(Input::Tab),
            Some(Action::CycleFocus { backwards: false })
        );
        assert_eq!(
            key_action(Input::BackTab),
            Some(Action::CycleFocus { backwards: true })
        );
    }
    #[test]
    fn settings_input_maps_selection_and_typed_changes() {
        assert_eq!(
            settings_action(Input::Up),
            Some(Action::SelectSetting { delta: -1 })
        );
        assert_eq!(
            settings_action(Input::Down),
            Some(Action::SelectSetting { delta: 1 })
        );
        assert_eq!(
            settings_action(Input::Left),
            Some(Action::ChangeSelectedSetting { backwards: true })
        );
        assert_eq!(
            settings_action(Input::Enter),
            Some(Action::ChangeSelectedSetting { backwards: false })
        );
        assert_eq!(
            settings_action(Input::Char('r')),
            Some(Action::RetrySettingsPersistence)
        );
        assert_eq!(settings_action(Input::Esc), None);
    }
    #[test]
    fn live_tasks_input_maps_selection_and_filter_controls() {
        assert_eq!(
            tasks_action(false, Input::Down),
            Some(Action::ScrollBuildTasks { delta: 1 })
        );
        assert_eq!(
            tasks_action(false, Input::Char('f')),
            Some(Action::CycleTaskStateFilter)
        );
        assert_eq!(
            tasks_action(false, Input::Char('F')),
            Some(Action::CycleTaskFilterField)
        );
        assert_eq!(
            tasks_action(false, Input::Char('/')),
            Some(Action::BeginTaskFilterEdit)
        );
        assert_eq!(
            tasks_action(true, Input::Char('x')),
            Some(Action::AppendTaskFilter('x'))
        );
        assert_eq!(
            tasks_action(true, Input::Esc),
            Some(Action::FinishTaskFilterEdit)
        );
        let action = model_action_from_backend_event(BackendEvent::TaskQueued {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            worker: Some("worker-1".into()),
            stats: Some(yoctui_model::TaskStats {
                completed: 3,
                total: 10,
                active: 2,
                failed: 0,
            }),
        });
        assert!(matches!(
            action,
            Some(Action::TaskQueued(TaskInfo {
                worker: Some(worker),
                stats: Some(yoctui_model::TaskStats { total: 10, .. }),
                ..
            })) if worker == "worker-1"
        ));
    }
    #[test]
    fn config_metadata_normalizes_typed_scope_and_history_once() {
        let action = model_action_from_backend_event(BackendEvent::Variable {
            name: "PACKAGE_ARCH".into(),
            recipe: Some("base-files".into()),
            value: Some("qemux86_64".into()),
            provenance: Some("/layers/meta/conf/machine/qemux86-64.conf:5".into()),
            unexpanded_value: Some("${MACHINE_ARCH}".into()),
            operations: vec![yoctui_model::VariableOperation {
                operation: "set".into(),
                file: Some("/layers/meta/conf/machine/qemux86-64.conf".into()),
                line: Some(5),
                value: Some("${MACHINE_ARCH}".into()),
            }],
            active_overrides: vec!["qemux86-64".into()],
        });
        assert!(matches!(
            action,
            Some(Action::VariableLoaded(VariableDetail {
                identity: VariableIdentity {
                    name,
                    recipe: Some(recipe),
                },
                unexpanded_value: Some(unexpanded),
                operations,
                ..
            })) if name == "PACKAGE_ARCH"
                && recipe == "base-files"
                && unexpanded == "${MACHINE_ARCH}"
                && operations.len() == 1
        ));
    }
    #[test]
    fn command_palette_global_shortcut_is_typed() {
        assert_eq!(key_action(Input::CtrlP), Some(Action::OpenCommandPalette));
        assert_eq!(focus_action(FocusTarget::CommandPalette, Input::Tab), None);
    }
    #[test]
    fn dialog_focus_navigation_keys_are_typed_before_cli_routing() {
        assert_eq!(
            key_action(Input::Tab),
            Some(Action::CycleFocus { backwards: false })
        );
        assert_eq!(
            key_action(Input::BackTab),
            Some(Action::CycleFocus { backwards: true })
        );
        assert_eq!(
            key_action(Input::Esc),
            Some(Action::Open(Screen::Dashboard))
        );
        assert_eq!(
            focus_action(FocusTarget::Navigator, Input::Up),
            Some(Action::SelectNavigator { delta: -1 })
        );
        assert_eq!(
            focus_action(FocusTarget::Inspector, Input::Up),
            None,
            "inspector arrows must not leak into workspace actions"
        );
        assert_eq!(
            focus_action(FocusTarget::Dialog, Input::Tab),
            None,
            "modal input is handled only by the active dialog"
        );
    }
    #[test]
    fn maps_log_controls() {
        assert_eq!(key_action(Input::Char('f')), Some(Action::ToggleLogFollow));
        assert_eq!(key_action(Input::Char('w')), Some(Action::ToggleLogWrap));
        assert_eq!(key_action(Input::Up), Some(Action::ScrollLogs { delta: 1 }));
    }
    #[test]
    fn log_workspace_maps_selection_search_filters_and_selected_actions() {
        assert_eq!(
            logs_action(false, Input::Up),
            Some(Action::ScrollLogs { delta: 1 })
        );
        assert_eq!(
            logs_action(false, Input::Char('B')),
            Some(Action::CycleLogBuildFilter)
        );
        assert_eq!(
            logs_action(false, Input::Char('o')),
            Some(Action::OpenSelectedLogSource)
        );
        assert_eq!(
            logs_action(false, Input::Char('C')),
            Some(Action::CopySelectedLog)
        );
        assert_eq!(
            logs_action(true, Input::Char('x')),
            Some(Action::AppendLogQuery('x'))
        );
        assert_eq!(logs_action(true, Input::Esc), Some(Action::FinishLogSearch));
    }
    #[test]
    fn enter_activates_contextual_notification() {
        assert_eq!(key_action(Input::Enter), Some(Action::ActivateNotification));
    }
    #[test]
    fn maps_severity_filter_control() {
        assert_eq!(key_action(Input::Char('s')), Some(Action::CycleLogSeverity));
    }
    #[test]
    fn error_workspace_maps_selection_log_jump_and_source_open() {
        assert_eq!(
            errors_action(Input::Up),
            Some(Action::SelectError { delta: -1 })
        );
        assert_eq!(
            errors_action(Input::Enter),
            Some(Action::JumpToSelectedError)
        );
        assert_eq!(
            errors_action(Input::Char('o')),
            Some(Action::OpenSelectedErrorSource)
        );
    }
    #[test]
    fn layer_tree_maps_lazy_navigation_hidden_refresh_and_inspector_modes() {
        assert_eq!(
            layer_tree_action(false, Input::Right),
            Some(Action::LayerBrowserExpand)
        );
        assert_eq!(
            layer_tree_action(false, Input::Left),
            Some(Action::LayerBrowserUp)
        );
        assert_eq!(
            layer_tree_action(false, Input::Char('.')),
            Some(Action::ToggleLayerBrowserHidden)
        );
        assert_eq!(
            layer_tree_action(false, Input::Char('g')),
            Some(Action::SetLayerInspectorMode(LayerInspectorMode::Git))
        );
        assert_eq!(
            layer_tree_action(true, Input::Char('b')),
            Some(Action::AppendMetadataQuery('b'))
        );
    }
    #[test]
    fn recipes_workspace_maps_search_selection_detail_and_dependencies() {
        assert_eq!(
            recipes_workspace_action(false, Input::Down),
            Some(Action::SelectRecipe { delta: 1 })
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Enter),
            Some(Action::BeginSelectedRecipeMetadata)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('g')),
            Some(Action::BeginSelectedRecipeDependencies)
        );
        assert_eq!(
            recipes_workspace_action(true, Input::Char('b')),
            Some(Action::AppendMetadataQuery('b'))
        );
        assert_eq!(
            recipes_workspace_action(true, Input::Backspace),
            Some(Action::BackspaceMetadataQuery)
        );
    }
    #[test]
    fn recipe_bitbake_action_maps_standard_and_forced_task_controls() {
        assert_eq!(
            recipes_workspace_action(false, Input::Char('f')),
            Some(Action::BeginSelectedRecipeForceTask)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('v')),
            Some(Action::BeginSelectedRecipeDevshell)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('K')),
            Some(Action::BeginSelectedRecipeDiffconfig)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('z')),
            Some(Action::BeginSelectedRecipeDiffsigs)
        );
        let request = BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("compile".into()),
            force: true,
        };
        let mut coordinator = BuildJobCoordinator::default();
        let actions = coordinator
            .queue_build(&request, SystemTime::UNIX_EPOCH)
            .unwrap();
        assert!(matches!(
            &actions[0],
            Action::QueueBackgroundJob(spec)
                if spec.context.target.as_deref() == Some("busybox")
                    && spec.context.task.as_deref() == Some("compile")
        ));
        assert!(
            coordinator
                .queue_build(&request, SystemTime::UNIX_EPOCH)
                .is_none()
        );
    }
    #[test]
    fn recipe_navigation_maps_files_logs_patches_and_devtool_routes() {
        assert_eq!(
            recipes_workspace_action(false, Input::Char('e')),
            Some(Action::OpenSelectedRecipeProvider)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('o')),
            Some(Action::BeginSelectedRecipeTaskLog)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('p')),
            Some(Action::BeginSelectedRecipePatchReview)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('d')),
            Some(Action::BeginSelectedRecipeDevtoolModify)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('t')),
            Some(Action::BeginSelectedRecipeDevtoolStatus)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('u')),
            Some(Action::BeginSelectedRecipeDevtoolUpdateRecipe)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('F')),
            Some(Action::BeginSelectedRecipeDevtoolFinish)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('P')),
            Some(Action::BeginSelectedRecipeDevtoolDeploy)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('D')),
            Some(Action::BeginSelectedRecipeDevtoolReset)
        );
    }

    #[test]
    fn devtool_metadata_shortcut_requests_typed_status() {
        assert_eq!(
            recipes_workspace_action(false, Input::Char('t')),
            Some(Action::BeginSelectedRecipeDevtoolStatus)
        );
    }

    #[test]
    fn config_workspace_maps_search_selection_and_lazy_detail() {
        assert_eq!(
            config_workspace_action(false, Input::Down),
            Some(Action::SelectConfigVariable { delta: 1 })
        );
        assert_eq!(
            config_workspace_action(false, Input::Enter),
            Some(Action::BeginSelectedConfigDetail)
        );
        assert_eq!(
            config_workspace_action(false, Input::Char('/')),
            Some(Action::BeginMetadataSearch)
        );
        assert_eq!(
            config_workspace_action(true, Input::Char('M')),
            Some(Action::AppendMetadataQuery('M'))
        );
    }

    #[test]
    fn config_copy_shortcuts_are_typed() {
        assert_eq!(
            config_workspace_action(false, Input::Char('C')),
            Some(Action::CopySelectedConfigEffective)
        );
        assert_eq!(
            config_workspace_action(false, Input::Char('U')),
            Some(Action::CopySelectedConfigUnexpanded)
        );
    }

    #[test]
    fn config_source_picker_keys_are_modal_and_typed() {
        assert_eq!(
            config_source_picker_action(Input::Down),
            Some(Action::SelectConfigSource { delta: 1 })
        );
        assert_eq!(
            config_source_picker_action(Input::Enter),
            Some(Action::OpenSelectedConfigSourceChoice)
        );
        assert_eq!(
            config_source_picker_action(Input::Esc),
            Some(Action::CancelConfigSourcePicker)
        );
    }

    #[test]
    fn config_scope_shortcut_and_picker_keys_are_typed() {
        assert_eq!(
            config_workspace_action(false, Input::Char('s')),
            Some(Action::OpenConfigScopePicker)
        );
        assert_eq!(
            config_scope_picker_action(Input::Down),
            Some(Action::SelectConfigScope { delta: 1 })
        );
        assert_eq!(
            config_scope_picker_action(Input::Enter),
            Some(Action::ConfirmConfigScope)
        );
        assert_eq!(
            config_scope_picker_action(Input::Esc),
            Some(Action::CancelConfigScopePicker)
        );
    }

    #[test]
    fn config_compare_shortcut_and_close_keys_are_typed() {
        assert_eq!(
            config_workspace_action(false, Input::Char('c')),
            Some(Action::OpenConfigComparison)
        );
        assert_eq!(
            config_compare_dialog_action(Input::Enter),
            Some(Action::CloseConfigComparison)
        );
        assert_eq!(
            config_compare_dialog_action(Input::Esc),
            Some(Action::CloseConfigComparison)
        );
    }

    #[test]
    fn config_edit_preview_shortcut_and_dialog_keys_are_typed() {
        assert_eq!(
            config_workspace_action(false, Input::Char('E')),
            Some(Action::BeginConfigEdit)
        );
        assert_eq!(
            config_edit_dialog_action(Input::Char('x')),
            Some(Action::AppendConfigEdit('x'))
        );
        assert_eq!(
            config_edit_dialog_action(Input::Enter),
            Some(Action::PreviewConfigEdit)
        );
        assert_eq!(
            config_edit_confirmation_action(Input::Enter),
            Some(Action::ConfirmConfigEdit)
        );
    }

    #[test]
    fn config_edit_write_confirmation_is_modal_and_cancellable() {
        assert_eq!(
            config_edit_confirmation_action(Input::Enter),
            Some(Action::ConfirmConfigEdit)
        );
        assert_eq!(
            config_edit_confirmation_action(Input::Esc),
            Some(Action::CancelConfigEditConfirmation)
        );
        assert_eq!(config_edit_confirmation_action(Input::Char('E')), None);
    }

    #[test]
    fn recipe_qa_action_maps_capabilities_and_persists_terminal_job_outcomes() {
        assert_eq!(
            recipes_workspace_action(false, Input::Char('V')),
            Some(Action::BeginSelectedRecipeCveCheck)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('X')),
            Some(Action::BeginSelectedRecipeSpdx)
        );

        let cve = BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("cve_check".into()),
            force: false,
        };
        let mut coordinator = BuildJobCoordinator::default();
        let mut app = App::new(20, 4_000);
        let queued = coordinator
            .queue_build(&cve, SystemTime::UNIX_EPOCH)
            .unwrap();
        assert!(matches!(
            &queued[0],
            Action::QueueBackgroundJob(spec)
                if spec.kind == BackgroundJobKind::CveCheck
                    && spec.context.workspace == Some(Screen::Recipes)
                    && spec.context.recipe.as_deref() == Some("busybox")
                    && spec.context.task.as_deref() == Some("cve_check")
        ));
        for action in queued {
            let _ = yoctui_model::update(&mut app, action);
        }
        for action in coordinator
            .actions_for_backend_event(BackendEvent::BuildStarted, SystemTime::UNIX_EPOCH)
        {
            let _ = yoctui_model::update(&mut app, action);
        }
        for action in coordinator.actions_for_backend_event(
            BackendEvent::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
            SystemTime::UNIX_EPOCH,
        ) {
            let _ = yoctui_model::update(&mut app, action);
        }
        let cve_job = app.background_jobs.jobs.back().unwrap();
        assert_eq!(cve_job.status, BackgroundJobStatus::Succeeded);
        assert!(
            cve_job
                .result
                .as_ref()
                .unwrap()
                .summary
                .contains("no result path")
        );
        assert!(cve_job.result.as_ref().unwrap().artifacts.is_empty());

        let spdx = BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("create_spdx".into()),
            force: false,
        };
        for action in coordinator
            .queue_build(&spdx, SystemTime::UNIX_EPOCH)
            .unwrap()
        {
            let _ = yoctui_model::update(&mut app, action);
        }
        assert!(
            coordinator
                .queue_build(&spdx, SystemTime::UNIX_EPOCH)
                .is_none()
        );
        let cancellation = coordinator.request_cancellation().unwrap();
        let _ = yoctui_model::update(&mut app, cancellation);
        for action in coordinator.actions_for_backend_event(
            BackendEvent::BuildCompleted {
                success: false,
                exit_code: Some(130),
            },
            SystemTime::UNIX_EPOCH,
        ) {
            let _ = yoctui_model::update(&mut app, action);
        }
        assert_eq!(
            app.background_jobs.jobs.back().unwrap().status,
            BackgroundJobStatus::Cancelled
        );

        for action in coordinator
            .queue_build(&cve, SystemTime::UNIX_EPOCH)
            .unwrap()
        {
            let _ = yoctui_model::update(&mut app, action);
        }
        for action in coordinator
            .actions_for_backend_event(BackendEvent::Disconnected, SystemTime::UNIX_EPOCH)
        {
            let _ = yoctui_model::update(&mut app, action);
        }
        assert_eq!(
            app.background_jobs.jobs.back().unwrap().status,
            BackgroundJobStatus::Lost
        );
    }
    #[test]
    fn maps_recipe_and_task_filter_controls() {
        assert_eq!(
            key_action(Input::Char('R')),
            Some(Action::CycleLogRecipeFilter)
        );
        assert_eq!(
            key_action(Input::Char('T')),
            Some(Action::CycleLogTaskFilter)
        );
    }
    #[test]
    fn maps_log_match_navigation_controls() {
        assert_eq!(key_action(Input::Char('n')), Some(Action::NextLogMatch));
        assert_eq!(key_action(Input::Char('N')), Some(Action::PreviousLogMatch));
    }
}
