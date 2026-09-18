//! Qemu jobs.
use super::*;

pub(crate) struct QemuCliOperation {
    pub(crate) id: QemuSessionId,
    pub(crate) runner: Option<QemuJobRunner>,
    pub(crate) cancellation:
        Option<tokio::task::JoinHandle<(QemuJobRunner, Result<bool, QemuAdapterError>)>>,
}

pub(crate) fn qemu_preview_for_request(
    capability: &QemuCapability,
    request: &QemuLaunchRequest,
) -> Result<QemuLaunchPreview, String> {
    let kernel = request
        .kernel
        .as_ref()
        .map(|path| {
            path.to_str()
                .map(str::to_owned)
                .ok_or_else(|| "runqemu kernel path is not valid UTF-8".to_owned())
        })
        .transpose()?
        .unwrap_or_default();
    let rootfs = request
        .rootfs
        .as_ref()
        .map(|path| {
            path.to_str()
                .map(str::to_owned)
                .ok_or_else(|| "runqemu rootfs path is not valid UTF-8".to_owned())
        })
        .transpose()?
        .unwrap_or_default();
    let draft = QemuLaunchDraft {
        machine: request.machine.clone(),
        image: request.image.clone(),
        artifact_kind: request.artifact_kind,
        kernel,
        rootfs,
        networking: request.networking,
        display: request.display,
        serial: request.serial,
        memory_mib: request.memory_mib.to_string(),
        extra_arguments: request.extra_arguments.join(" "),
    };
    let preview = draft.preview(capability).map_err(str::to_owned)?;
    if &preview.request != request {
        return Err("runqemu request changed while rebuilding its preview".into());
    }
    Ok(preview)
}

pub(crate) async fn begin_qemu_job(
    app: &mut App,
    operation: &mut Option<QemuCliOperation>,
    build_dir: &Path,
    cancellation_timeout: Duration,
    id: QemuSessionId,
    request: QemuLaunchRequest,
) {
    if operation.is_some() {
        let _ = update(
            app,
            Action::FailQemuSession {
                id,
                message: "another managed runqemu process is already owned by the CLI".into(),
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        return;
    }
    let start = qemu_preview_for_request(&app.qemu_capability, &request)
        .map_err(QemuAdapterError::InvalidRequest)
        .and_then(|preview| QemuCommandSpec::from_preview(&preview));
    let command = match start {
        Ok(command) => command,
        Err(error) => {
            let _ = update(
                app,
                Action::FailQemuSession {
                    id,
                    message: error.to_string(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
            return;
        }
    };
    let mut runner =
        QemuJobRunner::new(build_dir.to_path_buf()).with_cancellation_timeout(cancellation_timeout);
    match runner.start(command).await {
        Ok(()) => {
            *operation = Some(QemuCliOperation {
                id,
                runner: Some(runner),
                cancellation: None,
            });
        }
        Err(error) => {
            let _ = update(
                app,
                Action::FailQemuSession {
                    id,
                    message: error.to_string(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
        }
    }
}

pub(crate) fn begin_qemu_cancellation(
    app: &mut App,
    operation: &mut Option<QemuCliOperation>,
    id: QemuSessionId,
) {
    let Some(active) = operation.as_mut().filter(|active| active.id == id) else {
        let _ = update(
            app,
            Action::RejectQemuSessionCancellation {
                id,
                message: "the CLI does not own this runqemu process".into(),
            },
        );
        return;
    };
    if active.cancellation.is_some() {
        let _ = update(
            app,
            Action::RejectQemuSessionCancellation {
                id,
                message: "runqemu cancellation is already in progress".into(),
            },
        );
        return;
    }
    let Some(mut runner) = active.runner.take() else {
        let _ = update(
            app,
            Action::RejectQemuSessionCancellation {
                id,
                message: "the runqemu runner is unavailable".into(),
            },
        );
        return;
    };
    active.cancellation = Some(tokio::spawn(async move {
        let result = runner.cancel().await;
        (runner, result)
    }));
}

pub(crate) async fn poll_qemu_job(app: &mut App, operation: &mut Option<QemuCliOperation>) {
    let Some(active) = operation.as_mut() else {
        return;
    };
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
                    Action::RejectQemuSessionCancellation {
                        id: active.id,
                        message: error.to_string(),
                    },
                );
            }
            Err(error) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::LoseQemuSession {
                        id,
                        message: format!("runqemu cancellation task was lost: {error}"),
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
                QemuRunnerEvent::Completed { .. }
                    | QemuRunnerEvent::Failed { .. }
                    | QemuRunnerEvent::Cancelled { .. }
                    | QemuRunnerEvent::Lost { .. }
            );
            for action in qemu_actions_for_runner_event(active.id, event, SystemTime::now()) {
                let _ = update(app, action);
            }
            if terminal {
                *operation = None;
            }
        }
        Ok(Err(error)) => {
            let id = active.id;
            for action in qemu_actions_for_runner_event(
                id,
                QemuRunnerEvent::Lost {
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
