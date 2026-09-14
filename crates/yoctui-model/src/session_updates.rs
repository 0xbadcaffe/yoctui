//! Session updates.
use super::*;

pub(crate) fn begin_sdk_artifact_inventory(app: &mut App) -> Option<Effect> {
    let Some(root) = app.workspace.variables.get("SDK_DEPLOY").map(PathBuf::from) else {
        app.notification =
            Some("SDK artifacts are unavailable because SDK_DEPLOY was not reported.".into());
        return None;
    };
    let machine = app
        .workspace
        .variables
        .get("MACHINE")
        .cloned()
        .unwrap_or_default();
    app.sdk_artifact_generation = app.sdk_artifact_generation.wrapping_add(1).max(1);
    let request = SdkArtifactInventoryRequest {
        generation: app.sdk_artifact_generation,
        root,
        machine,
    };
    if let Err(message) = request.validate() {
        app.notification = Some(format!("SDK artifacts are unavailable: {message}."));
        return None;
    }
    app.sdk_artifacts = SdkArtifactInventoryState::Loading {
        request: request.clone(),
    };
    Some(Effect::GetSdkArtifacts(request))
}

pub(crate) fn set_sdk_artifact_selection_to_current_or_first(
    app: &mut App,
    previous: Option<SdkArtifactIdentity>,
) {
    let visible = app
        .filtered_sdk_artifacts()
        .into_iter()
        .map(|artifact| artifact.identity.clone())
        .collect::<Vec<_>>();
    app.sdk_artifact_selection = previous
        .filter(|identity| visible.contains(identity))
        .or_else(|| visible.first().cloned());
}

pub(crate) const MAX_SDK_SESSIONS: usize = 32;
pub(crate) const SDK_BACKGROUND_JOB_NAMESPACE: u64 = 1 << 62;

pub(crate) fn next_sdk_session_id(app: &mut App) -> SdkSessionId {
    app.sdk_session_generation = app.sdk_session_generation.wrapping_add(1).max(1);
    SdkSessionId(app.sdk_session_generation)
}

pub(crate) fn sdk_background_job_id(id: SdkSessionId) -> BackgroundJobId {
    BackgroundJobId(SDK_BACKGROUND_JOB_NAMESPACE | id.0)
}

pub(crate) fn sdk_job_id(app: &App, id: SdkSessionId) -> Option<BackgroundJobId> {
    app.sdk_session(id).map(|session| session.background_job_id)
}

pub(crate) fn mutate_sdk_session(
    app: &mut App,
    id: SdkSessionId,
    mutation: impl FnOnce(&mut SdkSession),
) -> Option<BackgroundJobId> {
    let session = app
        .sdk_sessions
        .iter_mut()
        .find(|session| session.id == id)?;
    let job_id = session.background_job_id;
    mutation(session);
    Some(job_id)
}

pub(crate) fn note_stale_sdk_event(app: &mut App) {
    app.background_jobs.ignored_transitions += 1;
}

pub(crate) fn queue_sdk_session(app: &mut App, operation: SdkOperation) -> Option<Effect> {
    if app.active_sdk_session().is_some() {
        app.notification = Some("A managed SDK tool operation is already active.".into());
        return None;
    }
    while app.sdk_sessions.len() >= MAX_SDK_SESSIONS {
        let Some(index) = app.sdk_sessions.iter().position(|session| {
            app.background_jobs
                .get(session.background_job_id)
                .is_none_or(|job| job.status.is_terminal())
        }) else {
            app.notification = Some("The SDK operation history is full.".into());
            return None;
        };
        app.sdk_sessions.remove(index);
    }
    let id = next_sdk_session_id(app);
    let background_job_id = sdk_background_job_id(id);
    let (title, path) = match &operation {
        SdkOperation::Publish(request) => (
            format!("Publish SDK {}", request.artifact.path.display()),
            Some(request.artifact.path.clone()),
        ),
        SdkOperation::Native(request) => (
            format!("SDK native {}", request.recipe),
            request.extracted_root.clone(),
        ),
    };
    app.background_jobs.queue(BackgroundJobSpec {
        id: background_job_id,
        kind: BackgroundJobKind::Sdk,
        title,
        context: BackgroundJobContext {
            workspace: Some(Screen::Sdk),
            path,
            ..BackgroundJobContext::default()
        },
        cancellation_supported: true,
        queued_at: SystemTime::now(),
    });
    if app.background_jobs.get(background_job_id).is_none() {
        app.notification = Some("The SDK operation could not be queued.".into());
        return None;
    }
    app.sdk_sessions.push_back(SdkSession {
        id,
        background_job_id,
        operation: operation.clone(),
        exit_code: None,
        error_detail: None,
    });
    Some(Effect::StartSdkSession { id, operation })
}

pub(crate) const MAX_TEST_SESSIONS: usize = 32;
pub(crate) const TEST_BACKGROUND_JOB_NAMESPACE: u64 = 3 << 60;

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

pub(crate) fn next_qemu_session_id(app: &mut App) -> QemuSessionId {
    app.qemu_session_generation = app.qemu_session_generation.wrapping_add(1);
    if app.qemu_session_generation == 0 {
        app.qemu_session_generation = 1;
    }
    QemuSessionId(app.qemu_session_generation)
}

pub(crate) fn qemu_background_job_id(id: QemuSessionId) -> BackgroundJobId {
    BackgroundJobId(QEMU_BACKGROUND_JOB_NAMESPACE | id.0)
}

pub(crate) fn qemu_job_id(app: &App, id: QemuSessionId) -> Option<BackgroundJobId> {
    app.qemu_session(id)
        .map(|session| session.background_job_id)
}

pub(crate) fn mutate_qemu_session(
    app: &mut App,
    id: QemuSessionId,
    mutation: impl FnOnce(&mut QemuSession),
) -> Option<BackgroundJobId> {
    let session = app
        .qemu_sessions
        .iter_mut()
        .find(|session| session.id == id)?;
    let job_id = session.background_job_id;
    mutation(session);
    Some(job_id)
}

pub(crate) fn note_stale_qemu_event(app: &mut App) {
    app.background_jobs.ignored_transitions += 1;
}

pub(crate) const MAX_WIC_SESSIONS: usize = 32;
pub(crate) const WIC_BACKGROUND_JOB_NAMESPACE: u64 = 2 << 62;

pub(crate) fn next_wic_session_id(app: &mut App) -> WicSessionId {
    app.wic_session_generation = app.wic_session_generation.wrapping_add(1);
    if app.wic_session_generation == 0 {
        app.wic_session_generation = 1;
    }
    WicSessionId(app.wic_session_generation)
}

pub(crate) fn wic_background_job_id(id: WicSessionId) -> BackgroundJobId {
    BackgroundJobId(WIC_BACKGROUND_JOB_NAMESPACE | id.0)
}

pub(crate) fn wic_job_id(app: &App, id: WicSessionId) -> Option<BackgroundJobId> {
    app.wic_session(id).map(|session| session.background_job_id)
}

pub(crate) fn mutate_wic_session(
    app: &mut App,
    id: WicSessionId,
    mutation: impl FnOnce(&mut WicSession),
) -> Option<BackgroundJobId> {
    let session = app
        .wic_sessions
        .iter_mut()
        .find(|session| session.id == id)?;
    let job_id = session.background_job_id;
    mutation(session);
    Some(job_id)
}

pub(crate) fn note_stale_wic_event(app: &mut App) {
    app.background_jobs.ignored_transitions += 1;
}

pub(crate) fn is_uncompressed_wic_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".wic") || name.ends_with(".direct"))
}

pub(crate) fn reconcile_wic_output_selection(app: &mut App) {
    let rows = app.wic_output_rows();
    let selected_is_present = app
        .wic_output_selection
        .as_ref()
        .is_some_and(|selected| rows.iter().any(|row| &row.identity == selected));
    if !selected_is_present {
        app.wic_output_selection = rows.first().map(|row| row.identity.clone());
    }
}

pub(crate) fn reconcile_wic_device_selection(app: &mut App) {
    let rows = app.wic_device_rows();
    let selected_is_present = app
        .wic_device_selection
        .as_ref()
        .is_some_and(|selected| rows.iter().any(|row| &row.identity == selected));
    if !selected_is_present {
        app.wic_device_selection = rows.first().map(|row| row.identity.clone());
    }
}

pub(crate) fn current_wic_write_preview(
    app: &App,
    request: &WicDeviceInventoryRequest,
    device_identity: &WicDeviceIdentity,
    phrase: &str,
) -> Result<WicWritePreview, String> {
    let (active_request, devices) = match &app.wic_devices {
        WicDeviceInventoryState::Available {
            request, devices, ..
        }
        | WicDeviceInventoryState::Partial {
            request, devices, ..
        } => (request, devices),
        _ => return Err("The Wic device inventory is unavailable.".into()),
    };
    if active_request != request {
        return Err("The Wic device inventory request is stale.".into());
    }
    let current_image = app.selected_wic_write_image()?;
    if current_image != request.image {
        return Err("The selected Wic image identity changed.".into());
    }
    let device = devices
        .iter()
        .find(|device| &device.identity == device_identity)
        .ok_or_else(|| "The selected Wic device identity is stale.".to_owned())?;
    WicWritePreview::new(&app.wic_capability, request.image.clone(), device, phrase)
        .map_err(str::to_owned)
}

pub(crate) fn queue_wic_session(app: &mut App, operation: WicOperation) -> Option<Effect> {
    if app.active_wic_session().is_some() {
        app.notification = Some("A managed Wic operation is already active.".into());
        return None;
    }
    while app.wic_sessions.len() >= MAX_WIC_SESSIONS {
        let Some(index) = app.wic_sessions.iter().position(|session| {
            app.background_jobs
                .get(session.background_job_id)
                .is_none_or(|job| job.status.is_terminal())
        }) else {
            app.notification = Some("The Wic operation history is full.".into());
            return None;
        };
        app.wic_sessions.remove(index);
    }
    let id = next_wic_session_id(app);
    let background_job_id = wic_background_job_id(id);
    let (title, target, path) = match &operation {
        WicOperation::Create(request) => (
            format!("wic create {}", request.image),
            Some(request.image.clone()),
            Some(request.output_directory.clone()),
        ),
        WicOperation::Write(request) => (
            format!("wic write {}", request.device.path.display()),
            None,
            Some(request.device.path.clone()),
        ),
    };
    app.background_jobs.queue(BackgroundJobSpec {
        id: background_job_id,
        kind: BackgroundJobKind::Wic,
        title,
        context: BackgroundJobContext {
            workspace: Some(Screen::Images),
            target,
            path,
            ..BackgroundJobContext::default()
        },
        cancellation_supported: true,
        queued_at: SystemTime::now(),
    });
    if app.background_jobs.get(background_job_id).is_none() {
        app.notification = Some("The Wic operation could not be queued.".into());
        return None;
    }
    app.wic_sessions.push_back(WicSession {
        id,
        background_job_id,
        operation: operation.clone(),
        exit_code: None,
        error_detail: None,
    });
    Some(Effect::StartWicSession { id, operation })
}
