#[derive(Debug)]
pub struct DevtoolJobCoordinator {
    pub(crate) next_job_id: u64,
    pub(crate) active_job: Option<BackgroundJobId>,
    pub(crate) active_operation: Option<DevtoolOperation>,
    pub(crate) cancellation_requested: bool,
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
            DevtoolOperation::UpdateRecipePatch { destination, .. } => {
                ("update-recipe patches", None, Some(destination.clone()))
            }
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
            DevtoolOperation::Upgrade { .. } => ("upgrade", None, None),
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
