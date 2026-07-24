//! Application-owned input mapping, keeping terminal concerns outside the reducer.
use std::time::SystemTime;
use yoctui_bitbake::BackendEvent;
use yoctui_model::{
    Action, AppError, BackgroundJobContext, BackgroundJobError, BackgroundJobId, BackgroundJobKind,
    BackgroundJobOutputEntry, BackgroundJobProgress, BackgroundJobResult, BackgroundJobSpec,
    BuildRequest, FocusTarget, Screen, Severity, TaskId, TaskInfo,
};

#[derive(Debug)]
pub struct BuildJobCoordinator {
    next_job_id: u64,
    active_job: Option<BackgroundJobId>,
    cancellation_requested: bool,
}
impl Default for BuildJobCoordinator {
    fn default() -> Self {
        Self {
            next_job_id: 1,
            active_job: None,
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
        let title = match request.task.as_deref() {
            Some(task) => format!("Build {}:{task}", request.targets.join(" ")),
            None => format!("Build {}", request.targets.join(" ")),
        };
        Some(vec![
            Action::QueueBackgroundJob(BackgroundJobSpec {
                id,
                kind: BackgroundJobKind::Build,
                title,
                context: BackgroundJobContext {
                    workspace: Some(Screen::Tasks),
                    target,
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
                            summary: "BitBake build completed successfully".into(),
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
            | BackendEvent::LayerRelationships(_)
            | BackendEvent::ParseProgress { .. }
            | BackendEvent::TaskStarted { .. }
            | BackendEvent::TaskProgress { .. }
            | BackendEvent::TaskCompleted { .. } => Vec::new(),
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
        BackendEvent::TaskStarted { recipe, task } => {
            let id = TaskId(format!("{recipe}:{task}"));
            Some(Action::TaskStarted(TaskInfo {
                id,
                recipe,
                task,
                progress: None,
            }))
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
        BackendEvent::Recipes(_)
        | BackendEvent::Layers(_)
        | BackendEvent::Variable { .. }
        | BackendEvent::Dependencies { .. }
        | BackendEvent::RecipeSources { .. }
        | BackendEvent::LayerRelationships(_) => None,
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
        Input::Enter => Some(Action::DismissNotification),
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
            severity: Severity::Warning,
            message: "cache miss".into(),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: None,
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
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
    fn enter_dismisses_notification() {
        assert_eq!(key_action(Input::Enter), Some(Action::DismissNotification));
    }
    #[test]
    fn maps_severity_filter_control() {
        assert_eq!(key_action(Input::Char('s')), Some(Action::CycleLogSeverity));
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
