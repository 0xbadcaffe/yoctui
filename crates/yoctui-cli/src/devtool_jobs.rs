//! Devtool jobs.
use super::*;

pub(crate) async fn begin_devtool_job(
    app: &mut App,
    coordinator: &mut DevtoolJobCoordinator,
    runner: &mut Option<DevtoolJobRunner>,
    build_dir: &Path,
    cancellation_timeout: Duration,
    compatibility: Option<&yoctui_model::DaemonCompatibilitySnapshot>,
    operation: DevtoolOperation,
) -> bool {
    let Some(actions) = coordinator.queue(operation.clone(), SystemTime::now()) else {
        let _ = update(
            app,
            Action::Notify("A Devtool background job is already active.".into()),
        );
        return false;
    };
    for action in actions {
        let _ = update(app, action);
    }
    let Some(compatibility) = compatibility else {
        for action in coordinator.start_failed(
            "Devtool is unavailable until the current environment capability snapshot is installed."
                .into(),
            SystemTime::now(),
        ) {
            let _ = update(app, action);
        }
        return false;
    };
    let command = match DevtoolCommandSpec::from_operation(
        &operation,
        compatibility,
        compatibility.snapshot.generation,
        build_dir,
    ) {
        Ok(command) => command,
        Err(error) => {
            for action in coordinator.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(app, action);
            }
            return false;
        }
    };
    let mut started = DevtoolJobRunner::new(build_dir.to_path_buf())
        .with_cancellation_timeout(cancellation_timeout);
    match started.start(command).await {
        Ok(()) => {
            *runner = Some(started);
            true
        }
        Err(error) => {
            for action in coordinator.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(app, action);
            }
            false
        }
    }
}

pub(crate) async fn poll_devtool_job(
    app: &mut App,
    coordinator: &mut DevtoolJobCoordinator,
    runner: &mut Option<DevtoolJobRunner>,
) -> Option<DevtoolOperation> {
    let active = runner.as_mut()?;
    match tokio::time::timeout(Duration::from_millis(1), active.next_event()).await {
        Ok(Ok(event)) => {
            let completed_operation = matches!(event, DevtoolRunnerEvent::Completed { .. })
                .then(|| coordinator.active_operation().cloned())
                .flatten();
            let terminal = matches!(
                event,
                DevtoolRunnerEvent::Completed { .. }
                    | DevtoolRunnerEvent::Failed { .. }
                    | DevtoolRunnerEvent::Cancelled { .. }
                    | DevtoolRunnerEvent::Lost { .. }
            );
            for action in coordinator.actions_for_event(event, SystemTime::now()) {
                let _ = update(app, action);
            }
            if terminal {
                *runner = None;
            }
            completed_operation
        }
        Ok(Err(error)) => {
            for action in coordinator.actions_for_event(
                DevtoolRunnerEvent::Lost {
                    message: error.to_string(),
                },
                SystemTime::now(),
            ) {
                let _ = update(app, action);
            }
            *runner = None;
            None
        }
        Err(_) => None,
    }
}

pub(crate) const SDK_TOOL_OPERATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
