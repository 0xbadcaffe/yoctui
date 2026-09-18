//! Test build.
use super::*;

pub(crate) async fn begin_test_build(
    backend: &mut Box<dyn BitBakeBackend>,
    app: &mut App,
    build_jobs: &mut BuildJobCoordinator,
    id: TestSessionId,
    request: BuildRequest,
) -> bool {
    let Some(actions) = build_jobs.queue_build(&request, SystemTime::now()) else {
        let _ = update(
            app,
            Action::FailTestSession {
                id,
                message: "another managed BitBake build is already active".into(),
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        return false;
    };
    for action in actions {
        let _ = update(app, action);
    }
    let Some(background_job_id) = build_jobs.active_job_id() else {
        let _ = update(
            app,
            Action::FailTestSession {
                id,
                message: "Testing build coordinator did not retain its job identity".into(),
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        return false;
    };
    let _ = update(
        app,
        Action::AttachTestBuildSession {
            id,
            background_job_id,
        },
    );
    match backend.start_build(request).await {
        Ok(()) => true,
        Err(error) => {
            let _ = update(
                app,
                Action::FailTestSession {
                    id,
                    message: error.to_string(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
            for action in build_jobs.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(app, action);
            }
            false
        }
    }
}

pub(crate) fn test_build_action_for_event(
    app: &App,
    id: TestSessionId,
    event: &BackendEvent,
) -> Option<Action> {
    let cancelling = app
        .test_session(id)
        .and_then(|session| session.background_job_id)
        .and_then(|job_id| app.background_jobs.get(job_id))
        .is_some_and(|job| job.status == yoctui_model::BackgroundJobStatus::Cancelling);
    match event {
        BackendEvent::BuildStarted => Some(Action::TestSessionRunning { id }),
        BackendEvent::BuildCompleted {
            success: true,
            exit_code,
        } => Some(Action::CompleteTestSession {
            id,
            exit_code: exit_code.unwrap_or(0),
            result_paths: Vec::new(),
            finished_at: SystemTime::now(),
        }),
        BackendEvent::BuildCompleted {
            success: false,
            exit_code,
        } if cancelling => Some(Action::CancelTestSession {
            id,
            exit_code: *exit_code,
            finished_at: SystemTime::now(),
        }),
        BackendEvent::BuildCompleted {
            success: false,
            exit_code,
        } => Some(Action::FailTestSession {
            id,
            message: exit_code.map_or_else(
                || "Testing BitBake task failed without an exit code".into(),
                |code| format!("Testing BitBake task failed with exit code {code}"),
            ),
            exit_code: *exit_code,
            finished_at: SystemTime::now(),
        }),
        BackendEvent::CommandFailed { code, message } => Some(Action::FailTestSession {
            id,
            message: format!("{code}: {message}"),
            exit_code: None,
            finished_at: SystemTime::now(),
        }),
        BackendEvent::Disconnected => Some(Action::LoseTestSession {
            id,
            message: "BitBake backend disconnected during Testing".into(),
            finished_at: SystemTime::now(),
        }),
        _ => None,
    }
}
