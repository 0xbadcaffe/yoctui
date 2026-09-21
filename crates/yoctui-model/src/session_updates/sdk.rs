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

