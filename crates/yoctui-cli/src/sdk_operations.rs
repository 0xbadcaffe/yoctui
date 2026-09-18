//! Sdk operations.
use super::*;

pub(crate) struct SdkArtifactBackgroundOperation {
    pub(crate) request: SdkArtifactInventoryRequest,
    pub(crate) cancellation: SdkArtifactCancellation,
    pub(crate) handle: tokio::task::JoinHandle<
        Result<yoctui_bitbake::SdkArtifactResponse, yoctui_bitbake::SdkArtifactAdapterError>,
    >,
}

pub(crate) struct SdkCapabilityBackgroundOperation {
    pub(crate) handle: tokio::task::JoinHandle<SdkToolCapability>,
}

pub(crate) struct SdkCliOperation {
    pub(crate) id: SdkSessionId,
    pub(crate) operation: SdkOperation,
    pub(crate) starting:
        Option<tokio::task::JoinHandle<(SdkToolJobRunner, Result<(), SdkToolAdapterError>)>>,
    pub(crate) runner: Option<SdkToolJobRunner>,
    pub(crate) timeout_wait: Option<
        tokio::task::JoinHandle<(
            SdkToolJobRunner,
            Result<SdkToolRunnerEvent, SdkToolAdapterError>,
        )>,
    >,
    pub(crate) cancellation:
        Option<tokio::task::JoinHandle<(SdkToolJobRunner, Result<bool, SdkToolAdapterError>)>>,
}

pub(crate) fn sdk_tool_adapter_for_workspace(
    app: &App,
    build_directory: &Path,
) -> Result<SdkToolAdapter, String> {
    let build_directory = fs::canonicalize(build_directory)
        .map_err(|error| format!("active build directory is unavailable: {error}"))?;
    if !build_directory.is_dir() {
        return Err("active build directory is not a canonical directory".into());
    }
    let sdk_deploy_root = app
        .workspace
        .variables
        .get("SDK_DEPLOY")
        .map(PathBuf::from)
        .ok_or_else(|| "SDK_DEPLOY is unavailable from the active Yocto workspace".to_owned())?;
    let mut workspace_roots = vec![build_directory.clone()];
    if let Some(source_directory) = app.workspace.source_dir.as_ref() {
        let source_directory = fs::canonicalize(source_directory)
            .map_err(|error| format!("active Yocto source directory is unavailable: {error}"))?;
        if !source_directory.is_dir() {
            return Err("active Yocto source directory is not a canonical directory".into());
        }
        workspace_roots.push(source_directory);
    }
    workspace_roots.sort();
    workspace_roots.dedup();
    Ok(SdkToolAdapter::new(
        build_directory,
        sdk_deploy_root,
        workspace_roots,
    ))
}

pub(crate) fn begin_sdk_capability_operation(
    app: &mut App,
    adapter: Option<&SdkToolAdapter>,
    operation: &mut Option<SdkCapabilityBackgroundOperation>,
    effect: Effect,
) {
    if !matches!(effect, Effect::InspectSdkTools) {
        return;
    }
    if let Some(stale) = operation.take() {
        stale.handle.abort();
    }
    let Some(adapter) = adapter.cloned() else {
        let _ = update(
            app,
            Action::SdkToolCapabilityLoaded(SdkToolCapability::Failed {
                message: "SDK tools cannot be inspected without a canonical build directory, SDK_DEPLOY, and authoritative workspace roots".into(),
            }),
        );
        return;
    };
    let handle = tokio::task::spawn_blocking(move || adapter.capability());
    *operation = Some(SdkCapabilityBackgroundOperation { handle });
}

pub(crate) async fn poll_sdk_capability_operation(
    app: &mut App,
    operation: &mut Option<SdkCapabilityBackgroundOperation>,
) {
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(operation) = operation.take() else {
        return;
    };
    let capability = match operation.handle.await {
        Ok(capability) => capability,
        Err(error) => SdkToolCapability::Failed {
            message: format!("SDK capability inspection task was lost: {error}"),
        },
    };
    let _ = update(app, Action::SdkToolCapabilityLoaded(capability));
}

pub(crate) fn begin_sdk_artifact_operation(
    app: &mut App,
    adapter: Option<&SdkArtifactAdapter>,
    operation: &mut Option<SdkArtifactBackgroundOperation>,
    effect: Effect,
) {
    let Effect::GetSdkArtifacts(request) = effect else {
        return;
    };
    if let Some(stale) = operation.take() {
        stale.cancellation.cancel();
        stale.handle.abort();
    }
    let Some(adapter) = adapter.cloned() else {
        let _ = update(
            app,
            Action::SdkArtifactInventoryFailed {
                request,
                message: "SDK_DEPLOY is unavailable from the active Yocto workspace".into(),
            },
        );
        return;
    };
    let cancellation = SdkArtifactCancellation::default();
    let worker_cancellation = cancellation.clone();
    let worker_request = request.clone();
    let handle = tokio::spawn(async move {
        adapter
            .scan_with_cancellation(worker_request, worker_cancellation)
            .await
    });
    *operation = Some(SdkArtifactBackgroundOperation {
        request,
        cancellation,
        handle,
    });
}

pub(crate) async fn poll_sdk_artifact_operation(
    app: &mut App,
    operation: &mut Option<SdkArtifactBackgroundOperation>,
) {
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(operation) = operation.take() else {
        return;
    };
    let action = match operation.handle.await {
        Ok(Ok(response)) => match response.outcome {
            SdkArtifactScanOutcome::Empty => Action::SdkArtifactInventoryLoaded {
                request: response.request,
                artifacts: Vec::new(),
                limitations: Vec::new(),
            },
            SdkArtifactScanOutcome::Complete(artifacts) => Action::SdkArtifactInventoryLoaded {
                request: response.request,
                artifacts,
                limitations: Vec::new(),
            },
            SdkArtifactScanOutcome::Partial {
                artifacts,
                limitations,
            } => Action::SdkArtifactInventoryLoaded {
                request: response.request,
                artifacts,
                limitations,
            },
        },
        Ok(Err(error)) => Action::SdkArtifactInventoryFailed {
            request: operation.request,
            message: error.to_string(),
        },
        Err(error) => Action::SdkArtifactInventoryFailed {
            request: operation.request,
            message: format!("SDK artifact background task was lost: {error}"),
        },
    };
    let _ = update(app, action);
}

pub(crate) fn sdk_command_for_operation(
    adapter: &SdkToolAdapter,
    operation: &SdkOperation,
) -> Result<SdkToolCommandSpec, SdkToolAdapterError> {
    match operation {
        SdkOperation::Publish(request) => {
            let preview = SdkPublishPreview::new(
                request.executable.clone(),
                request.artifact.clone(),
                request.destination.clone(),
            )
            .map_err(|message| SdkToolAdapterError::InvalidRequest(message.into()))?;
            adapter.publication_command(&preview)
        }
        SdkOperation::Native(request) => {
            let preview = SdkNativePreview::new(request.clone())
                .map_err(|message| SdkToolAdapterError::InvalidRequest(message.into()))?;
            adapter.native_command(&preview)
        }
    }
}
