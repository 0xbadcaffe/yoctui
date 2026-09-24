//! Rootfs operations.
use super::*;

pub(crate) struct RootfsCompositionBackgroundOperation {
    pub(crate) request: RootfsCompositionRequest,
    #[cfg(unix)]
    pub(crate) authority: Option<yoctui_protocol::rootfs::RootfsSourcesRequestData>,
    pub(crate) _cancellation: RootfsCompositionCancellation,
    pub(crate) handle: tokio::task::JoinHandle<BackendEvent>,
}

impl Drop for RootfsCompositionBackgroundOperation {
    fn drop(&mut self) {
        self._cancellation.cancel();
    }
}

pub(crate) fn begin_rootfs_composition_operation_with_sources(
    app: &mut App,
    build_directory: &Path,
    operation: &mut Option<RootfsCompositionBackgroundOperation>,
    request: RootfsCompositionRequest,
    sources: RootfsCompositionSources,
) {
    if operation.is_some() {
        app.notification = Some("A rootfs composition operation is already running.".into());
        return;
    }
    let adapter =
        RootfsCompositionAdapter::new(build_directory.to_path_buf(), sources, request.generation);
    let cancellation = RootfsCompositionCancellation::default();
    let worker_cancellation = cancellation.clone();
    let worker_request = request.clone();
    let handle = tokio::spawn(async move {
        match adapter
            .scan_with_cancellation(worker_request.clone(), worker_cancellation)
            .await
        {
            Ok(response) if response.composition.is_unavailable() => {
                BackendEvent::RootfsCompositionUnavailable {
                    request: response.request,
                    reason: "the selected image manifest and IMAGE_ROOTFS are unavailable".into(),
                }
            }
            Ok(response) => response.into(),
            Err(error) => BackendEvent::RootfsCompositionFailed {
                request: worker_request,
                message: error.to_string(),
            },
        }
    });
    *operation = Some(RootfsCompositionBackgroundOperation {
        request,
        #[cfg(unix)]
        authority: None,
        _cancellation: cancellation,
        handle,
    });
}

pub(crate) async fn begin_rootfs_composition_operation(
    backend: &mut dyn BitBakeBackend,
    app: &mut App,
    build_directory: &Path,
    operation: &mut Option<RootfsCompositionBackgroundOperation>,
    effect: Effect,
    daemon_attached: bool,
) {
    let Effect::GetRootfsComposition(request) = effect else {
        return;
    };
    if operation.is_some() {
        if daemon_attached {
            operation.as_ref().unwrap()._cancellation.cancel();
            let _ = update(
                app,
                Action::RootfsCompositionFailed {
                    request,
                    message: "Previous rootfs lookup is stopping; refresh when it finishes.".into(),
                },
            );
        } else {
            app.notification = Some("A rootfs composition operation is already running.".into());
        }
        return;
    }
    #[cfg(unix)]
    if daemon_attached {
        let query = match daemon_rootfs::query_for_app(app, &request) {
            Ok(query) => query,
            Err(error) => {
                let _ = update(
                    app,
                    Action::RootfsCompositionFailed {
                        request,
                        message: error.to_string(),
                    },
                );
                return;
            }
        };
        let fallback = client_rootfs_composition_sources(app, &request, None, None, None);
        let build = build_directory.to_path_buf();
        let cancellation = RootfsCompositionCancellation::default();
        let worker_cancellation = cancellation.clone();
        let worker_request = request.clone();
        let worker_query = query.clone();
        let handle = tokio::spawn(async move {
            let query_build = build.clone();
            let query_cancel = worker_cancellation.clone();
            let result = tokio::task::spawn_blocking(move || {
                daemon_rootfs::request_sources(&worker_query, &query_build, &query_cancel)
            })
            .await
            .map_err(|error| format!("rootfs source worker was lost: {error}"))
            .and_then(|result| {
                result.map_err(|error| format!("rootfs source lookup failed: {error:#}"))
            });
            let (sources, metadata_limitation) = match result {
                Ok(sources) => (
                    RootfsCompositionSources {
                        image: worker_request.image.clone(),
                        manifest: fallback
                            .manifest
                            .or_else(|| sources.image_manifest.map(PathBuf::from)),
                        pkgdata_directory: sources
                            .pkgdata_dir
                            .map(PathBuf::from)
                            .or(fallback.pkgdata_directory),
                        image_rootfs: sources.image_rootfs.map(PathBuf::from),
                    },
                    None,
                ),
                Err(message) => (
                    fallback,
                    Some(format!(
                        "BitBake rootfs metadata was unavailable; deployed artifacts were used: {message}"
                    )),
                ),
            };
            match RootfsCompositionAdapter::new(build, sources, worker_request.generation)
                .scan_with_cancellation(worker_request.clone(), worker_cancellation)
                .await
            {
                Ok(mut response) if response.composition.is_unavailable() => {
                    if let Some(limitation) = metadata_limitation {
                        response.limitations.push(limitation);
                    }
                    BackendEvent::RootfsCompositionUnavailable {
                        request: response.request,
                        reason: response.limitations.first().cloned().unwrap_or_else(|| {
                            "the selected image manifest and IMAGE_ROOTFS are unavailable".into()
                        }),
                    }
                }
                Ok(mut response) => {
                    if let Some(limitation) = metadata_limitation {
                        response.limitations.push(limitation);
                    }
                    response.into()
                }
                Err(error) => BackendEvent::RootfsCompositionFailed {
                    request: worker_request,
                    message: error.to_string(),
                },
            }
        });
        *operation = Some(RootfsCompositionBackgroundOperation {
            request,
            authority: Some(query),
            _cancellation: cancellation,
            handle,
        });
        return;
    }
    #[cfg(not(unix))]
    let _ = daemon_attached;
    let recipe = request.image.image.clone();
    let manifest = backend
        .get_variable("IMAGE_MANIFEST".into(), Some(recipe.clone()))
        .await;
    let pkgdata = backend
        .get_variable("PKGDATA_DIR".into(), Some(recipe.clone()))
        .await;
    let rootfs = backend
        .get_variable("IMAGE_ROOTFS".into(), Some(recipe))
        .await;
    let source = |name: &str, result: Result<VariableValue, yoctui_bitbake::BackendError>| {
        result
            .map(|value| value.value.map(PathBuf::from))
            .map_err(|error| format!("could not query {name} for the selected image: {error}"))
    };
    let sources = match (
        source("IMAGE_MANIFEST", manifest),
        source("PKGDATA_DIR", pkgdata),
        source("IMAGE_ROOTFS", rootfs),
    ) {
        (Ok(manifest), Ok(pkgdata_directory), Ok(image_rootfs)) => {
            client_rootfs_composition_sources(
                app,
                &request,
                manifest,
                pkgdata_directory,
                image_rootfs,
            )
        }
        values => {
            let message = [values.0.err(), values.1.err(), values.2.err()]
                .into_iter()
                .flatten()
                .next()
                .expect("one rootfs source query failed");
            let _ = update(app, Action::RootfsCompositionFailed { request, message });
            return;
        }
    };
    begin_rootfs_composition_operation_with_sources(
        app,
        build_directory,
        operation,
        request,
        sources,
    );
}

pub(crate) fn client_rootfs_composition_sources(
    app: &App,
    request: &RootfsCompositionRequest,
    manifest: Option<PathBuf>,
    pkgdata_directory: Option<PathBuf>,
    image_rootfs: Option<PathBuf>,
) -> RootfsCompositionSources {
    let selected = app
        .image_artifacts
        .artifacts()
        .unwrap_or_default()
        .iter()
        .find(|artifact| artifact.identity == request.image);
    let manifest = manifest.or_else(|| {
        selected
            .and_then(|artifact| artifact.manifests.available())
            .and_then(|paths| paths.first().cloned())
    });
    let pkgdata_directory = pkgdata_directory.or_else(|| {
        app.workspace
            .variables
            .get("PKGDATA_DIR")
            .map(PathBuf::from)
    });
    RootfsCompositionSources {
        image: request.image.clone(),
        manifest,
        pkgdata_directory,
        image_rootfs,
    }
}

pub(crate) async fn poll_rootfs_composition_operation(
    app: &mut App,
    operation: &mut Option<RootfsCompositionBackgroundOperation>,
) {
    #[cfg(unix)]
    if let Some(pending) = operation.as_ref()
        && let Some(authority) = &pending.authority
        && (app.rootfs_composition.request() != Some(&pending.request)
            || daemon_rootfs::query_for_app(app, &pending.request)
                .as_ref()
                .ok()
                != Some(authority))
    {
        pending._cancellation.cancel();
    }
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(mut operation) = operation.take() else {
        return;
    };
    let event = match (&mut operation.handle).await {
        Ok(event) => event,
        Err(error) => BackendEvent::RootfsCompositionFailed {
            request: operation.request.clone(),
            message: format!("rootfs composition background task was lost: {error}"),
        },
    };
    #[cfg(unix)]
    let event = if let Some(authority) = &operation.authority
        && (operation._cancellation.is_cancelled()
            || daemon_rootfs::query_for_app(app, &operation.request)
                .as_ref()
                .ok()
                != Some(authority))
    {
        BackendEvent::RootfsCompositionFailed {
            request: operation.request.clone(),
            message: "rootfs source authority changed; refresh and retry".into(),
        }
    } else {
        event
    };
    if let Some(action) = model_action_from_backend_event(event) {
        let _ = update(app, action);
    }
}
