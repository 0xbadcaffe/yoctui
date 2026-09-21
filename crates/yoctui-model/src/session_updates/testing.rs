pub(crate) fn next_test_session_id(app: &mut App) -> TestSessionId {
    app.test_session_generation = app.test_session_generation.wrapping_add(1).max(1);
    TestSessionId(app.test_session_generation)
}

pub(crate) fn test_background_job_id(id: TestSessionId) -> BackgroundJobId {
    BackgroundJobId(TEST_BACKGROUND_JOB_NAMESPACE | id.0)
}

pub(crate) fn test_job_id(app: &App, id: TestSessionId) -> Option<BackgroundJobId> {
    app.test_session(id)
        .and_then(|session| session.background_job_id)
}

pub(crate) fn mutate_test_session(
    app: &mut App,
    id: TestSessionId,
    mutation: impl FnOnce(&mut TestSession),
) -> Option<Option<BackgroundJobId>> {
    let session = app
        .test_sessions
        .iter_mut()
        .find(|session| session.id == id)?;
    mutation(session);
    Some(session.background_job_id)
}

pub(crate) fn note_stale_test_event(app: &mut App) {
    app.background_jobs.ignored_transitions += 1;
}

pub(crate) fn test_launch_draft(app: &App, family: TestFamily) -> TestLaunchDraft {
    TestLaunchDraft::new(
        family,
        app.workspace
            .variables
            .get("MACHINE")
            .cloned()
            .unwrap_or_default(),
        app.workspace
            .variables
            .get("DISTRO")
            .cloned()
            .unwrap_or_default(),
        app.build.target.clone().unwrap_or_default(),
    )
}

pub(crate) fn test_preview_is_current(app: &App, preview: &TestLaunchPreview) -> bool {
    match preview {
        TestLaunchPreview::Selftest(request) => {
            TestSelftestRequest::new(
                request.executable.clone(),
                request.family,
                request.selector.clone(),
                request.parallelism,
                request.verbose,
                request.skip_network,
            )
            .as_ref()
                == Ok(request)
                && app.test_capability.executable_for(request.family).as_ref()
                    == Ok(&request.executable)
        }
        TestLaunchPreview::Build {
            family, request, ..
        } => {
            test_launch_draft(app, *family)
                .preview(&app.test_capability)
                .as_ref()
                .is_ok_and(|current| current == preview)
                && request.validate().is_ok()
        }
    }
}

pub(crate) fn queue_test_session(app: &mut App, operation: TestOperation) -> Option<Effect> {
    if app.active_test_session().is_some() {
        app.notification = Some("A managed Testing operation is already active.".into());
        return None;
    }
    while app.test_sessions.len() >= MAX_TEST_SESSIONS {
        let Some(index) = app.test_sessions.iter().position(|session| {
            session.background_job_id.is_some_and(|job_id| {
                app.background_jobs
                    .get(job_id)
                    .is_none_or(|job| job.status.is_terminal())
            })
        }) else {
            app.notification = Some("The Testing session history is full.".into());
            return None;
        };
        app.test_sessions.remove(index);
    }
    let id = next_test_session_id(app);
    let background_job_id =
        matches!(&operation, TestOperation::Selftest(_)).then(|| test_background_job_id(id));
    if let Some(job_id) = background_job_id {
        let family = operation.family();
        app.background_jobs.queue(BackgroundJobSpec {
            id: job_id,
            kind: BackgroundJobKind::Test,
            title: family.label().into(),
            context: BackgroundJobContext {
                workspace: Some(Screen::Testing),
                target: match &operation {
                    TestOperation::Selftest(request) => request.selector.clone(),
                    TestOperation::Build { request, .. } => request.targets.first().cloned(),
                },
                task: family.task().map(str::to_owned),
                image: match &operation {
                    TestOperation::Build { request, .. } => request.targets.first().cloned(),
                    TestOperation::Selftest(_) => None,
                },
                ..BackgroundJobContext::default()
            },
            cancellation_supported: true,
            queued_at: SystemTime::now(),
        });
        if app.background_jobs.get(job_id).is_none() {
            app.notification = Some("The Testing operation could not be queued.".into());
            return None;
        }
    }
    app.test_sessions.push_back(TestSession {
        id,
        background_job_id,
        operation: operation.clone(),
        exit_code: None,
        result_paths: Vec::new(),
        error_detail: None,
        outcome: None,
    });
    match operation {
        TestOperation::Selftest(_) => Some(Effect::StartTestSession { id, operation }),
        TestOperation::Build { family, request } => Some(Effect::StartTestBuildSession {
            id,
            family,
            request,
        }),
    }
}

pub(crate) fn next_test_result_generation(app: &mut App) -> u64 {
    app.test_result_generation = app.test_result_generation.wrapping_add(1).max(1);
    app.test_result_generation
}

pub(crate) fn begin_test_result_import(app: &mut App, roots: Vec<PathBuf>) -> Option<Effect> {
    let generation = next_test_result_generation(app);
    let request = match TestResultImportRequest::new(generation, roots) {
        Ok(request) => request,
        Err(message) => {
            app.notification = Some(format!("Test results are unavailable: {message}."));
            return None;
        }
    };
    app.test_results = TestResultInventoryState::Loading {
        request: request.clone(),
    };
    Some(Effect::ImportTestResults(request))
}

pub(crate) fn test_result_request_is_current(app: &App, request: &TestResultImportRequest) -> bool {
    matches!(
        &app.test_results,
        TestResultInventoryState::Loading { request: current } if current == request
    )
}

pub(crate) fn set_test_result_selection_to_current_or_first(
    app: &mut App,
    previous: Option<TestResultIdentity>,
) {
    let visible = app
        .filtered_test_results()
        .into_iter()
        .map(|record| record.identity.clone())
        .collect::<Vec<_>>();
    app.test_result_selection = previous
        .filter(|identity| visible.contains(identity))
        .or_else(|| visible.first().cloned());
    if app.test_case_selection.as_ref().is_some_and(|identity| {
        app.selected_test_result()
            .and_then(|record| record.case(identity))
            .is_none()
    }) {
        app.test_case_selection = None;
        app.test_result_drilled = false;
    }
}

pub(crate) fn test_comparison_inputs_exist(app: &App, request: &TestComparisonRequest) -> bool {
    app.test_results
        .records()
        .iter()
        .any(|record| record.identity == request.baseline)
        && app
            .test_results
            .records()
            .iter()
            .any(|record| record.identity == request.candidate)
}

pub(crate) fn test_comparison_request_is_current(
    app: &App,
    request: &TestComparisonRequest,
) -> bool {
    matches!(
        &app.test_comparison,
        TestComparisonState::Loading { request: current } if current == request
    ) && test_comparison_inputs_exist(app, request)
}

pub(crate) fn set_test_comparison_selection(app: &mut App) {
    let identities = app
        .test_comparison_transitions()
        .iter()
        .map(|transition| transition.identity.clone())
        .collect::<Vec<_>>();
    app.test_comparison_selection = app
        .test_comparison_selection
        .take()
        .filter(|identity| identities.contains(identity))
        .or_else(|| identities.first().cloned());
}

pub(crate) fn test_junit_request_is_current(app: &App, request: &TestJunitExportRequest) -> bool {
    app.test_results
        .records()
        .iter()
        .any(|record| record.identity == request.result)
        && match &app.test_junit_export {
            TestJunitExportState::Running(current) => current == request,
            _ => false,
        }
}

pub(crate) const MAX_QEMU_SESSIONS: usize = 32;
pub(crate) const QEMU_BACKGROUND_JOB_NAMESPACE: u64 = 3 << 62;

