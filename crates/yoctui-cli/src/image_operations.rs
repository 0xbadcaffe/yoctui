//! Image operations.
use super::*;

pub(crate) struct ImageArtifactBackgroundOperation {
    pub(crate) request: ImageArtifactRequest,
    pub(crate) cancellation: ImageArtifactCancellation,
    pub(crate) handle: tokio::task::JoinHandle<BackendEvent>,
}

pub(crate) fn image_artifact_generation(app: &App) -> Option<u64> {
    match &app.image_artifacts {
        ImageArtifactInventoryState::Loading { request }
        | ImageArtifactInventoryState::AvailableEmpty { request, .. }
        | ImageArtifactInventoryState::Available { request, .. }
        | ImageArtifactInventoryState::Partial { request, .. }
        | ImageArtifactInventoryState::Failed { request, .. } => Some(request.generation),
        ImageArtifactInventoryState::NotLoaded => None,
    }
}

#[cfg(test)]
pub(crate) fn execute_qemu_capability_effect(
    app: &mut App,
    inspector: &QemuCapabilityInspector,
    effect: Effect,
) {
    if !matches!(effect, Effect::InspectQemuCapability) {
        return;
    }
    let capability = app.image_artifacts.inventory().map_or_else(
        || QemuCapability::Failed {
            message: "image artifact inventory is unavailable".into(),
        },
        |inventory| inspector.inspect(&inventory.artifacts),
    );
    let _ = update(app, Action::QemuCapabilityLoaded(capability));
}

pub(crate) fn begin_image_artifact_operation(
    app: &mut App,
    adapter: Option<&ImageArtifactAdapter>,
    operation: &mut Option<ImageArtifactBackgroundOperation>,
    effect: Effect,
) {
    let Effect::GetImageArtifacts(request) = effect else {
        return;
    };
    if operation.is_some() {
        app.notification = Some("An image artifact operation is already running.".into());
        return;
    }
    let Some(adapter) = adapter.cloned() else {
        let _ = update(
            app,
            Action::ImageArtifactInventoryFailed {
                request,
                message: "DEPLOY_DIR_IMAGE is unavailable from the active Yocto workspace".into(),
            },
        );
        let _ = update(
            app,
            Action::QemuCapabilityLoaded(QemuCapability::Failed {
                message: "image artifact inventory is unavailable".into(),
            }),
        );
        let _ = update(
            app,
            Action::WicCapabilityLoaded(WicCapability::Failed {
                message: "image artifact inventory is unavailable".into(),
            }),
        );
        return;
    };
    let cancellation = ImageArtifactCancellation::default();
    let worker_cancellation = cancellation.clone();
    let worker_request = request.clone();
    let handle = tokio::spawn(async move {
        match adapter
            .scan_with_cancellation(worker_request.clone(), worker_cancellation)
            .await
        {
            Ok(response) => response.into(),
            Err(error) => BackendEvent::ImageArtifactsFailed {
                request: worker_request,
                message: error.to_string(),
            },
        }
    });
    *operation = Some(ImageArtifactBackgroundOperation {
        request,
        cancellation,
        handle,
    });
}

pub(crate) async fn poll_image_artifact_operation(
    app: &mut App,
    operation: &mut Option<ImageArtifactBackgroundOperation>,
    _qemu_inspector: &QemuCapabilityInspector,
    _wic_inspector: &WicCapabilityInspector,
    _wic_operation: &mut Option<WicCapabilityBackgroundOperation>,
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
    let event = match operation.handle.await {
        Ok(event) => event,
        Err(error) => BackendEvent::ImageArtifactsFailed {
            request: operation.request,
            message: format!("image artifact background task was lost: {error}"),
        },
    };
    let failed_message = match &event {
        BackendEvent::ImageArtifactsFailed { message, .. } => Some(message.clone()),
        _ => None,
    };
    if let Some(action) = model_action_from_backend_event(event) {
        let _ = update(app, action);
    }
    if let Some(message) = failed_message {
        let _ = update(
            app,
            Action::QemuCapabilityLoaded(QemuCapability::Failed {
                message: format!("image artifact inventory failed: {message}"),
            }),
        );
        let _ = update(
            app,
            Action::WicCapabilityLoaded(WicCapability::Failed {
                message: format!("image artifact inventory failed: {message}"),
            }),
        );
    }
}
