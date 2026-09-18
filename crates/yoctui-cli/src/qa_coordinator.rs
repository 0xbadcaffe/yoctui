//! Qa coordinator.
use super::*;

pub(crate) struct QaCapabilityCliOperation {
    pub(crate) handle: tokio::task::JoinHandle<
        std::result::Result<yoctui_bitbake::QaTaskCapabilityResponse, String>,
    >,
}

pub(crate) struct QaLayerCapabilityCliOperation {
    pub(crate) handle: tokio::task::JoinHandle<
        std::result::Result<yoctui_bitbake::QaLayerCapabilityResponse, String>,
    >,
}

pub(crate) struct QaReportCliOperation {
    pub(crate) request: QaReportRequest,
    pub(crate) cancellation: QaReportCancellation,
    pub(crate) handle: tokio::task::JoinHandle<
        std::result::Result<yoctui_bitbake::QaReportResponse, QaReportAdapterError>,
    >,
}

pub(crate) struct QaLayerCliOperation {
    pub(crate) id: QaLayerSessionId,
    pub(crate) runner: QaLayerJobRunner,
}

pub(crate) struct QaCliCoordinator {
    pub(crate) build_directory: PathBuf,
    pub(crate) path_directories: Vec<PathBuf>,
    pub(crate) report_adapter: QaReportAdapter,
    pub(crate) capability: Option<QaCapabilityCliOperation>,
    pub(crate) layer_capability: Option<QaLayerCapabilityCliOperation>,
    pub(crate) report: Option<QaReportCliOperation>,
    pub(crate) layer: Option<QaLayerCliOperation>,
}

impl QaCliCoordinator {
    pub(crate) fn new(build_directory: PathBuf, path_directories: Vec<PathBuf>) -> Self {
        Self {
            build_directory,
            path_directories,
            report_adapter: QaReportAdapter::new(),
            capability: None,
            layer_capability: None,
            report: None,
            layer: None,
        }
    }

    pub(crate) fn owns_layer(&self, id: QaLayerSessionId) -> bool {
        self.layer.as_ref().is_some_and(|active| active.id == id)
    }

    pub(crate) async fn handle_effect(&mut self, app: &mut App, effect: Effect) -> bool {
        let Effect::Qa(effect) = effect else {
            return false;
        };
        match effect {
            QaEffect::InspectCapability { scope } => {
                self.begin_capability_inspection(app, scope);
            }
            QaEffect::InspectLayerCapability => self.begin_layer_capability_inspection(app),
            QaEffect::ImportReports(request) => self.begin_report_scan(app, request),
            QaEffect::StartLayerCheck {
                session,
                layer,
                executable,
                arguments,
            } => {
                self.begin_layer_check(app, session, layer, executable, arguments)
                    .await;
            }
            QaEffect::CancelLayerCheck(id) => self.cancel_layer_check(app, id).await,
            QaEffect::StartBuild { .. }
            | QaEffect::CancelBuild { .. }
            | QaEffect::OpenReport(_)
            | QaEffect::OpenProvider(_)
            | QaEffect::OpenSource(_)
            | QaEffect::OpenLayerRoot(_) => return false,
        }
        true
    }

    pub(crate) fn begin_capability_inspection(&mut self, app: &App, scope: Option<QaScope>) {
        if let Some(stale) = self.capability.take() {
            stale.handle.abort();
        }
        let input = qa_task_capability_input(app, self.build_directory.clone(), scope);
        let handle = tokio::task::spawn_blocking(move || {
            let input = input?;
            QaTaskCapabilityInspector::new(input)
                .inspect()
                .map_err(|error| error.to_string())
        });
        self.capability = Some(QaCapabilityCliOperation { handle });
    }

    pub(crate) fn begin_layer_capability_inspection(&mut self, app: &App) {
        if let Some(stale) = self.layer_capability.take() {
            stale.handle.abort();
        }
        let input = qa_layer_capability_input(
            app,
            self.build_directory.clone(),
            self.path_directories.clone(),
        );
        let handle = tokio::task::spawn_blocking(move || {
            let input = input?;
            QaLayerCapabilityInspector::inspect(input).map_err(|error| error.to_string())
        });
        self.layer_capability = Some(QaLayerCapabilityCliOperation { handle });
    }

    pub(crate) fn begin_report_scan(&mut self, app: &App, request: QaReportRequest) {
        if let Some(stale) = self.report.take() {
            stale.cancellation.cancel();
            stale.handle.abort();
        }
        let input = qa_report_scan_input(app, self.build_directory.clone(), request.clone());
        let cancellation = QaReportCancellation::default();
        let worker_cancellation = cancellation.clone();
        let adapter = self.report_adapter.clone();
        let handle = tokio::spawn(async move {
            match input {
                Ok(input) => {
                    adapter
                        .scan_with_cancellation(input, worker_cancellation)
                        .await
                }
                Err(message) => Err(QaReportAdapterError::InvalidRequest(message)),
            }
        });
        self.report = Some(QaReportCliOperation {
            request,
            cancellation,
            handle,
        });
    }

    pub(crate) async fn begin_layer_check(
        &mut self,
        app: &mut App,
        id: QaLayerSessionId,
        layer: QaLayerIdentity,
        executable: yoctui_model::QaExecutableIdentity,
        arguments: Vec<String>,
    ) {
        if self.owns_layer(id) {
            let _ = update(
                app,
                Action::Notify("The exact layer-QA process is already owned by the CLI.".into()),
            );
            return;
        }
        if self.layer.is_some() {
            let _ = update(
                app,
                Action::Qa(QaAction::FailLayerSession {
                    session: id,
                    exit_code: None,
                    message: "another layer-QA process is already active".into(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        }
        let preview = app
            .qa
            .layer_sessions
            .iter()
            .find(|session| session.id == id)
            .map(|session| session.operation.clone());
        let Some(preview) = preview else {
            let _ = update(
                app,
                Action::Qa(QaAction::LoseLayerSession {
                    session: id,
                    message: "the exact layer-QA session is unavailable".into(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        };
        if preview.layer != layer
            || preview.executable != executable
            || preview.arguments != arguments
        {
            let _ = update(
                app,
                Action::Qa(QaAction::FailLayerSession {
                    session: id,
                    exit_code: None,
                    message: "layer-QA effect does not match its confirmed preview".into(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        }
        let command = match QaLayerCommandSpec::from_preview(id, &preview) {
            Ok(command) => command,
            Err(error) => {
                let _ = update(
                    app,
                    Action::Qa(QaAction::FailLayerSession {
                        session: id,
                        exit_code: None,
                        message: error.to_string(),
                        finished_at: SystemTime::now(),
                    }),
                );
                return;
            }
        };
        let mut runner = QaLayerJobRunner::new();
        if let Err(error) = runner.start(command).await {
            let _ = update(
                app,
                Action::Qa(QaAction::FailLayerSession {
                    session: id,
                    exit_code: None,
                    message: error.to_string(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        }
        self.layer = Some(QaLayerCliOperation { id, runner });
    }

    pub(crate) async fn cancel_layer_check(&mut self, app: &mut App, id: QaLayerSessionId) {
        let Some(active) = self.layer.as_mut().filter(|active| active.id == id) else {
            let _ = update(
                app,
                Action::Qa(QaAction::RejectLayerCancellation {
                    session: id,
                    message: "the CLI does not own this layer-QA process".into(),
                }),
            );
            return;
        };
        if let Err(error) = active.runner.cancel(id).await {
            let _ = update(
                app,
                Action::Qa(QaAction::RejectLayerCancellation {
                    session: id,
                    message: error.to_string(),
                }),
            );
        }
    }

    pub(crate) async fn poll(&mut self, app: &mut App) {
        self.poll_capability(app).await;
        self.poll_layer_capability(app).await;
        self.poll_report(app).await;
        self.poll_layer(app).await;
    }

    pub(crate) async fn poll_capability(&mut self, app: &mut App) {
        if !self
            .capability
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return;
        }
        let operation = self
            .capability
            .take()
            .expect("finished QA capability checked");
        let action = match operation.handle.await {
            Ok(Ok(response)) => qa_task_capability_action(response),
            Ok(Err(message)) => Action::Qa(QaAction::CapabilityFailed(message)),
            Err(error) => Action::Qa(QaAction::CapabilityFailed(format!(
                "QA capability task was lost: {error}"
            ))),
        };
        let _ = update(app, action);
    }

    pub(crate) async fn poll_layer_capability(&mut self, app: &mut App) {
        if !self
            .layer_capability
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return;
        }
        let operation = self
            .layer_capability
            .take()
            .expect("finished layer-QA capability checked");
        let action = match operation.handle.await {
            Ok(Ok(response)) => qa_layer_capability_action(response),
            Ok(Err(message)) => Action::Qa(QaAction::LayerCapabilityFailed(message)),
            Err(error) => Action::Qa(QaAction::LayerCapabilityFailed(format!(
                "layer-QA capability task was lost: {error}"
            ))),
        };
        let _ = update(app, action);
    }

    pub(crate) async fn poll_report(&mut self, app: &mut App) {
        if !self
            .report
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return;
        }
        let operation = self.report.take().expect("finished QA report scan checked");
        let action = match operation.handle.await {
            Ok(Ok(response)) => qa_report_response_action(response),
            Ok(Err(error)) => qa_report_error_action(operation.request, error),
            Err(error) if error.is_cancelled() => {
                qa_report_error_action(operation.request, QaReportAdapterError::Cancelled)
            }
            Err(error) => qa_report_error_action(
                operation.request,
                QaReportAdapterError::WorkerLost(error.to_string()),
            ),
        };
        let _ = update(app, action);
    }

    pub(crate) async fn poll_layer(&mut self, app: &mut App) {
        let Some(operation) = self.layer.as_mut() else {
            return;
        };
        let result =
            tokio::time::timeout(Duration::from_millis(1), operation.runner.next_event()).await;
        let event = match result {
            Ok(Ok(event)) => Some(event),
            Ok(Err(error)) => Some(QaLayerRunnerEvent::Lost {
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
            QaLayerRunnerEvent::Completed { .. }
                | QaLayerRunnerEvent::Failed { .. }
                | QaLayerRunnerEvent::Cancelled { .. }
                | QaLayerRunnerEvent::TimedOut { .. }
                | QaLayerRunnerEvent::Lost { .. }
        );
        let action = match event {
            QaLayerRunnerEvent::Completed {
                id,
                exit_code: Some(exit_code),
            } => {
                let result_paths = app
                    .qa
                    .layer_sessions
                    .iter()
                    .find(|session| session.id == id)
                    .map(|session| session.operation.report_roots.clone())
                    .unwrap_or_default();
                Some(Action::Qa(QaAction::CompleteLayerSession {
                    session: id,
                    exit_code,
                    result_paths,
                    finished_at: SystemTime::now(),
                }))
            }
            event => qa_layer_runner_action(event, SystemTime::now()),
        };
        let followup = action.and_then(|action| compatibility_workspace_action(app, action));
        if terminal {
            self.layer = None;
        }
        if let Some(effect) = followup {
            let _ = self.handle_effect(app, effect).await;
        }
    }

    pub(crate) fn revalidate_report(
        &self,
        app: &App,
        identity: &QaReportIdentity,
    ) -> Result<(), String> {
        let retained = app
            .qa
            .inventory
            .reports()
            .unwrap_or_default()
            .iter()
            .any(|report| &report.identity == identity);
        if !retained {
            return Err("the exact QA report is no longer retained".into());
        }
        self.report_adapter
            .revalidate(identity)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn revalidate_provider(
        &self,
        app: &App,
        identity: &RecipeIdentity,
    ) -> Result<(), String> {
        let retained = app.qa.capability.snapshot().is_some_and(|snapshot| {
            snapshot
                .scopes
                .iter()
                .any(|scope| &scope.recipe == identity)
        });
        if !retained {
            return Err("the exact QA provider scope is no longer retained".into());
        }
        revalidate_canonical_regular_file(&identity.file, "QA provider")
    }

    pub(crate) fn revalidate_source(
        &self,
        app: &App,
        source: &QaSourceLocation,
    ) -> Result<(), String> {
        let retained = app
            .qa
            .inventory
            .reports()
            .unwrap_or_default()
            .iter()
            .flat_map(|report| report.findings.iter())
            .any(|finding| finding.source.as_ref() == Some(source));
        if !retained {
            return Err("the exact QA finding source is no longer retained".into());
        }
        revalidate_canonical_regular_file(&source.path, "QA finding source")
    }

    pub(crate) fn revalidate_layer(
        &self,
        app: &App,
        layer: &QaLayerIdentity,
    ) -> Result<(), String> {
        let retained = app.qa.layer_capability.snapshot().is_some_and(|snapshot| {
            snapshot
                .layers
                .iter()
                .any(|candidate| &candidate.identity == layer)
        });
        if !retained {
            return Err("the exact configured layer is no longer retained".into());
        }
        revalidate_canonical_directory(&layer.root, "configured QA layer")
    }
}
