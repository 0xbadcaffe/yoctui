//! Qa build.
use super::*;

pub(crate) async fn begin_qa_build(
    backend: &mut Box<dyn BitBakeBackend>,
    app: &mut App,
    build_jobs: &mut BuildJobCoordinator,
    session: QaSessionId,
    request: BuildRequest,
) -> bool {
    let Some(actions) = build_jobs.queue_build(&request, SystemTime::now()) else {
        let _ = update(
            app,
            Action::Qa(QaAction::FailSession {
                session,
                message: "another managed BitBake build is already active".into(),
                finished_at: SystemTime::now(),
            }),
        );
        return false;
    };
    for action in actions {
        let _ = update(app, action);
    }
    let Some(background_job) = build_jobs.active_job_id() else {
        let _ = update(
            app,
            Action::Qa(QaAction::LoseSession {
                session,
                message: "QA build coordinator did not retain its job identity".into(),
                finished_at: SystemTime::now(),
            }),
        );
        return false;
    };
    let _ = update(
        app,
        Action::Qa(QaAction::AttachBackgroundJob {
            session,
            background_job,
        }),
    );
    match backend.start_build(request).await {
        Ok(()) => true,
        Err(error) => {
            let _ = update(
                app,
                Action::Qa(QaAction::FailSession {
                    session,
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

pub(crate) fn qa_build_action_for_event(
    app: &App,
    session: QaSessionId,
    event: &BackendEvent,
) -> Option<Action> {
    let qa_session = app
        .qa
        .sessions
        .iter()
        .find(|candidate| candidate.id == session)?;
    match event {
        BackendEvent::BuildStarted => Some(Action::Qa(QaAction::SessionRunning(session))),
        BackendEvent::BuildCompleted { success: true, .. } => {
            Some(Action::Qa(QaAction::CompleteSession {
                session,
                result_paths: qa_session.operation.report_roots.clone(),
                finished_at: SystemTime::now(),
            }))
        }
        BackendEvent::BuildCompleted { success: false, .. }
            if qa_session.status == QaSessionStatus::Cancelling =>
        {
            Some(Action::Qa(QaAction::CancelSession {
                session,
                finished_at: SystemTime::now(),
            }))
        }
        BackendEvent::BuildCompleted {
            success: false,
            exit_code,
        } => Some(Action::Qa(QaAction::FailSession {
            session,
            message: exit_code.map_or_else(
                || "QA BitBake task failed without an exit code".into(),
                |code| format!("QA BitBake task failed with exit code {code}"),
            ),
            finished_at: SystemTime::now(),
        })),
        BackendEvent::CommandFailed { code, message } => Some(Action::Qa(QaAction::FailSession {
            session,
            message: format!("{code}: {message}"),
            finished_at: SystemTime::now(),
        })),
        BackendEvent::Disconnected => Some(Action::Qa(QaAction::LoseSession {
            session,
            message: "BitBake backend disconnected during QA".into(),
            finished_at: SystemTime::now(),
        })),
        _ => None,
    }
}
