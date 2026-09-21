use super::*;

#[derive(Debug)]
pub struct BuildJobCoordinator {
    pub(crate) next_job_id: u64,
    pub(crate) active_job: Option<BackgroundJobId>,
    pub(crate) active_kind: Option<BackgroundJobKind>,
    pub(crate) cancellation_requested: bool,
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
            Some(task @ ("testimage" | "testsdk" | "testsdkext")) => (
                BackgroundJobKind::Test,
                format!("Test {}:{task}", request.targets.join(" ")),
                Screen::Testing,
                None,
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
            | BackendEvent::RootfsComposition { .. }
            | BackendEvent::RootfsCompositionUnavailable { .. }
            | BackendEvent::RootfsCompositionFailed { .. }
            | BackendEvent::RecipeSources { .. }
            | BackendEvent::RecipeMetadata(_)
            | BackendEvent::LayerRelationships(_)
            | BackendEvent::ParseProgress { .. }
            | BackendEvent::TaskStats(_)
            | BackendEvent::SstateSummary(_)
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
