//! Package operations.
use super::*;

#[derive(Debug, Clone)]
pub(crate) enum PackageOperationRequest {
    Inventory(PackageInventoryRequest),
    Detail(PackageDetailRequest),
}

pub(crate) struct PackageBackgroundOperation {
    pub(crate) request: PackageOperationRequest,
    pub(crate) cancellation: PackageDataCancellation,
    pub(crate) handle: tokio::task::JoinHandle<BackendEvent>,
}

pub(crate) fn begin_package_operation(
    app: &mut App,
    adapter: &PackageDataAdapter,
    operation: &mut Option<PackageBackgroundOperation>,
    effect: Effect,
) {
    if operation.is_some() {
        let _ = update(
            app,
            Action::Notify("A package-data operation is already running.".into()),
        );
        return;
    }
    let adapter = app.workspace.variables.get("PKGDATA_DIR").map_or_else(
        || adapter.clone(),
        |path| adapter.clone().with_pkgdata_dir(PathBuf::from(path)),
    );
    let adapter = match app.workspace_compatibility.authority().cloned() {
        Some(compatibility) => {
            let generation = compatibility.snapshot.generation;
            match adapter.with_compatibility(compatibility, generation) {
                Ok(adapter) => adapter,
                Err(error) => {
                    let event = match effect {
                        Effect::GetPackageInventory(request) => {
                            BackendEvent::PackageInventoryFailed {
                                request,
                                message: error.to_string(),
                            }
                        }
                        Effect::GetPackageDetail(request) => BackendEvent::PackageDetailFailed {
                            request,
                            message: error.to_string(),
                        },
                        _ => return,
                    };
                    if let Some(action) = model_action_from_backend_event(event) {
                        let _ = update(app, action);
                    }
                    return;
                }
            }
        }
        None => adapter,
    };
    let cancellation = PackageDataCancellation::default();
    let worker_cancellation = cancellation.clone();
    let (request, handle) = match effect {
        Effect::GetPackageInventory(request) => {
            let worker_request = request;
            let handle = tokio::spawn(async move {
                match adapter
                    .inventory_with_cancellation(worker_request, worker_cancellation)
                    .await
                {
                    Ok(response) => response.into(),
                    Err(error) => BackendEvent::PackageInventoryFailed {
                        request: worker_request,
                        message: error.to_string(),
                    },
                }
            });
            (PackageOperationRequest::Inventory(request), handle)
        }
        Effect::GetPackageDetail(request) => {
            let worker_request = request.clone();
            let handle = tokio::spawn(async move {
                match adapter
                    .detail_with_cancellation(worker_request.clone(), worker_cancellation)
                    .await
                {
                    Ok(response) => response.into(),
                    Err(error) => BackendEvent::PackageDetailFailed {
                        request: worker_request,
                        message: error.to_string(),
                    },
                }
            });
            (PackageOperationRequest::Detail(request), handle)
        }
        _ => return,
    };
    *operation = Some(PackageBackgroundOperation {
        request,
        cancellation,
        handle,
    });
}

pub(crate) async fn poll_package_operation(
    app: &mut App,
    operation: &mut Option<PackageBackgroundOperation>,
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
        Err(error) => match operation.request {
            PackageOperationRequest::Inventory(request) => BackendEvent::PackageInventoryFailed {
                request,
                message: format!("package-data background task was lost: {error}"),
            },
            PackageOperationRequest::Detail(request) => BackendEvent::PackageDetailFailed {
                request,
                message: format!("package-data background task was lost: {error}"),
            },
        },
    };
    if let Some(action) = model_action_from_backend_event(event) {
        let _ = update(app, action);
    }
}
