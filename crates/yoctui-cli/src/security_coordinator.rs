//! Security coordinator.
use super::*;

pub(crate) struct SecurityCapabilityCliOperation {
    pub(crate) handle: tokio::task::JoinHandle<
        std::result::Result<yoctui_model::SecurityCapabilitySnapshot, String>,
    >,
}

pub(crate) struct SecurityReportCliOperation {
    pub(crate) request: SecurityReportRequest,
    pub(crate) cancellation: SecurityReportCancellation,
    pub(crate) handle: tokio::task::JoinHandle<
        std::result::Result<yoctui_bitbake::SecurityReportResponse, SecurityReportAdapterError>,
    >,
}

pub(crate) struct SecurityMapperCliOperation {
    pub(crate) id: SecuritySessionId,
    pub(crate) runner: SecurityMapperJobRunner,
}

pub(crate) struct SecurityCliCoordinator {
    pub(crate) build_directory: PathBuf,
    pub(crate) path_directories: Vec<PathBuf>,
    pub(crate) report_adapter: SecurityReportAdapter,
    pub(crate) capability: Option<SecurityCapabilityCliOperation>,
    pub(crate) report: Option<SecurityReportCliOperation>,
    pub(crate) mapper: Option<SecurityMapperCliOperation>,
}

impl SecurityCliCoordinator {
    pub(crate) fn new(build_directory: PathBuf, path_directories: Vec<PathBuf>) -> Self {
        Self {
            build_directory,
            path_directories,
            report_adapter: SecurityReportAdapter::new(),
            capability: None,
            report: None,
            mapper: None,
        }
    }

    pub(crate) fn owns_mapper(&self, id: SecuritySessionId) -> bool {
        self.mapper.as_ref().is_some_and(|active| active.id == id)
    }

    pub(crate) async fn handle_effect(&mut self, app: &mut App, effect: Effect) -> bool {
        let Effect::Security(effect) = effect else {
            return false;
        };
        match effect {
            SecurityEffect::InspectCapability => self.begin_capability_inspection(app),
            SecurityEffect::StartPackageMap {
                id,
                executable,
                arguments,
            } => {
                self.begin_mapper(app, id, executable, arguments).await;
            }
            SecurityEffect::CancelSession(id) => self.cancel_mapper(app, id).await,
            SecurityEffect::ImportReports(request) => self.begin_report_scan(request),
            SecurityEffect::OpenPath(_) | SecurityEffect::OpenUrl(_) => return false,
            SecurityEffect::StartBuild { .. } => return false,
        }
        true
    }

    pub(crate) fn begin_capability_inspection(&mut self, app: &mut App) {
        if let Some(stale) = self.capability.take() {
            stale.handle.abort();
        }
        let input = security_capability_input(
            app,
            self.build_directory.clone(),
            self.path_directories.clone(),
        );
        let handle = tokio::task::spawn_blocking(move || {
            let input = input?;
            SecurityCapabilityInspector::new(input)
                .inspect()
                .map_err(|error| error.to_string())
        });
        self.capability = Some(SecurityCapabilityCliOperation { handle });
    }

    pub(crate) fn begin_report_scan(&mut self, request: SecurityReportRequest) {
        if let Some(stale) = self.report.take() {
            stale.cancellation.cancel();
            stale.handle.abort();
        }
        let cancellation = SecurityReportCancellation::default();
        let worker_cancellation = cancellation.clone();
        let worker_request = request.clone();
        let adapter = self.report_adapter.clone();
        let handle = tokio::spawn(async move {
            adapter
                .scan_with_cancellation(worker_request, worker_cancellation)
                .await
        });
        self.report = Some(SecurityReportCliOperation {
            request,
            cancellation,
            handle,
        });
    }

    pub(crate) async fn begin_mapper(
        &mut self,
        app: &mut App,
        id: SecuritySessionId,
        executable: PathBuf,
        arguments: Vec<String>,
    ) {
        if self.owns_mapper(id) {
            let _ = update(
                app,
                Action::Notify(
                    "The exact Security package-mapping process is already owned by the CLI."
                        .into(),
                ),
            );
            return;
        }
        if self.mapper.is_some() {
            let _ = update(
                app,
                Action::Security(SecurityAction::FailSession {
                    id,
                    message: "another Security package-mapping process is already active".into(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        }
        let preview = app
            .security
            .sessions
            .iter()
            .find(|session| session.preview.id == id)
            .map(|session| session.preview.clone());
        let Some(preview) = preview else {
            let _ = update(
                app,
                Action::Security(SecurityAction::LoseSession {
                    id,
                    message: "the exact Security package-mapping session is unavailable".into(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        };
        if !matches!(
            &preview.operation,
            SecurityOperation::PackageMap {
                executable: expected_executable,
                arguments: expected_arguments,
            } if expected_executable == &executable && expected_arguments == &arguments
        ) {
            let _ = update(
                app,
                Action::Security(SecurityAction::FailSession {
                    id,
                    message: "Security package-mapping effect does not match its preview".into(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        }
        let command = match SecurityMapperCommandSpec::from_preview(&preview) {
            Ok(command) => command,
            Err(error) => {
                let _ = update(
                    app,
                    Action::Security(SecurityAction::FailSession {
                        id,
                        message: error.to_string(),
                        finished_at: SystemTime::now(),
                    }),
                );
                return;
            }
        };
        let mut runner = SecurityMapperJobRunner::new();
        if let Err(error) = runner.start(command).await {
            let _ = update(
                app,
                Action::Security(SecurityAction::FailSession {
                    id,
                    message: error.to_string(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        }
        self.mapper = Some(SecurityMapperCliOperation { id, runner });
    }

    pub(crate) async fn cancel_mapper(&mut self, app: &mut App, id: SecuritySessionId) {
        let Some(active) = self.mapper.as_mut().filter(|active| active.id == id) else {
            let _ = update(
                app,
                Action::Security(SecurityAction::RejectCancellation {
                    id,
                    message: "the CLI does not own this Security package-mapping process".into(),
                }),
            );
            return;
        };
        if let Err(error) = active.runner.cancel(id).await {
            let _ = update(
                app,
                Action::Security(SecurityAction::RejectCancellation {
                    id,
                    message: error.to_string(),
                }),
            );
        }
    }

    pub(crate) async fn poll(&mut self, app: &mut App) {
        self.poll_capability(app).await;
        self.poll_report(app).await;
        self.poll_mapper(app).await;
    }

    pub(crate) async fn poll_capability(&mut self, app: &mut App) {
        if !self
            .capability
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return;
        }
        let operation = self.capability.take().expect("finished capability checked");
        let action = match operation.handle.await {
            Ok(Ok(capability)) => SecurityAction::CapabilityLoaded(capability),
            Ok(Err(message)) => SecurityAction::CapabilityFailed(message),
            Err(error) => SecurityAction::CapabilityFailed(format!(
                "Security capability task was lost: {error}"
            )),
        };
        let _ = update(app, Action::Security(action));
    }

    pub(crate) async fn poll_report(&mut self, app: &mut App) {
        if !self
            .report
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return;
        }
        let operation = self.report.take().expect("finished report scan checked");
        let action = match operation.handle.await {
            Ok(Ok(response)) => match response.outcome {
                SecurityReportScanOutcome::Empty => SecurityAction::ReportsLoaded {
                    request: response.request,
                    reports: Vec::new(),
                    limitations: Vec::new(),
                },
                SecurityReportScanOutcome::Complete(reports) => SecurityAction::ReportsLoaded {
                    request: response.request,
                    reports,
                    limitations: Vec::new(),
                },
                SecurityReportScanOutcome::Partial {
                    reports,
                    limitations,
                } => SecurityAction::ReportsLoaded {
                    request: response.request,
                    reports,
                    limitations,
                },
            },
            Ok(Err(SecurityReportAdapterError::Cancelled)) => {
                SecurityAction::ReportsCancelled(operation.request)
            }
            Ok(Err(SecurityReportAdapterError::Timeout(_))) => {
                SecurityAction::ReportsTimedOut(operation.request)
            }
            Ok(Err(SecurityReportAdapterError::WorkerLost(message))) => {
                SecurityAction::ReportsLost {
                    request: operation.request,
                    message,
                }
            }
            Ok(Err(error)) => SecurityAction::ReportsFailed {
                request: operation.request,
                message: error.to_string(),
            },
            Err(error) if error.is_cancelled() => {
                SecurityAction::ReportsCancelled(operation.request)
            }
            Err(error) => SecurityAction::ReportsLost {
                request: operation.request,
                message: error.to_string(),
            },
        };
        let _ = update(app, Action::Security(action));
    }

    pub(crate) async fn poll_mapper(&mut self, app: &mut App) {
        let mut followups = Vec::new();
        let Some(operation) = self.mapper.as_mut() else {
            return;
        };
        let result =
            tokio::time::timeout(Duration::from_millis(1), operation.runner.next_event()).await;
        let event = match result {
            Ok(Ok(event)) => Some(event),
            Ok(Err(error)) => Some(SecurityMapperRunnerEvent::Lost {
                id: operation.id,
                message: error.to_string(),
            }),
            Err(_) => None,
        };
        let Some(event) = event else {
            return;
        };
        let terminal = matches!(
            event,
            SecurityMapperRunnerEvent::Completed { .. }
                | SecurityMapperRunnerEvent::Failed { .. }
                | SecurityMapperRunnerEvent::Cancelled { .. }
                | SecurityMapperRunnerEvent::TimedOut { .. }
                | SecurityMapperRunnerEvent::Lost { .. }
        );
        for action in security_actions_for_mapper_event(event, SystemTime::now()) {
            if let Some(effect) = compatibility_workspace_action(app, action) {
                followups.push(effect);
            }
        }
        if terminal {
            self.mapper = None;
        }
        for effect in followups {
            let _ = self.handle_effect(app, effect).await;
        }
    }

    pub(crate) async fn revalidate_open_path(&self, app: &App, path: &Path) -> Result<(), String> {
        let selected_identity = app
            .security
            .selected_report()
            .map(|report| report.identity().clone())
            .filter(|identity| identity.path == path);
        if let Some(identity) = selected_identity {
            let request =
                SecurityReportRequest::new(1, vec![path.to_path_buf()]).map_err(str::to_owned)?;
            let response = self
                .report_adapter
                .scan(request)
                .await
                .map_err(|error| error.to_string())?;
            if response
                .outcome
                .reports()
                .iter()
                .any(|report| report.identity() == &identity)
            {
                return Ok(());
            }
            return Err("the selected Security report changed before it could be opened".into());
        }
        let provider = app.security.scope.as_ref().and_then(|scope| match scope {
            SecurityScope::Recipe(identity) => Some(identity.file.as_path()),
            SecurityScope::Image { .. } => None,
        });
        if provider != Some(path) {
            return Err(
                "the requested path is not the selected Security report or provider".into(),
            );
        }
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("could not inspect the Security provider: {error}"))?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || fs::canonicalize(path).ok().as_deref() != Some(path)
        {
            return Err(
                "the selected Security provider is no longer a canonical regular file".into(),
            );
        }
        Ok(())
    }

    pub(crate) fn url_opener(&self) -> Option<PathBuf> {
        self.path_directories.iter().find_map(|directory| {
            let candidate = directory.join("xdg-open");
            let metadata = fs::symlink_metadata(&candidate).ok()?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || fs::canonicalize(&candidate).ok().as_ref() != Some(&candidate)
            {
                return None;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o111 == 0 {
                    return None;
                }
            }
            Some(candidate)
        })
    }
}
