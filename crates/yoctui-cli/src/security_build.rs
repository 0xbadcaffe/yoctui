//! Security build.
use super::*;

pub(crate) async fn begin_security_build(
    backend: &mut Box<dyn BitBakeBackend>,
    app: &mut App,
    build_jobs: &mut BuildJobCoordinator,
    id: SecuritySessionId,
    request: BuildRequest,
) -> bool {
    let Some(actions) = build_jobs.queue_build(&request, SystemTime::now()) else {
        let _ = update(
            app,
            Action::Security(SecurityAction::FailSession {
                id,
                message: "another managed BitBake build is already active".into(),
                finished_at: SystemTime::now(),
            }),
        );
        return false;
    };
    for action in actions {
        let _ = update(app, action);
    }
    let Some(background_job_id) = build_jobs.active_job_id() else {
        let _ = update(
            app,
            Action::Security(SecurityAction::LoseSession {
                id,
                message: "Security build coordinator did not retain its job identity".into(),
                finished_at: SystemTime::now(),
            }),
        );
        return false;
    };
    let _ = update(
        app,
        Action::Security(SecurityAction::AttachBackgroundJob {
            id,
            background_job_id,
        }),
    );
    match backend.start_build(request).await {
        Ok(()) => true,
        Err(error) => {
            let _ = update(
                app,
                Action::Security(SecurityAction::FailSession {
                    id,
                    message: error.to_string(),
                    finished_at: SystemTime::now(),
                }),
            );
            for action in build_jobs.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(app, action);
            }
            false
        }
    }
}

pub(crate) fn security_build_action_for_event(
    app: &App,
    id: SecuritySessionId,
    event: &BackendEvent,
) -> Option<Action> {
    let cancelling = app
        .security
        .sessions
        .iter()
        .find(|session| session.preview.id == id)
        .is_some_and(|session| session.status == SecuritySessionStatus::Cancelling);
    match event {
        BackendEvent::BuildStarted => Some(Action::Security(SecurityAction::SessionRunning(id))),
        BackendEvent::BuildCompleted { success: true, .. } => {
            Some(Action::Security(SecurityAction::CompleteSession {
                id,
                result_paths: Vec::new(),
                finished_at: SystemTime::now(),
            }))
        }
        BackendEvent::BuildCompleted { success: false, .. } if cancelling => {
            Some(Action::Security(SecurityAction::CancelSession {
                id,
                finished_at: SystemTime::now(),
            }))
        }
        BackendEvent::BuildCompleted {
            success: false,
            exit_code,
        } => Some(Action::Security(SecurityAction::FailSession {
            id,
            message: exit_code.map_or_else(
                || "Security BitBake task failed without an exit code".into(),
                |code| format!("Security BitBake task failed with exit code {code}"),
            ),
            finished_at: SystemTime::now(),
        })),
        BackendEvent::CommandFailed { code, message } => {
            Some(Action::Security(SecurityAction::FailSession {
                id,
                message: format!("{code}: {message}"),
                finished_at: SystemTime::now(),
            }))
        }
        BackendEvent::Disconnected => Some(Action::Security(SecurityAction::LoseSession {
            id,
            message: "BitBake backend disconnected during Security operation".into(),
            finished_at: SystemTime::now(),
        })),
        _ => None,
    }
}
