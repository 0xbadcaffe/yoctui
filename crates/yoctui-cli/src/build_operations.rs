//! Build operations.
use super::*;

pub(crate) async fn begin_build(
    backend: &mut Box<dyn BitBakeBackend>,
    app: &mut App,
    build_jobs: &mut BuildJobCoordinator,
    request: BuildRequest,
) -> bool {
    let Some(actions) = build_jobs.queue_build(&request, SystemTime::now()) else {
        let _ = update(
            app,
            Action::Notify("A build background job is already active.".into()),
        );
        return false;
    };
    for action in actions {
        let _ = update(app, action);
    }
    match backend.start_build(request).await {
        Ok(()) => true,
        Err(error) => {
            for action in build_jobs.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(app, action);
            }
            let _ = update(
                app,
                Action::Failure(AppError::new(
                    "BitBake",
                    error.to_string(),
                    "check backend diagnostics and retry",
                )),
            );
            false
        }
    }
}

#[cfg(unix)]
pub(crate) async fn begin_runtime_build(
    runtime: &mut Option<client_runtime::InteractiveDaemonRuntime>,
    backend: &mut Box<dyn BitBakeBackend>,
    app: &mut App,
    build_jobs: &mut BuildJobCoordinator,
    request: BuildRequest,
) -> bool {
    if let Some(runtime) = runtime.as_mut() {
        match runtime.route_effect(app, &Effect::Start(request.clone())) {
            Ok(client_runtime::RuntimeEffectRoute::Daemon(request_id)) => {
                app.notification = Some(format!(
                    "Build request {} submitted to the Yoctui daemon.",
                    request_id.0
                ));
                return true;
            }
            Ok(client_runtime::RuntimeEffectRoute::ClientLocal) => {}
            Err(error) => {
                app.notification = Some(format!("Daemon build request was not sent: {error}"));
                return false;
            }
        }
    }
    begin_build(backend, app, build_jobs, request).await
}

#[cfg(unix)]
pub(crate) fn submit_daemon_effect(
    runtime: &mut Option<client_runtime::InteractiveDaemonRuntime>,
    app: &mut App,
    effect: &Effect,
) -> Option<bool> {
    let runtime = runtime.as_mut()?;
    match runtime.route_effect(app, effect) {
        Ok(client_runtime::RuntimeEffectRoute::Daemon(request_id)) => {
            app.notification = Some(format!(
                "Request {} submitted to the Yoctui daemon.",
                request_id.0
            ));
            Some(true)
        }
        Ok(client_runtime::RuntimeEffectRoute::ClientLocal) => None,
        Err(error) => {
            app.notification = Some(format!("Daemon request was not sent: {error}"));
            Some(false)
        }
    }
}

#[cfg(not(unix))]
pub(crate) async fn begin_runtime_build(
    _runtime: &mut Option<()>,
    backend: &mut Box<dyn BitBakeBackend>,
    app: &mut App,
    build_jobs: &mut BuildJobCoordinator,
    request: BuildRequest,
) -> bool {
    begin_build(backend, app, build_jobs, request).await
}
