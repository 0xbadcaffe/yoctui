//! Test coordinator.
use super::*;

pub(crate) struct TestSessionCliOperation {
    pub(crate) id: TestSessionId,
    pub(crate) runner: TestRunnerJob,
}

pub(crate) struct TestResultImportCliOperation {
    pub(crate) request: yoctui_model::TestResultImportRequest,
    pub(crate) deadline: tokio::time::Instant,
    pub(crate) handle: tokio::task::JoinHandle<
        std::result::Result<yoctui_bitbake::TestResultImportResponse, String>,
    >,
}

pub(crate) struct TestResultCliOperation {
    pub(crate) runner: TestResultJob,
    pub(crate) operation: TestResultOperation,
    pub(crate) comparison: Option<TestComparison>,
}

pub(crate) struct TestCliCoordinator {
    pub(crate) runner_adapter: TestRunnerAdapter,
    pub(crate) result_adapter: TestResultAdapter,
    pub(crate) session: Option<TestSessionCliOperation>,
    pub(crate) import: Option<TestResultImportCliOperation>,
    pub(crate) result: Option<TestResultCliOperation>,
}

impl TestCliCoordinator {
    pub(crate) fn new(
        build_directory: PathBuf,
        path_directories: Vec<PathBuf>,
        ptest: yoctui_model::PtestCapability,
    ) -> Self {
        Self {
            runner_adapter: TestRunnerAdapter::new(
                build_directory,
                path_directories.clone(),
                ptest,
            ),
            result_adapter: TestResultAdapter::new(path_directories),
            session: None,
            import: None,
            result: None,
        }
    }

    pub(crate) async fn handle_effect(&mut self, app: &mut App, effect: Effect) -> bool {
        match effect {
            Effect::InspectTestCapability => {
                let next = update(
                    app,
                    Action::TestCapabilityLoaded(self.runner_adapter.capability()),
                );
                if matches!(next, Some(Effect::InspectResultToolCapability)) {
                    let _ = update(
                        app,
                        Action::ResultToolCapabilityLoaded(self.result_adapter.capability()),
                    );
                }
                true
            }
            Effect::InspectResultToolCapability => {
                let _ = update(
                    app,
                    Action::ResultToolCapabilityLoaded(self.result_adapter.capability()),
                );
                true
            }
            Effect::StartTestSession { id, operation } => {
                self.begin_session(app, id, operation).await;
                true
            }
            Effect::CancelTestSession(id)
                if self.session.as_ref().is_some_and(|active| active.id == id) =>
            {
                let result = self
                    .session
                    .as_mut()
                    .expect("exact active Testing session was checked")
                    .runner
                    .cancel()
                    .await;
                if let Err(error) = result {
                    let _ = update(
                        app,
                        Action::RejectTestSessionCancellation {
                            id,
                            message: error.to_string(),
                        },
                    );
                }
                true
            }
            Effect::ImportTestResults(request) => {
                if let Some(previous) = self.import.take() {
                    previous.handle.abort();
                }
                let adapter = self.result_adapter.clone();
                let owned_request = request.clone();
                self.import = Some(TestResultImportCliOperation {
                    request,
                    deadline: tokio::time::Instant::now() + Duration::from_secs(30),
                    handle: tokio::task::spawn_blocking(move || {
                        adapter
                            .import(&owned_request)
                            .map_err(|error| error.to_string())
                    }),
                });
                true
            }
            Effect::CompareTestResults(request) => {
                self.begin_comparison(app, request).await;
                true
            }
            Effect::InspectTestJunitDestination {
                result,
                destination,
            } => {
                let inspection = self.result_adapter.inspect_junit_destination(destination);
                let _ = update(
                    app,
                    Action::TestJunitDestinationInspected { result, inspection },
                );
                true
            }
            Effect::ExportTestJunit(request) => {
                self.begin_junit(app, request).await;
                true
            }
            _ => false,
        }
    }

    pub(crate) async fn begin_session(
        &mut self,
        app: &mut App,
        id: TestSessionId,
        operation: TestOperation,
    ) {
        if self.session.is_some() {
            let _ = update(
                app,
                Action::FailTestSession {
                    id,
                    message: "another selftest runner is already active".into(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
            return;
        }
        let TestOperation::Selftest(request) = operation else {
            let _ = update(
                app,
                Action::FailTestSession {
                    id,
                    message: "managed BitBake Testing reached the selftest runner".into(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
            return;
        };
        let command = match self.runner_adapter.command(&request) {
            Ok(command) => command,
            Err(error) => {
                let _ = update(
                    app,
                    Action::FailTestSession {
                        id,
                        message: error.to_string(),
                        exit_code: None,
                        finished_at: SystemTime::now(),
                    },
                );
                return;
            }
        };
        let mut runner = TestRunnerJob::new();
        if let Err(error) = runner.start(command).await {
            let _ = update(
                app,
                Action::FailTestSession {
                    id,
                    message: error.to_string(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
            return;
        }
        self.session = Some(TestSessionCliOperation { id, runner });
    }

    pub(crate) async fn begin_comparison(
        &mut self,
        app: &mut App,
        request: yoctui_model::TestComparisonRequest,
    ) {
        if self.result.is_some() {
            let _ = update(
                app,
                Action::TestComparisonFailed {
                    request,
                    message: "another resulttool operation is already active".into(),
                },
            );
            return;
        }
        let baseline = app
            .test_results
            .records()
            .iter()
            .find(|record| record.identity == request.baseline)
            .cloned();
        let candidate = app
            .test_results
            .records()
            .iter()
            .find(|record| record.identity == request.candidate)
            .cloned();
        let Some((baseline, candidate)) = baseline.zip(candidate) else {
            let _ = update(
                app,
                Action::TestComparisonFailed {
                    request,
                    message: "an exact comparison input is unavailable".into(),
                },
            );
            return;
        };
        let preview = self
            .result_adapter
            .capability()
            .executable()
            .and_then(|executable| {
                yoctui_model::TestComparisonPreview::new(executable, request.clone())
            });
        let command = preview.map_err(str::to_owned).and_then(|preview| {
            self.result_adapter
                .comparison_command(&preview, &baseline, &candidate)
                .map_err(|error| error.to_string())
        });
        let comparison = TestComparison::between(&baseline, &candidate);
        let (command, comparison) = match (command, comparison.map_err(str::to_owned)) {
            (Ok(command), Ok(comparison)) => (command, comparison),
            (Err(message), _) | (_, Err(message)) => {
                let _ = update(app, Action::TestComparisonFailed { request, message });
                return;
            }
        };
        let mut runner = TestResultJob::new();
        if let Err(error) = runner.start(command).await {
            let _ = update(
                app,
                Action::TestComparisonFailed {
                    request,
                    message: error.to_string(),
                },
            );
            return;
        }
        self.result = Some(TestResultCliOperation {
            runner,
            operation: TestResultOperation::Comparison(request),
            comparison: Some(comparison),
        });
    }

    pub(crate) async fn begin_junit(
        &mut self,
        app: &mut App,
        request: yoctui_model::TestJunitExportRequest,
    ) {
        if self.result.is_some() {
            let _ = update(
                app,
                Action::TestJunitExportFailed {
                    request,
                    message: "another resulttool operation is already active".into(),
                },
            );
            return;
        }
        let record = app
            .test_results
            .records()
            .iter()
            .find(|record| record.identity == request.result)
            .cloned();
        let Some(record) = record else {
            let _ = update(
                app,
                Action::TestJunitExportFailed {
                    request,
                    message: "the exact JUnit source result is unavailable".into(),
                },
            );
            return;
        };
        let preview = self
            .result_adapter
            .capability()
            .executable()
            .and_then(|executable| {
                yoctui_model::TestJunitExportPreview::new(executable, request.clone())
            });
        let command = preview.map_err(str::to_owned).and_then(|preview| {
            self.result_adapter
                .junit_command(&preview, &record)
                .map_err(|error| error.to_string())
        });
        let command = match command {
            Ok(command) => command,
            Err(message) => {
                let _ = update(app, Action::TestJunitExportFailed { request, message });
                return;
            }
        };
        let mut runner = TestResultJob::new();
        if let Err(error) = runner.start(command).await {
            let _ = update(
                app,
                Action::TestJunitExportFailed {
                    request,
                    message: error.to_string(),
                },
            );
            return;
        }
        self.result = Some(TestResultCliOperation {
            runner,
            operation: TestResultOperation::Junit(request),
            comparison: None,
        });
    }

    pub(crate) async fn poll(&mut self, app: &mut App) {
        let mut followups = Vec::new();
        if let Some(operation) = self.session.as_mut()
            && let Ok(event) =
                tokio::time::timeout(Duration::from_millis(1), operation.runner.next_event()).await
        {
            let terminal = event.is_err()
                || matches!(
                    event,
                    Ok(TestRunnerEvent::Completed { .. }
                        | TestRunnerEvent::Failed { .. }
                        | TestRunnerEvent::Cancelled { .. }
                        | TestRunnerEvent::TimedOut { .. }
                        | TestRunnerEvent::Lost { .. })
                );
            match event {
                Ok(event) => {
                    for action in
                        test_actions_for_runner_event(operation.id, event, SystemTime::now())
                    {
                        if let Some(effect) = compatibility_workspace_action(app, action) {
                            followups.push(effect);
                        }
                    }
                }
                Err(error) => {
                    let _ = update(
                        app,
                        Action::LoseTestSession {
                            id: operation.id,
                            message: error.to_string(),
                            finished_at: SystemTime::now(),
                        },
                    );
                }
            }
            if terminal {
                self.session = None;
            }
        }
        if self
            .import
            .as_ref()
            .is_some_and(|operation| tokio::time::Instant::now() >= operation.deadline)
        {
            let operation = self.import.take().expect("expired import was checked");
            operation.handle.abort();
            let _ = update(
                app,
                Action::TestResultsTimedOut {
                    request: operation.request,
                },
            );
        } else if self
            .import
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            let operation = self.import.take().expect("finished import was checked");
            let action = match operation.handle.await {
                Ok(Ok(response)) => test_results_import_action(response),
                Ok(Err(message)) => Action::TestResultsFailed {
                    request: operation.request,
                    message,
                },
                Err(error) if error.is_cancelled() => Action::TestResultsCancelled {
                    request: operation.request,
                },
                Err(error) => Action::TestResultsLost {
                    request: operation.request,
                    message: error.to_string(),
                },
            };
            if let Some(effect) = compatibility_workspace_action(app, action) {
                followups.push(effect);
            }
        }
        if let Some(operation) = self.result.as_mut()
            && let Ok(event) =
                tokio::time::timeout(Duration::from_millis(1), operation.runner.next_event()).await
        {
            let terminal = event.is_err()
                || matches!(
                    event,
                    Ok(TestResultRunnerEvent::Completed { .. }
                        | TestResultRunnerEvent::Failed { .. }
                        | TestResultRunnerEvent::Cancelled { .. }
                        | TestResultRunnerEvent::TimedOut { .. }
                        | TestResultRunnerEvent::Lost { .. })
                );
            let event = match event {
                Ok(event) => event,
                Err(error) => TestResultRunnerEvent::Lost {
                    operation: Some(operation.operation.clone()),
                    message: error.to_string(),
                },
            };
            for action in test_result_actions_for_runner_event(
                event,
                operation.comparison.clone(),
                Vec::new(),
            ) {
                if let Some(effect) = compatibility_workspace_action(app, action) {
                    followups.push(effect);
                }
            }
            if terminal {
                self.result = None;
            }
        }
        for effect in followups {
            let _ = self.handle_effect(app, effect).await;
        }
    }
}
