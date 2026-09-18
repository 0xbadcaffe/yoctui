//! Sdk jobs.
use super::*;

pub(crate) fn begin_sdk_job(
    app: &mut App,
    owned: &mut Option<SdkCliOperation>,
    adapter: Option<&SdkToolAdapter>,
    cancellation_timeout: Duration,
    operation_timeout: Duration,
    id: SdkSessionId,
    operation: SdkOperation,
) {
    if owned.is_some() {
        let _ = update(
            app,
            Action::FailSdkSession {
                id,
                message: "another managed SDK tool process is already owned by the CLI".into(),
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        return;
    }
    let Some(adapter) = adapter.cloned() else {
        let _ = update(
            app,
            Action::FailSdkSession {
                id,
                message: "SDK tool execution is unavailable for the active workspace".into(),
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        return;
    };
    let worker_operation = operation.clone();
    let starting = tokio::spawn(async move {
        let mut runner = SdkToolJobRunner::new()
            .with_cancellation_timeout(cancellation_timeout)
            .with_operation_timeout(operation_timeout);
        let result = match sdk_command_for_operation(&adapter, &worker_operation) {
            Ok(command) => runner.start(command).await,
            Err(error) => Err(error),
        };
        (runner, result)
    });
    *owned = Some(SdkCliOperation {
        id,
        operation,
        starting: Some(starting),
        runner: None,
        timeout_wait: None,
        cancellation: None,
    });
}

pub(crate) fn begin_sdk_cancellation(
    app: &mut App,
    operation: &mut Option<SdkCliOperation>,
    id: SdkSessionId,
) {
    let Some(active) = operation.as_mut().filter(|active| active.id == id) else {
        let _ = update(
            app,
            Action::RejectSdkSessionCancellation {
                id,
                message: "the CLI does not own this SDK tool process".into(),
            },
        );
        return;
    };
    if active.cancellation.is_some() {
        let _ = update(
            app,
            Action::RejectSdkSessionCancellation {
                id,
                message: "SDK tool cancellation is already in progress".into(),
            },
        );
        return;
    }
    if active.timeout_wait.is_some() {
        let _ = update(
            app,
            Action::RejectSdkSessionCancellation {
                id,
                message: "SDK tool timeout finalization is already in progress".into(),
            },
        );
        return;
    }
    if let Some(starting) = active.starting.take() {
        starting.abort();
        let _ = update(
            app,
            Action::CancelSdkSession {
                id,
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        *operation = None;
        return;
    }
    let Some(mut runner) = active.runner.take() else {
        let _ = update(
            app,
            Action::RejectSdkSessionCancellation {
                id,
                message: "the SDK tool runner is unavailable".into(),
            },
        );
        return;
    };
    active.cancellation = Some(tokio::spawn(async move {
        let result = runner.cancel().await;
        (runner, result)
    }));
}

pub(crate) async fn poll_sdk_job(
    app: &mut App,
    operation: &mut Option<SdkCliOperation>,
) -> Option<SdkOperation> {
    let active = operation.as_mut()?;
    if active
        .starting
        .as_ref()
        .is_some_and(tokio::task::JoinHandle::is_finished)
    {
        let handle = active.starting.take().expect("checked above");
        match handle.await {
            Ok((runner, Ok(()))) => active.runner = Some(runner),
            Ok((_, Err(error))) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::FailSdkSession {
                        id,
                        message: error.to_string(),
                        exit_code: None,
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return None;
            }
            Err(error) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::LoseSdkSession {
                        id,
                        message: format!("SDK tool startup task was lost: {error}"),
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return None;
            }
        }
    }
    if active.starting.is_some() {
        return None;
    }
    if active
        .cancellation
        .as_ref()
        .is_some_and(tokio::task::JoinHandle::is_finished)
    {
        let handle = active.cancellation.take().expect("checked above");
        match handle.await {
            Ok((runner, Ok(_))) => active.runner = Some(runner),
            Ok((runner, Err(error))) => {
                active.runner = Some(runner);
                let _ = update(
                    app,
                    Action::RejectSdkSessionCancellation {
                        id: active.id,
                        message: error.to_string(),
                    },
                );
            }
            Err(error) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::LoseSdkSession {
                        id,
                        message: format!("SDK tool cancellation task was lost: {error}"),
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return None;
            }
        }
    }
    if active.cancellation.is_some() {
        return None;
    }
    if active
        .timeout_wait
        .as_ref()
        .is_some_and(tokio::task::JoinHandle::is_finished)
    {
        let handle = active.timeout_wait.take().expect("checked above");
        let event = match handle.await {
            Ok((_runner, Ok(event))) => event,
            Ok((_runner, Err(error))) => SdkToolRunnerEvent::Lost {
                message: error.to_string(),
            },
            Err(error) => SdkToolRunnerEvent::Lost {
                message: format!("SDK timeout finalization task was lost: {error}"),
            },
        };
        let id = active.id;
        for action in sdk_actions_for_runner_event(id, event, SystemTime::now()) {
            let _ = update(app, action);
        }
        *operation = None;
        return None;
    }
    if active.timeout_wait.is_some() {
        return None;
    }
    let runner = active.runner.as_mut()?;
    if runner.operation_timeout_due_within(Duration::from_millis(2)) {
        let mut runner = active.runner.take().expect("runner checked above");
        active.timeout_wait = Some(tokio::spawn(async move {
            let result = runner.next_event().await;
            (runner, result)
        }));
        return None;
    }
    match tokio::time::timeout(Duration::from_millis(1), runner.next_event()).await {
        Ok(Ok(event)) => {
            let completed = matches!(event, SdkToolRunnerEvent::Completed { .. });
            let terminal = completed
                || matches!(
                    event,
                    SdkToolRunnerEvent::Failed { .. }
                        | SdkToolRunnerEvent::Cancelled { .. }
                        | SdkToolRunnerEvent::TimedOut { .. }
                        | SdkToolRunnerEvent::Lost { .. }
                );
            let completed_operation = completed.then(|| active.operation.clone());
            for action in sdk_actions_for_runner_event(active.id, event, SystemTime::now()) {
                let _ = update(app, action);
            }
            if terminal {
                *operation = None;
            }
            completed_operation
        }
        Ok(Err(error)) => {
            let id = active.id;
            for action in sdk_actions_for_runner_event(
                id,
                SdkToolRunnerEvent::Lost {
                    message: error.to_string(),
                },
                SystemTime::now(),
            ) {
                let _ = update(app, action);
            }
            *operation = None;
            None
        }
        Err(_) => None,
    }
}
