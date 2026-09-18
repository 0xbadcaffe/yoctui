//! Wic jobs.
use super::*;

pub(crate) struct WicCliOperation {
    pub(crate) id: WicSessionId,
    pub(crate) starting:
        Option<tokio::task::JoinHandle<(WicJobRunner, Result<(), WicAdapterError>)>>,
    pub(crate) runner: Option<WicJobRunner>,
    pub(crate) cancellation:
        Option<tokio::task::JoinHandle<(WicJobRunner, Result<bool, WicAdapterError>)>>,
}

pub(crate) fn wic_preview_for_request(
    capability: &WicCapability,
    request: &WicCreateRequest,
) -> Result<WicCreatePreview, String> {
    let output_directory = request
        .output_directory
        .to_str()
        .ok_or_else(|| "Wic output directory is not valid UTF-8".to_owned())?;
    let draft = WicCreateDraft {
        machine: request.machine.clone(),
        image: request.image.clone(),
        kickstart: request.kickstart.clone(),
        output_directory: output_directory.into(),
        generate_bmap: request.generate_bmap,
        compression: request.compression,
    };
    let preview = draft.preview(capability).map_err(str::to_owned)?;
    if &preview.request != request {
        return Err("Wic request changed while rebuilding its preview".into());
    }
    Ok(preview)
}

pub(crate) async fn begin_wic_job(
    app: &mut App,
    operation: &mut Option<WicCliOperation>,
    device_inspector: &WicDeviceInspector,
    build_dir: &Path,
    cancellation_timeout: Duration,
    id: WicSessionId,
    requested_operation: WicOperation,
) {
    if operation.is_some() {
        let _ = update(
            app,
            Action::FailWicSession {
                id,
                message: "another managed Wic process is already owned by the CLI".into(),
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        return;
    }
    let mut runner =
        WicJobRunner::new(build_dir.to_path_buf()).with_cancellation_timeout(cancellation_timeout);
    let start = match requested_operation {
        WicOperation::Create(request) => {
            let output_directory = request.output_directory.clone();
            match wic_preview_for_request(&app.wic_capability, &request)
                .map_err(WicAdapterError::InvalidRequest)
                .and_then(|preview| {
                    WicCreateCommandSpec::from_preview(&preview, &app.wic_capability)
                }) {
                Ok(command) => runner.start(command, output_directory).await,
                Err(error) => Err(error),
            }
        }
        WicOperation::Write(request) => {
            let inspector = device_inspector.clone();
            let starting = tokio::spawn(async move {
                let result = runner.start_write(&inspector, request).await;
                (runner, result)
            });
            *operation = Some(WicCliOperation {
                id,
                starting: Some(starting),
                runner: None,
                cancellation: None,
            });
            return;
        }
    };
    match start {
        Ok(()) => {
            *operation = Some(WicCliOperation {
                id,
                starting: None,
                runner: Some(runner),
                cancellation: None,
            });
        }
        Err(error) => {
            let _ = update(
                app,
                Action::FailWicSession {
                    id,
                    message: error.to_string(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
        }
    }
}

pub(crate) fn begin_wic_cancellation(
    app: &mut App,
    operation: &mut Option<WicCliOperation>,
    id: WicSessionId,
) {
    let Some(active) = operation.as_mut().filter(|active| active.id == id) else {
        let _ = update(
            app,
            Action::RejectWicSessionCancellation {
                id,
                message: "the CLI does not own this Wic process".into(),
            },
        );
        return;
    };
    if active.cancellation.is_some() {
        let _ = update(
            app,
            Action::RejectWicSessionCancellation {
                id,
                message: "Wic cancellation is already in progress".into(),
            },
        );
        return;
    }
    if let Some(starting) = active.starting.take() {
        starting.abort();
        let _ = update(
            app,
            Action::CancelWicSession {
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
            Action::RejectWicSessionCancellation {
                id,
                message: "the Wic runner is unavailable".into(),
            },
        );
        return;
    };
    active.cancellation = Some(tokio::spawn(async move {
        let result = runner.cancel().await;
        (runner, result)
    }));
}

pub(crate) async fn poll_wic_job(app: &mut App, operation: &mut Option<WicCliOperation>) {
    let Some(active) = operation.as_mut() else {
        return;
    };
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
                    Action::FailWicSession {
                        id,
                        message: error.to_string(),
                        exit_code: None,
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return;
            }
            Err(error) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::LoseWicSession {
                        id,
                        message: format!("Wic startup task was lost: {error}"),
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return;
            }
        }
    }
    if active.starting.is_some() {
        return;
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
                    Action::RejectWicSessionCancellation {
                        id: active.id,
                        message: error.to_string(),
                    },
                );
            }
            Err(error) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::LoseWicSession {
                        id,
                        message: format!("Wic cancellation task was lost: {error}"),
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return;
            }
        }
    }
    if active.cancellation.is_some() {
        return;
    }
    let Some(runner) = active.runner.as_mut() else {
        return;
    };
    match tokio::time::timeout(Duration::from_millis(1), runner.next_event()).await {
        Ok(Ok(event)) => {
            let terminal = matches!(
                event,
                WicRunnerEvent::Completed { .. }
                    | WicRunnerEvent::Failed { .. }
                    | WicRunnerEvent::Cancelled { .. }
                    | WicRunnerEvent::Lost { .. }
            );
            for action in wic_actions_for_runner_event(active.id, event, SystemTime::now()) {
                let _ = update(app, action);
            }
            if terminal {
                *operation = None;
            }
        }
        Ok(Err(error)) => {
            let id = active.id;
            for action in wic_actions_for_runner_event(
                id,
                WicRunnerEvent::Lost {
                    message: error.to_string(),
                },
                SystemTime::now(),
            ) {
                let _ = update(app, action);
            }
            *operation = None;
        }
        Err(_) => {}
    }
}
