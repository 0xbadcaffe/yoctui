//! Wic inspection.
use super::*;

pub(crate) struct WicCapabilityBackgroundOperation {
    pub(crate) image_generation: u64,
    pub(crate) handle: tokio::task::JoinHandle<WicCapability>,
}

pub(crate) struct WicDeviceBackgroundOperation {
    pub(crate) request: WicDeviceInventoryRequest,
    pub(crate) handle: tokio::task::JoinHandle<Result<WicDeviceInventoryResponse, WicAdapterError>>,
}

pub(crate) fn begin_wic_device_operation(
    inspector: &WicDeviceInspector,
    operation: &mut Option<WicDeviceBackgroundOperation>,
    effect: Effect,
) {
    let Effect::GetWicDevices(request) = effect else {
        return;
    };
    if let Some(stale) = operation.take() {
        stale.handle.abort();
    }
    let worker_request = request.clone();
    let inspector = inspector.clone();
    let handle = tokio::spawn(async move { inspector.discover(worker_request).await });
    *operation = Some(WicDeviceBackgroundOperation { request, handle });
}

pub(crate) async fn poll_wic_device_operation(
    app: &mut App,
    operation: &mut Option<WicDeviceBackgroundOperation>,
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
        Ok(Ok(response)) => Action::WicDeviceInventoryLoaded {
            request: response.request,
            devices: response.devices,
            limitations: response.limitations,
        },
        Ok(Err(error)) => Action::WicDeviceInventoryFailed {
            request: operation.request,
            message: error.to_string(),
        },
        Err(error) => Action::WicDeviceInventoryFailed {
            request: operation.request,
            message: format!("Wic device discovery task was lost: {error}"),
        },
    };
    let _ = update(app, action);
}

pub(crate) fn configure_wic_capability_inspector(
    app: &App,
    inspector: WicCapabilityInspector,
) -> WicCapabilityInspector {
    let configured_kickstarts = ["WKS_FILE", "WKS_FILES"]
        .into_iter()
        .filter_map(|name| app.workspace.variables.get(name))
        .flat_map(|value| value.split_ascii_whitespace())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .collect();
    let mut canned_roots = app
        .workspace
        .variables
        .get("WKS_SEARCH_PATH")
        .map_or_else(Vec::new, |value| env::split_paths(value).collect());
    if let Some(directory) = app.workspace.variables.get("WKS_FILES_DIR") {
        let directory = PathBuf::from(directory);
        if directory.is_absolute() {
            canned_roots.push(directory);
        }
    }
    canned_roots.retain(|path| path.is_absolute());
    canned_roots.sort();
    canned_roots.dedup();
    inspector.with_sources(configured_kickstarts, canned_roots)
}

pub(crate) fn wic_capability_inspector(app: &App) -> WicCapabilityInspector {
    configure_wic_capability_inspector(app, WicCapabilityInspector::default())
}

pub(crate) fn begin_wic_capability_operation(
    app: &mut App,
    inspector: &WicCapabilityInspector,
    operation: &mut Option<WicCapabilityBackgroundOperation>,
    effect: Effect,
) {
    if !matches!(effect, Effect::InspectWicCapability) || operation.is_some() {
        return;
    }
    let Some(inventory) = app.image_artifacts.inventory() else {
        let _ = update(
            app,
            Action::WicCapabilityLoaded(WicCapability::Failed {
                message: "image artifact inventory is unavailable".into(),
            }),
        );
        return;
    };
    let Some(image_generation) = image_artifact_generation(app) else {
        return;
    };
    let mut image_targets = inventory
        .artifacts
        .iter()
        .map(|artifact| artifact.identity.image.clone())
        .collect::<Vec<_>>();
    image_targets.sort();
    image_targets.dedup();
    let inspector = inspector.clone();
    let handle = tokio::spawn(async move { inspector.inspect(image_targets).await });
    *operation = Some(WicCapabilityBackgroundOperation {
        image_generation,
        handle,
    });
}

pub(crate) async fn poll_wic_capability_operation(
    app: &mut App,
    inspector: &WicCapabilityInspector,
    operation: &mut Option<WicCapabilityBackgroundOperation>,
) {
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(active) = operation.take() else {
        return;
    };
    let active_generation = active.image_generation;
    let capability = match active.handle.await {
        Ok(capability) => capability,
        Err(error) => WicCapability::Failed {
            message: format!("Wic capability background task was lost: {error}"),
        },
    };
    if image_artifact_generation(app) == Some(active_generation) {
        let _ = update(app, Action::WicCapabilityLoaded(capability));
    } else if app.image_artifacts.inventory().is_some() {
        begin_wic_capability_operation(app, inspector, operation, Effect::InspectWicCapability);
    }
}
