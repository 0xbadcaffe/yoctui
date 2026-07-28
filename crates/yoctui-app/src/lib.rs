//! Application-owned input mapping, keeping terminal concerns outside the reducer.
use std::time::SystemTime;
use yoctui_bitbake::{
    BackendEvent, DevtoolOutputStream, DevtoolRunnerEvent, QemuRunnerEvent, QemuRunnerOutputStream,
};
use yoctui_model::{
    Action, AppError, BackgroundJobContext, BackgroundJobError, BackgroundJobId, BackgroundJobKind,
    BackgroundJobOutputEntry, BackgroundJobOutputSource, BackgroundJobProgress,
    BackgroundJobResult, BackgroundJobSpec, BuildRequest, DevtoolOperation, FocusTarget,
    LayerInspectorMode, LayerRelationship, LayerRelationships, QemuOutputStream, QemuSessionId,
    RecipeDependencies, Screen, Severity, TaskId, TaskInfo, VariableDetail, VariableIdentity,
};

pub fn qemu_actions_for_runner_event(
    id: QemuSessionId,
    event: QemuRunnerEvent,
    timestamp: SystemTime,
) -> Vec<Action> {
    match event {
        QemuRunnerEvent::Starting => vec![Action::QemuSessionStarting {
            id,
            started_at: timestamp,
        }],
        QemuRunnerEvent::Started => vec![Action::QemuSessionRunning { id }],
        QemuRunnerEvent::Output {
            stream,
            line,
            truncated,
        } => vec![Action::AppendQemuSessionOutput {
            id,
            stream: match stream {
                QemuRunnerOutputStream::Stdout => QemuOutputStream::Stdout,
                QemuRunnerOutputStream::Stderr => QemuOutputStream::Stderr,
            },
            line,
            truncated,
            timestamp,
        }],
        QemuRunnerEvent::Completed { exit_code } => vec![Action::CompleteQemuSession {
            id,
            exit_code,
            finished_at: timestamp,
        }],
        QemuRunnerEvent::Failed { message, exit_code } => vec![Action::FailQemuSession {
            id,
            message,
            exit_code,
            finished_at: timestamp,
        }],
        QemuRunnerEvent::Cancelled { forced, exit_code } => {
            let mut actions = Vec::new();
            if forced {
                actions.push(Action::AppendQemuSessionOutput {
                    id,
                    stream: QemuOutputStream::Stderr,
                    line: "runqemu cancellation required forced termination".into(),
                    truncated: false,
                    timestamp,
                });
            }
            actions.push(Action::CancelQemuSession {
                id,
                exit_code,
                finished_at: timestamp,
            });
            actions
        }
        QemuRunnerEvent::CancellationRejected { message } => {
            vec![Action::RejectQemuSessionCancellation { id, message }]
        }
        QemuRunnerEvent::Lost { message } => vec![Action::LoseQemuSession {
            id,
            message,
            finished_at: timestamp,
        }],
    }
}

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
                    source: BackgroundJobOutputSource::Backend,
                    truncated: false,
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
                    source: BackgroundJobOutputSource::Backend,
                    truncated: false,
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
            | BackendEvent::DependencyGraph { .. }
            | BackendEvent::DependencyGraphFailed { .. }
            | BackendEvent::SignatureDump { .. }
            | BackendEvent::SignatureDumpFailed { .. }
            | BackendEvent::SignatureComparison { .. }
            | BackendEvent::SignatureComparisonFailed { .. }
            | BackendEvent::PackageInventory { .. }
            | BackendEvent::PackageInventoryFailed { .. }
            | BackendEvent::PackageDetail { .. }
            | BackendEvent::PackageDetailFailed { .. }
            | BackendEvent::ImageArtifacts { .. }
            | BackendEvent::ImageArtifactsFailed { .. }
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

#[derive(Debug)]
pub struct DevtoolJobCoordinator {
    next_job_id: u64,
    active_job: Option<BackgroundJobId>,
    active_operation: Option<DevtoolOperation>,
    cancellation_requested: bool,
}
impl Default for DevtoolJobCoordinator {
    fn default() -> Self {
        Self {
            next_job_id: 1_u64 << 63,
            active_job: None,
            active_operation: None,
            cancellation_requested: false,
        }
    }
}
impl DevtoolJobCoordinator {
    pub fn active_job_id(&self) -> Option<BackgroundJobId> {
        self.active_job
    }

    pub fn active_operation(&self) -> Option<&DevtoolOperation> {
        self.active_operation.as_ref()
    }

    pub fn queue(
        &mut self,
        operation: DevtoolOperation,
        queued_at: SystemTime,
    ) -> Option<Vec<Action>> {
        if self.active_job.is_some() || operation.validate().is_err() {
            return None;
        }
        let id = BackgroundJobId(self.next_job_id);
        self.next_job_id = self.next_job_id.checked_add(1).unwrap_or(1_u64 << 63);
        let recipe = operation.recipe().to_owned();
        let (label, target, path) = match &operation {
            DevtoolOperation::Modify { .. } => ("modify", None, None),
            DevtoolOperation::UpdateRecipe { .. } => ("update-recipe", None, None),
            DevtoolOperation::Finish { destination, .. } => {
                ("finish", None, Some(destination.clone()))
            }
            DevtoolOperation::DeployTarget { target, .. } => {
                ("deploy-target", Some(target.clone()), None)
            }
            DevtoolOperation::UndeployTarget { target, .. } => {
                ("undeploy-target", Some(target.clone()), None)
            }
            DevtoolOperation::Reset { .. } => ("reset", None, None),
        };
        self.active_job = Some(id);
        self.active_operation = Some(operation);
        self.cancellation_requested = false;
        Some(vec![
            Action::QueueBackgroundJob(BackgroundJobSpec {
                id,
                kind: BackgroundJobKind::Devtool,
                title: format!("Devtool {label} {recipe}"),
                context: BackgroundJobContext {
                    workspace: Some(Screen::Recipes),
                    target,
                    recipe: Some(recipe),
                    path,
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
        let Some(id) = self.active_job.take() else {
            return Vec::new();
        };
        self.active_operation = None;
        self.cancellation_requested = false;
        vec![Action::FailBackgroundJob {
            id,
            error: BackgroundJobError {
                summary: "Could not start Devtool".into(),
                detail: Some(message),
            },
            finished_at,
        }]
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
                    message: format!("Devtool cancellation failed: {message}"),
                    source: BackgroundJobOutputSource::Backend,
                    truncated: false,
                    timestamp,
                },
            },
            Action::RejectBackgroundJobCancellation { id },
        ]
    }

    pub fn actions_for_event(
        &mut self,
        event: DevtoolRunnerEvent,
        timestamp: SystemTime,
    ) -> Vec<Action> {
        let Some(id) = self.active_job else {
            return Vec::new();
        };
        match event {
            DevtoolRunnerEvent::Started => vec![Action::RunBackgroundJob { id }],
            DevtoolRunnerEvent::Output {
                stream,
                line,
                truncated,
            } => vec![Action::AppendBackgroundJobOutput {
                id,
                entry: BackgroundJobOutputEntry {
                    severity: Severity::Info,
                    message: line,
                    source: match stream {
                        DevtoolOutputStream::Stdout => BackgroundJobOutputSource::Stdout,
                        DevtoolOutputStream::Stderr => BackgroundJobOutputSource::Stderr,
                    },
                    truncated,
                    timestamp,
                },
            }],
            DevtoolRunnerEvent::Completed { exit_code } => {
                self.active_job = None;
                self.active_operation = None;
                self.cancellation_requested = false;
                vec![Action::SucceedBackgroundJob {
                    id,
                    result: BackgroundJobResult {
                        summary: exit_code.map_or_else(
                            || "Devtool completed successfully".into(),
                            |code| format!("Devtool completed successfully (exit code {code})"),
                        ),
                        artifacts: Vec::new(),
                    },
                    finished_at: timestamp,
                }]
            }
            DevtoolRunnerEvent::Failed { exit_code } => {
                self.active_job = None;
                self.active_operation = None;
                self.cancellation_requested = false;
                vec![Action::FailBackgroundJob {
                    id,
                    error: BackgroundJobError {
                        summary: "Devtool failed".into(),
                        detail: exit_code.map(|code| format!("exit code {code}")),
                    },
                    finished_at: timestamp,
                }]
            }
            DevtoolRunnerEvent::Cancelled { forced, exit_code } => {
                self.active_job = None;
                self.active_operation = None;
                self.cancellation_requested = false;
                let mut actions = Vec::new();
                if forced {
                    actions.push(Action::AppendBackgroundJobOutput {
                        id,
                        entry: BackgroundJobOutputEntry {
                            severity: Severity::Warning,
                            message: "Devtool cancellation required forced termination".into(),
                            source: BackgroundJobOutputSource::Backend,
                            truncated: false,
                            timestamp,
                        },
                    });
                }
                if let Some(code) = exit_code {
                    actions.push(Action::AppendBackgroundJobOutput {
                        id,
                        entry: BackgroundJobOutputEntry {
                            severity: Severity::Info,
                            message: format!("Devtool cancellation exit code {code}"),
                            source: BackgroundJobOutputSource::Backend,
                            truncated: false,
                            timestamp,
                        },
                    });
                }
                actions.push(Action::CancelBackgroundJob {
                    id,
                    finished_at: timestamp,
                });
                actions
            }
            DevtoolRunnerEvent::Lost { message } => {
                self.active_job = None;
                self.active_operation = None;
                self.cancellation_requested = false;
                vec![Action::LoseBackgroundJob {
                    id,
                    error: BackgroundJobError {
                        summary: "Devtool process lost".into(),
                        detail: Some(message),
                    },
                    finished_at: timestamp,
                }]
            }
        }
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
        BackendEvent::DependencyGraph { graph, limitations } => {
            if limitations.is_empty() {
                Some(Action::DependencyGraphLoaded(graph))
            } else {
                Some(Action::DependencyGraphPartial { graph, limitations })
            }
        }
        BackendEvent::DependencyGraphFailed { root, message } => {
            Some(Action::DependencyGraphFailed { root, message })
        }
        BackendEvent::SignatureDump {
            target,
            records,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::SignatureDumpLoaded { target, records })
            } else {
                Some(Action::SignatureDumpPartial {
                    target,
                    records,
                    limitations,
                })
            }
        }
        BackendEvent::SignatureDumpFailed { target, message } => {
            Some(Action::SignatureDumpFailed { target, message })
        }
        BackendEvent::SignatureComparison {
            request,
            differences,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::SignatureComparisonLoaded {
                    request,
                    differences,
                })
            } else {
                Some(Action::SignatureComparisonPartial {
                    request,
                    differences,
                    limitations,
                })
            }
        }
        BackendEvent::SignatureComparisonFailed { request, message } => {
            Some(Action::SignatureComparisonFailed { request, message })
        }
        BackendEvent::PackageInventory {
            request,
            packages,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::PackageInventoryLoaded { request, packages })
            } else {
                Some(Action::PackageInventoryPartial {
                    request,
                    packages,
                    limitations,
                })
            }
        }
        BackendEvent::PackageInventoryFailed { request, message } => {
            Some(Action::PackageInventoryFailed { request, message })
        }
        BackendEvent::PackageDetail {
            request,
            detail,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::PackageDetailLoaded { request, detail })
            } else {
                Some(Action::PackageDetailPartial {
                    request,
                    detail,
                    limitations,
                })
            }
        }
        BackendEvent::PackageDetailFailed { request, message } => {
            Some(Action::PackageDetailFailed { request, message })
        }
        BackendEvent::ImageArtifacts {
            request,
            inventory,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::ImageArtifactInventoryLoaded { request, inventory })
            } else {
                Some(Action::ImageArtifactInventoryPartial {
                    request,
                    inventory,
                    limitations,
                })
            }
        }
        BackendEvent::ImageArtifactsFailed { request, message } => {
            Some(Action::ImageArtifactInventoryFailed { request, message })
        }
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
pub fn dependency_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectDependencyGraphNode { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectDependencyGraphNode { delta: 1 }),
        Input::Enter => Some(Action::OpenSelectedDependencyRecipe),
        Input::Char('o') => Some(Action::OpenSelectedDependencyProvider),
        Input::Char('L') => Some(Action::OpenSelectedDependencyTaskLog),
        Input::Char('r') => Some(Action::RefreshDependencyGraph),
        _ => None,
    }
}
pub fn signature_task_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSignatureTask { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSignatureTask { delta: 1 }),
        Input::Enter => Some(Action::ConfirmSignatureTask),
        Input::Esc => Some(Action::CancelSignatureTaskPicker),
        _ => None,
    }
}
pub fn signature_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSignatureRecord { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSignatureRecord { delta: 1 }),
        Input::Char('1') => Some(Action::SetSelectedSignatureComparisonSide(
            yoctui_model::SignatureComparisonSide::Left,
        )),
        Input::Char('2') => Some(Action::SetSelectedSignatureComparisonSide(
            yoctui_model::SignatureComparisonSide::Right,
        )),
        Input::Char('c') => Some(Action::BeginSignatureComparison),
        Input::Char('r') => Some(Action::RefreshSignatureDump),
        Input::Char('e') => Some(Action::OpenSignatureProvider),
        Input::Esc => Some(Action::LeaveSignatureWorkspace),
        _ => None,
    }
}
pub fn package_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendPackageQuery(character)),
            Input::Backspace => Some(Action::BackspacePackageQuery),
            Input::Enter | Input::Esc => Some(Action::FinishPackageSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectPackage { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectPackage { delta: 1 }),
        Input::Enter => Some(Action::BeginSelectedPackageDetail),
        Input::Char('/') => Some(Action::BeginPackageSearch),
        Input::Char('R') => Some(Action::RefreshPackageInventory),
        Input::Char('c') => Some(Action::CancelPackageOperation),
        Input::Char('D') => Some(Action::TogglePackageDependencyKind),
        Input::Char('[') => Some(Action::SelectPackageDependency { delta: -1 }),
        Input::Char(']') => Some(Action::SelectPackageDependency { delta: 1 }),
        Input::Char('d') => Some(Action::OpenSelectedPackageDependency),
        Input::Char('u') => Some(Action::BackPackageNavigation),
        Input::Char('o') => Some(Action::OpenSelectedPackageRecipe),
        Input::Char('e') => Some(Action::OpenSelectedPackageProvider),
        _ => None,
    }
}

pub fn images_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendImageArtifactQuery(character)),
            Input::Backspace => Some(Action::BackspaceImageArtifactQuery),
            Input::Enter | Input::Esc => Some(Action::FinishImageArtifactSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectImageArtifact { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectImageArtifact { delta: 1 }),
        Input::Char('/') => Some(Action::BeginImageArtifactSearch),
        Input::Char('R') => Some(Action::RefreshImageArtifactInventory),
        Input::Char('c') => Some(Action::CancelImageArtifactOperation),
        Input::Char('b') => Some(Action::BeginSelectedImageArtifactBuild),
        Input::Char('o') => Some(Action::OpenSelectedImageArtifact),
        Input::Char('m') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Manifest,
        )),
        Input::Char('l') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::License,
        )),
        Input::Char('s') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Spdx,
        )),
        Input::Char('w') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Wic,
        )),
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
        Input::Char('Z') => Some(Action::BeginSelectedRecipeSignatures),
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

pub fn devtool_modify_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolModify),
        Input::Esc => Some(Action::CancelDevtoolModify),
        _ => None,
    }
}

pub fn devtool_update_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolUpdateRecipe),
        Input::Esc => Some(Action::CancelDevtoolUpdateRecipe),
        _ => None,
    }
}

pub fn devtool_finish_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectDevtoolFinishLayer { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectDevtoolFinishLayer { delta: 1 }),
        Input::Enter => Some(Action::PreviewDevtoolFinish),
        Input::Esc => Some(Action::CancelDevtoolFinish),
        _ => None,
    }
}

pub fn devtool_finish_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolFinish),
        Input::Esc => Some(Action::CancelDevtoolFinishConfirmation),
        _ => None,
    }
}

pub fn devtool_deploy_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendDevtoolDeployTarget(character)),
        Input::Backspace => Some(Action::BackspaceDevtoolDeployTarget),
        Input::Enter => Some(Action::PreviewDevtoolDeploy),
        Input::Esc => Some(Action::CancelDevtoolDeploy),
        _ => None,
    }
}

pub fn devtool_deploy_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolDeploy),
        Input::Esc => Some(Action::CancelDevtoolDeployConfirmation),
        _ => None,
    }
}

pub fn devtool_reset_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolReset),
        Input::Esc => Some(Action::CancelDevtoolReset),
        _ => None,
    }
}

pub fn recipe_editor_action(editing: bool, key: Input) -> Option<Action> {
    match key {
        Input::Esc => Some(Action::CloseRecipeEditor),
        Input::Up => Some(Action::SelectRecipeEditorFile { delta: -1 }),
        Input::Down => Some(Action::SelectRecipeEditorFile { delta: 1 }),
        Input::Enter if editing => Some(Action::AppendRecipeEditor('\n')),
        Input::Enter | Input::Char('e') if !editing => Some(Action::ToggleRecipeEditorEditing),
        Input::CtrlS => Some(Action::SaveRecipeEditor),
        Input::CtrlB => Some(Action::BeginRecipeEditorBuild),
        Input::Backspace => Some(Action::BackspaceRecipeEditor),
        Input::Char(character) => Some(Action::AppendRecipeEditor(character)),
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
    use yoctui_model::{
        App, BackgroundJobStatus, BuildStatus, DependencyEdge, DependencyEdgeKind, DependencyGraph,
        DependencyGraphState, DependencyNodeId, ImageArtifact, ImageArtifactField,
        ImageArtifactIdentity, ImageArtifactInventory, ImageArtifactKind, ImageArtifactRequest,
        PackageDetail, PackageDetailRequest, PackageField, PackageIdentity,
        PackageInventoryRequest, PackageSummary, SignatureComparisonRequest, SignatureDifference,
        SignatureDifferenceCategory, SignatureIdentity, SignatureRecord, SignatureTarget, update,
    };

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
    fn dependency_graph_typed_events_map_success_partial_and_failure() {
        let root = DependencyNodeId::recipe("core-image-minimal");
        let (graph, _) = DependencyGraph::normalize(
            root.clone(),
            Vec::new(),
            vec![DependencyEdge {
                from: root.clone(),
                to: DependencyNodeId::recipe("busybox"),
                kind: DependencyEdgeKind::Runtime,
            }],
            10,
            10,
        );
        assert_eq!(
            model_action_from_backend_event(BackendEvent::DependencyGraph {
                graph: graph.clone(),
                limitations: Vec::new(),
            }),
            Some(Action::DependencyGraphLoaded(graph.clone()))
        );
        let mut compatibility = App::new(10, 1_000);
        let action = model_action_from_backend_event(BackendEvent::DependencyGraph {
            graph: graph.clone(),
            limitations: Vec::new(),
        })
        .unwrap();
        let _ = update(&mut compatibility, action);
        assert_eq!(
            compatibility.dependencies.as_ref().unwrap().runtime,
            ["busybox"]
        );
        assert_eq!(
            model_action_from_backend_event(BackendEvent::DependencyGraph {
                graph: graph.clone(),
                limitations: vec!["task graph unavailable".into()],
            }),
            Some(Action::DependencyGraphPartial {
                graph,
                limitations: vec!["task graph unavailable".into()],
            })
        );
        assert_eq!(
            model_action_from_backend_event(BackendEvent::DependencyGraphFailed {
                root: root.clone(),
                message: "query failed".into(),
            }),
            Some(Action::DependencyGraphFailed {
                root,
                message: "query failed".into(),
            })
        );

        let mut app = App::new(10, 1_000);
        let action = model_action_from_backend_event(BackendEvent::DependencyGraphFailed {
            root: DependencyNodeId::recipe("image"),
            message: "offline".into(),
        })
        .unwrap();
        let _ = update(&mut app, action);
        assert!(matches!(
            app.dependency_graph,
            DependencyGraphState::Failed { .. }
        ));
    }
    #[test]
    fn signature_model_typed_events_map_dump_comparison_partial_and_failure() {
        let target = SignatureTarget {
            recipe: "busybox".into(),
            task: "do_compile".into(),
        };
        let identity = SignatureIdentity {
            target: target.clone(),
            hash: Some("abc".into()),
            path: Some("/tmp/busybox.sigdata".into()),
        };
        let record = SignatureRecord {
            identity: identity.clone(),
            base_hash: Some("base".into()),
            task_hash: Some("task".into()),
            variables: Vec::new(),
            dependencies: Vec::new(),
        };
        assert_eq!(
            model_action_from_backend_event(BackendEvent::SignatureDump {
                target: target.clone(),
                records: vec![record.clone()],
                limitations: Vec::new(),
            }),
            Some(Action::SignatureDumpLoaded {
                target: target.clone(),
                records: vec![record.clone()],
            })
        );
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::SignatureDump {
                target: target.clone(),
                records: vec![record],
                limitations: vec!["partial".into()],
            }),
            Some(Action::SignatureDumpPartial { .. })
        ));
        let request = SignatureComparisonRequest {
            left: identity.clone(),
            right: SignatureIdentity {
                hash: Some("def".into()),
                path: Some("/tmp/busybox-old.sigdata".into()),
                ..identity
            },
        };
        let difference = SignatureDifference {
            category: SignatureDifferenceCategory::ChangedValue,
            key: "CC".into(),
            left: Some("gcc".into()),
            right: Some("clang".into()),
        };
        assert_eq!(
            model_action_from_backend_event(BackendEvent::SignatureComparison {
                request: request.clone(),
                differences: vec![difference.clone()],
                limitations: Vec::new(),
            }),
            Some(Action::SignatureComparisonLoaded {
                request: request.clone(),
                differences: vec![difference],
            })
        );
        assert_eq!(
            model_action_from_backend_event(BackendEvent::SignatureComparisonFailed {
                request: request.clone(),
                message: "tool failed".into(),
            }),
            Some(Action::SignatureComparisonFailed {
                request,
                message: "tool failed".into(),
            })
        );
    }

    #[test]
    fn pkgdata_model_typed_events_map_inventory_detail_partial_and_failure() {
        let inventory_request = PackageInventoryRequest { generation: 7 };
        let package = PackageSummary {
            identity: PackageIdentity::new("busybox"),
            recipe: PackageField::Available("busybox".into()),
            provider: PackageField::Available("/layers/core/recipes-core/busybox.bb".into()),
            version: PackageField::Available("1.37.0".into()),
            installed_size_bytes: PackageField::Available(1_024),
            license: PackageField::Available("GPL-2.0-only".into()),
            image_membership: PackageField::Available(vec!["core-image-minimal".into()]),
        };
        assert_eq!(
            model_action_from_backend_event(BackendEvent::PackageInventory {
                request: inventory_request,
                packages: vec![package.clone()],
                limitations: Vec::new(),
            }),
            Some(Action::PackageInventoryLoaded {
                request: inventory_request,
                packages: vec![package],
            })
        );
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::PackageInventory {
                request: inventory_request,
                packages: Vec::new(),
                limitations: vec!["pkgdata directory is incomplete".into()],
            }),
            Some(Action::PackageInventoryPartial { .. })
        ));
        assert_eq!(
            model_action_from_backend_event(BackendEvent::PackageInventoryFailed {
                request: inventory_request,
                message: "pkgdata directory is missing".into(),
            }),
            Some(Action::PackageInventoryFailed {
                request: inventory_request,
                message: "pkgdata directory is missing".into(),
            })
        );

        let detail_request = PackageDetailRequest {
            identity: PackageIdentity::new("busybox"),
            generation: 3,
        };
        let detail = PackageDetail {
            identity: detail_request.identity.clone(),
            files: PackageField::Available(vec!["/bin/busybox".into()]),
            runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
            reverse_dependencies: PackageField::Unavailable,
        };
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::PackageDetail {
                request: detail_request.clone(),
                detail: detail.clone(),
                limitations: vec!["reverse dependencies unavailable".into()],
            }),
            Some(Action::PackageDetailPartial { .. })
        ));
        assert_eq!(
            model_action_from_backend_event(BackendEvent::PackageDetailFailed {
                request: detail_request.clone(),
                message: "package was not found".into(),
            }),
            Some(Action::PackageDetailFailed {
                request: detail_request,
                message: "package was not found".into(),
            })
        );
    }

    #[test]
    fn image_artifact_model_typed_events_map_success_partial_and_failure() {
        let request = ImageArtifactRequest {
            generation: 9,
            machine: "qemux86-64".into(),
        };
        let artifact = ImageArtifact {
            identity: ImageArtifactIdentity {
                machine: request.machine.clone(),
                image: "core-image-minimal".into(),
                path: "/build/tmp/deploy/images/qemux86-64/core-image-minimal.wic".into(),
            },
            kind: ImageArtifactKind::Wic,
            size_bytes: ImageArtifactField::Available(8_192),
            modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
            checksums: ImageArtifactField::Unavailable,
            manifests: ImageArtifactField::Available(Vec::new()),
            licenses: ImageArtifactField::Unavailable,
            spdx: ImageArtifactField::Unavailable,
            wic_files: ImageArtifactField::Available(Vec::new()),
        };
        let inventory = ImageArtifactInventory {
            machine: request.machine.clone(),
            deploy_directory: ImageArtifactField::Available(
                "/build/tmp/deploy/images/qemux86-64".into(),
            ),
            artifacts: vec![artifact],
        };
        assert_eq!(
            model_action_from_backend_event(BackendEvent::ImageArtifacts {
                request: request.clone(),
                inventory: inventory.clone(),
                limitations: Vec::new(),
            }),
            Some(Action::ImageArtifactInventoryLoaded {
                request: request.clone(),
                inventory: inventory.clone(),
            })
        );
        assert_eq!(
            model_action_from_backend_event(BackendEvent::ImageArtifacts {
                request: request.clone(),
                inventory: inventory.clone(),
                limitations: vec!["license metadata unavailable".into()],
            }),
            Some(Action::ImageArtifactInventoryPartial {
                request: request.clone(),
                inventory,
                limitations: vec!["license metadata unavailable".into()],
            })
        );
        assert_eq!(
            model_action_from_backend_event(BackendEvent::ImageArtifactsFailed {
                request: request.clone(),
                message: "deploy directory is missing".into(),
            }),
            Some(Action::ImageArtifactInventoryFailed {
                request,
                message: "deploy directory is missing".into(),
            })
        );
    }

    #[test]
    fn image_artifact_adapter_response_crosses_the_app_boundary_as_typed_action() {
        let request = ImageArtifactRequest {
            generation: 12,
            machine: "qemux86-64".into(),
        };
        let inventory = ImageArtifactInventory {
            machine: request.machine.clone(),
            deploy_directory: ImageArtifactField::Available(
                "/build/tmp/deploy/images/qemux86-64".into(),
            ),
            artifacts: Vec::new(),
        };
        let event: BackendEvent = yoctui_bitbake::ImageArtifactResponse {
            request: request.clone(),
            inventory: inventory.clone(),
            limitations: vec!["one symlink was not followed".into()],
        }
        .into();
        assert_eq!(
            model_action_from_backend_event(event),
            Some(Action::ImageArtifactInventoryPartial {
                request,
                inventory,
                limitations: vec!["one symlink was not followed".into()],
            })
        );
    }

    #[test]
    fn pkgdata_adapter_responses_cross_the_app_boundary_as_typed_actions() {
        let inventory_request = PackageInventoryRequest { generation: 11 };
        let package = PackageSummary {
            identity: PackageIdentity::new("busybox"),
            recipe: PackageField::Available("busybox".into()),
            provider: PackageField::Unavailable,
            version: PackageField::Available("1.37.0-r0".into()),
            installed_size_bytes: PackageField::Available(1_024),
            license: PackageField::Available("GPL-2.0-only".into()),
            image_membership: PackageField::Unavailable,
        };
        let event: BackendEvent = yoctui_bitbake::PackageInventoryResponse {
            request: inventory_request,
            packages: vec![package.clone()],
            limitations: vec!["provider path unavailable".into()],
        }
        .into();
        assert_eq!(
            model_action_from_backend_event(event),
            Some(Action::PackageInventoryPartial {
                request: inventory_request,
                packages: vec![package],
                limitations: vec!["provider path unavailable".into()],
            })
        );

        let request = PackageDetailRequest {
            identity: PackageIdentity::new("busybox"),
            generation: 4,
        };
        let detail = PackageDetail {
            identity: request.identity.clone(),
            files: PackageField::Available(vec!["/bin/busybox".into()]),
            runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
            reverse_dependencies: PackageField::Available(Vec::new()),
        };
        let event: BackendEvent = yoctui_bitbake::PackageDetailResponse {
            request: request.clone(),
            detail: detail.clone(),
            limitations: Vec::new(),
        }
        .into();
        assert_eq!(
            model_action_from_backend_event(event),
            Some(Action::PackageDetailLoaded { request, detail })
        );
    }

    #[test]
    fn pkgdata_workspace_maps_search_navigation_refresh_and_context_actions() {
        assert_eq!(
            package_workspace_action(false, Input::Up),
            Some(Action::SelectPackage { delta: -1 })
        );
        assert_eq!(
            package_workspace_action(false, Input::Enter),
            Some(Action::BeginSelectedPackageDetail)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('R')),
            Some(Action::RefreshPackageInventory)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('D')),
            Some(Action::TogglePackageDependencyKind)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char(']')),
            Some(Action::SelectPackageDependency { delta: 1 })
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('d')),
            Some(Action::OpenSelectedPackageDependency)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('o')),
            Some(Action::OpenSelectedPackageRecipe)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('e')),
            Some(Action::OpenSelectedPackageProvider)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('c')),
            Some(Action::CancelPackageOperation)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('/')),
            Some(Action::BeginPackageSearch)
        );
        assert_eq!(
            package_workspace_action(true, Input::Char('b')),
            Some(Action::AppendPackageQuery('b'))
        );
        assert_eq!(
            package_workspace_action(true, Input::Backspace),
            Some(Action::BackspacePackageQuery)
        );
        assert_eq!(
            package_workspace_action(true, Input::Esc),
            Some(Action::FinishPackageSearch)
        );
        assert_eq!(package_workspace_action(false, Input::Char('x')), None);
    }

    #[test]
    fn signature_adapter_responses_cross_the_app_boundary_as_typed_actions() {
        let target = SignatureTarget {
            recipe: "busybox".into(),
            task: "do_compile".into(),
        };
        let identity = SignatureIdentity {
            target: target.clone(),
            hash: Some("aaa".into()),
            path: Some("/build/tmp/stamps/busybox/do_compile.sigdata.aaa".into()),
        };
        let record = SignatureRecord {
            identity: identity.clone(),
            base_hash: Some("base-aaa".into()),
            task_hash: Some("aaa".into()),
            variables: Vec::new(),
            dependencies: Vec::new(),
        };
        let event: BackendEvent = yoctui_bitbake::SignatureDumpResponse {
            target: target.clone(),
            records: vec![record.clone()],
            limitations: vec!["one malformed historical signature was omitted".into()],
        }
        .into();
        assert_eq!(
            model_action_from_backend_event(event),
            Some(Action::SignatureDumpPartial {
                target,
                records: vec![record],
                limitations: vec!["one malformed historical signature was omitted".into()],
            })
        );

        let request = SignatureComparisonRequest {
            left: identity.clone(),
            right: SignatureIdentity {
                hash: Some("bbb".into()),
                ..identity
            },
        };
        let difference = SignatureDifference {
            category: SignatureDifferenceCategory::BaseHash,
            key: "base_hash".into(),
            left: Some("base-aaa".into()),
            right: Some("base-bbb".into()),
        };
        let event: BackendEvent = yoctui_bitbake::SignatureComparisonResponse {
            request: request.clone(),
            differences: vec![difference.clone()],
            limitations: Vec::new(),
        }
        .into();
        assert_eq!(
            model_action_from_backend_event(event),
            Some(Action::SignatureComparisonLoaded {
                request,
                differences: vec![difference],
            })
        );
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
    fn dependency_workspace_maps_typed_navigation_refresh_and_open_actions() {
        assert_eq!(
            dependency_workspace_action(Input::Up),
            Some(Action::SelectDependencyGraphNode { delta: -1 })
        );
        assert_eq!(
            dependency_workspace_action(Input::Char('j')),
            Some(Action::SelectDependencyGraphNode { delta: 1 })
        );
        assert_eq!(
            dependency_workspace_action(Input::Enter),
            Some(Action::OpenSelectedDependencyRecipe)
        );
        assert_eq!(
            dependency_workspace_action(Input::Char('o')),
            Some(Action::OpenSelectedDependencyProvider)
        );
        assert_eq!(
            dependency_workspace_action(Input::Char('L')),
            Some(Action::OpenSelectedDependencyTaskLog)
        );
        assert_eq!(
            dependency_workspace_action(Input::Char('r')),
            Some(Action::RefreshDependencyGraph)
        );
        assert_eq!(dependency_workspace_action(Input::Char('x')), None);
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
    fn devtool_modify_routes_confirmation_and_workspace_editor_build_keys() {
        assert_eq!(
            devtool_modify_confirmation_action(Input::Enter),
            Some(Action::ConfirmDevtoolModify)
        );
        assert_eq!(
            devtool_modify_confirmation_action(Input::Esc),
            Some(Action::CancelDevtoolModify)
        );
        assert_eq!(devtool_modify_confirmation_action(Input::Char('b')), None);
        assert_eq!(
            recipe_editor_action(false, Input::CtrlB),
            Some(Action::BeginRecipeEditorBuild)
        );
        assert_eq!(
            recipe_editor_action(false, Input::Enter),
            Some(Action::ToggleRecipeEditorEditing)
        );
        assert_eq!(
            recipe_editor_action(true, Input::Enter),
            Some(Action::AppendRecipeEditor('\n'))
        );
    }

    #[test]
    fn devtool_publish_update_routes_only_confirmation_keys() {
        assert_eq!(
            devtool_update_confirmation_action(Input::Enter),
            Some(Action::ConfirmDevtoolUpdateRecipe)
        );
        assert_eq!(
            devtool_update_confirmation_action(Input::Esc),
            Some(Action::CancelDevtoolUpdateRecipe)
        );
        assert_eq!(devtool_update_confirmation_action(Input::Char('u')), None);
    }

    #[test]
    fn devtool_publish_finish_routes_picker_and_confirmation_keys() {
        assert_eq!(
            devtool_finish_picker_action(Input::Up),
            Some(Action::SelectDevtoolFinishLayer { delta: -1 })
        );
        assert_eq!(
            devtool_finish_picker_action(Input::Down),
            Some(Action::SelectDevtoolFinishLayer { delta: 1 })
        );
        assert_eq!(
            devtool_finish_picker_action(Input::Enter),
            Some(Action::PreviewDevtoolFinish)
        );
        assert_eq!(
            devtool_finish_confirmation_action(Input::Enter),
            Some(Action::ConfirmDevtoolFinish)
        );
        assert_eq!(
            devtool_finish_confirmation_action(Input::Esc),
            Some(Action::CancelDevtoolFinishConfirmation)
        );
    }

    #[test]
    fn devtool_target_deploy_routes_entry_and_confirmation_keys() {
        assert_eq!(
            devtool_deploy_dialog_action(Input::Char('q')),
            Some(Action::AppendDevtoolDeployTarget('q'))
        );
        assert_eq!(
            devtool_deploy_dialog_action(Input::Backspace),
            Some(Action::BackspaceDevtoolDeployTarget)
        );
        assert_eq!(
            devtool_deploy_dialog_action(Input::Enter),
            Some(Action::PreviewDevtoolDeploy)
        );
        assert_eq!(
            devtool_deploy_confirmation_action(Input::Enter),
            Some(Action::ConfirmDevtoolDeploy)
        );
        assert_eq!(
            devtool_deploy_confirmation_action(Input::Esc),
            Some(Action::CancelDevtoolDeployConfirmation)
        );
    }

    #[test]
    fn devtool_target_reset_routes_only_destructive_confirmation_keys() {
        assert_eq!(
            devtool_reset_confirmation_action(Input::Enter),
            Some(Action::ConfirmDevtoolReset)
        );
        assert_eq!(
            devtool_reset_confirmation_action(Input::Esc),
            Some(Action::CancelDevtoolReset)
        );
        assert_eq!(devtool_reset_confirmation_action(Input::Char('D')), None);
    }

    #[test]
    fn devtool_job_lifecycle_maps_runner_events_and_stays_independent_from_bitbake() {
        let now = SystemTime::UNIX_EPOCH;
        let mut devtool = DevtoolJobCoordinator::default();
        let operation = DevtoolOperation::Reset {
            recipe: "busybox".into(),
        };
        let actions = devtool.queue(operation.clone(), now).unwrap();
        let id = devtool.active_job_id().unwrap();
        assert_eq!(id, BackgroundJobId(1_u64 << 63));
        assert_eq!(devtool.active_operation(), Some(&operation));
        assert!(devtool.queue(operation, now).is_none());

        let mut build = BuildJobCoordinator::default();
        let build_actions = build
            .queue_build(
                &BuildRequest {
                    targets: vec!["core-image-minimal".into()],
                    task: None,
                    force: false,
                },
                now,
            )
            .unwrap();
        assert_eq!(build.active_job_id(), Some(BackgroundJobId(1)));
        assert_ne!(build.active_job_id(), devtool.active_job_id());

        let mut app = yoctui_model::App::new(10, 1_000);
        for action in actions.into_iter().chain(build_actions) {
            let _ = yoctui_model::update(&mut app, action);
        }
        for action in devtool.actions_for_event(DevtoolRunnerEvent::Started, now) {
            let _ = yoctui_model::update(&mut app, action);
        }
        for action in devtool.actions_for_event(
            DevtoolRunnerEvent::Output {
                stream: DevtoolOutputStream::Stderr,
                line: "progress".into(),
                truncated: true,
            },
            now,
        ) {
            let _ = yoctui_model::update(&mut app, action);
        }
        app.screen = Screen::Dashboard;
        for action in
            devtool.actions_for_event(DevtoolRunnerEvent::Completed { exit_code: Some(0) }, now)
        {
            let _ = yoctui_model::update(&mut app, action);
        }
        let job = app.background_jobs.get(id).unwrap();
        assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
        assert_eq!(job.output[0].source, BackgroundJobOutputSource::Stderr);
        assert!(job.output[0].truncated);
        assert_eq!(app.screen, Screen::Dashboard);
        assert_eq!(build.active_job_id(), Some(BackgroundJobId(1)));
    }

    #[test]
    fn devtool_job_lifecycle_maps_start_failure_cancel_failure_cancel_and_loss() {
        let now = SystemTime::UNIX_EPOCH;
        let operation = DevtoolOperation::Reset {
            recipe: "busybox".into(),
        };

        let mut coordinator = DevtoolJobCoordinator::default();
        let id = {
            let _ = coordinator.queue(operation.clone(), now);
            coordinator.active_job_id().unwrap()
        };
        assert!(matches!(
            coordinator.start_failed("missing".into(), now).as_slice(),
            [Action::FailBackgroundJob { id: failed, .. }] if *failed == id
        ));

        let mut coordinator = DevtoolJobCoordinator::default();
        let _ = coordinator.queue(operation.clone(), now);
        assert!(matches!(
            coordinator.request_cancellation(),
            Some(Action::RequestBackgroundJobCancellation { .. })
        ));
        assert!(coordinator.request_cancellation().is_none());
        let rejected = coordinator.cancellation_failed("signal".into(), now);
        assert!(matches!(
            rejected.last(),
            Some(Action::RejectBackgroundJobCancellation { .. })
        ));
        assert!(matches!(
            coordinator
                .actions_for_event(
                    DevtoolRunnerEvent::Cancelled {
                        forced: true,
                        exit_code: None,
                    },
                    now,
                )
                .last(),
            Some(Action::CancelBackgroundJob { .. })
        ));

        let mut coordinator = DevtoolJobCoordinator::default();
        let _ = coordinator.queue(operation, now);
        assert!(matches!(
            coordinator
                .actions_for_event(
                    DevtoolRunnerEvent::Lost {
                        message: "channel".into(),
                    },
                    now,
                )
                .as_slice(),
            [Action::LoseBackgroundJob { .. }]
        ));
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
    fn signature_workspace_maps_recipe_entry_picker_and_workspace_keys() {
        assert_eq!(
            recipes_workspace_action(false, Input::Char('Z')),
            Some(Action::BeginSelectedRecipeSignatures)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('z')),
            Some(Action::BeginSelectedRecipeDiffsigs)
        );
        assert_eq!(
            signature_task_picker_action(Input::Down),
            Some(Action::SelectSignatureTask { delta: 1 })
        );
        assert_eq!(
            signature_task_picker_action(Input::Enter),
            Some(Action::ConfirmSignatureTask)
        );
        assert_eq!(
            signature_task_picker_action(Input::Esc),
            Some(Action::CancelSignatureTaskPicker)
        );
        assert_eq!(
            signature_workspace_action(Input::Up),
            Some(Action::SelectSignatureRecord { delta: -1 })
        );
        assert_eq!(
            signature_workspace_action(Input::Char('1')),
            Some(Action::SetSelectedSignatureComparisonSide(
                yoctui_model::SignatureComparisonSide::Left
            ))
        );
        assert_eq!(
            signature_workspace_action(Input::Char('2')),
            Some(Action::SetSelectedSignatureComparisonSide(
                yoctui_model::SignatureComparisonSide::Right
            ))
        );
        assert_eq!(
            signature_workspace_action(Input::Char('c')),
            Some(Action::BeginSignatureComparison)
        );
        assert_eq!(
            signature_workspace_action(Input::Char('r')),
            Some(Action::RefreshSignatureDump)
        );
        assert_eq!(
            signature_workspace_action(Input::Char('e')),
            Some(Action::OpenSignatureProvider)
        );
        assert_eq!(
            signature_workspace_action(Input::Esc),
            Some(Action::LeaveSignatureWorkspace)
        );
        assert_eq!(signature_workspace_action(Input::Char('x')), None);
    }
    #[test]
    fn images_workspace_image_action_maps_search_refresh_build_cancel_and_open_actions() {
        assert_eq!(
            images_workspace_action(false, Input::Up),
            Some(Action::SelectImageArtifact { delta: -1 })
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('R')),
            Some(Action::RefreshImageArtifactInventory)
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('b')),
            Some(Action::BeginSelectedImageArtifactBuild)
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('c')),
            Some(Action::CancelImageArtifactOperation)
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('o')),
            Some(Action::OpenSelectedImageArtifact)
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('m')),
            Some(Action::OpenSelectedImageArtifactAssociation(
                yoctui_model::ImageArtifactAssociation::Manifest
            ))
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('/')),
            Some(Action::BeginImageArtifactSearch)
        );
        assert_eq!(
            images_workspace_action(true, Input::Char('w')),
            Some(Action::AppendImageArtifactQuery('w'))
        );
        assert_eq!(
            images_workspace_action(true, Input::Esc),
            Some(Action::FinishImageArtifactSearch)
        );
    }
    #[test]
    fn qemu_model_normalizes_typed_runner_events_without_parsing_output() {
        let id = QemuSessionId(7);
        let timestamp = SystemTime::UNIX_EPOCH;
        assert_eq!(
            qemu_actions_for_runner_event(id, QemuRunnerEvent::Starting, timestamp),
            vec![Action::QemuSessionStarting {
                id,
                started_at: timestamp
            }]
        );
        assert_eq!(
            qemu_actions_for_runner_event(
                id,
                QemuRunnerEvent::Output {
                    stream: QemuRunnerOutputStream::Stderr,
                    line: "verbatim runner output".into(),
                    truncated: true,
                },
                timestamp,
            ),
            vec![Action::AppendQemuSessionOutput {
                id,
                stream: QemuOutputStream::Stderr,
                line: "verbatim runner output".into(),
                truncated: true,
                timestamp,
            }]
        );
        assert_eq!(
            qemu_actions_for_runner_event(
                id,
                QemuRunnerEvent::Failed {
                    message: "spawn failed".into(),
                    exit_code: Some(127),
                },
                timestamp,
            ),
            vec![Action::FailQemuSession {
                id,
                message: "spawn failed".into(),
                exit_code: Some(127),
                finished_at: timestamp,
            }]
        );
        assert_eq!(
            qemu_actions_for_runner_event(
                id,
                QemuRunnerEvent::CancellationRejected {
                    message: "not running".into(),
                },
                timestamp,
            ),
            vec![Action::RejectQemuSessionCancellation {
                id,
                message: "not running".into(),
            }]
        );
    }
    #[test]
    fn qemu_adapter_normalizes_forced_cancellation_and_loss() {
        let id = QemuSessionId(11);
        let timestamp = SystemTime::UNIX_EPOCH;
        assert_eq!(
            qemu_actions_for_runner_event(
                id,
                QemuRunnerEvent::Cancelled {
                    forced: true,
                    exit_code: Some(137),
                },
                timestamp,
            ),
            vec![
                Action::AppendQemuSessionOutput {
                    id,
                    stream: QemuOutputStream::Stderr,
                    line: "runqemu cancellation required forced termination".into(),
                    truncated: false,
                    timestamp,
                },
                Action::CancelQemuSession {
                    id,
                    exit_code: Some(137),
                    finished_at: timestamp,
                }
            ]
        );
        assert_eq!(
            qemu_actions_for_runner_event(
                id,
                QemuRunnerEvent::Lost {
                    message: "event channel lost".into(),
                },
                timestamp,
            ),
            vec![Action::LoseQemuSession {
                id,
                message: "event channel lost".into(),
                finished_at: timestamp,
            }]
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
