fn update_recipe_session(state: &mut QaState, action: QaAction) -> QaTransition {
    match action {
        QaAction::ConfirmOperation(preview) => {
            if state.pending_operation.as_ref() != Some(&preview)
                || exact_capability_check(state, &preview).is_none()
                || state.active_session().is_some()
            {
                return QaTransition::notify(
                    "The QA confirmation is stale or no longer available.",
                );
            }
            let session = QaSessionId(next_id(&mut state.session_generation));
            state.sessions.push_back(QaSession {
                id: session,
                operation: preview.clone(),
                status: QaSessionStatus::Starting,
                background_job_id: None,
                started_at: SystemTime::now(),
                finished_at: None,
                message: None,
                result_paths: Vec::new(),
                output: VecDeque::new(),
                dropped_output: 0,
            });
            while state.sessions.len() > MAX_QA_SESSIONS {
                state.sessions.pop_front();
            }
            state.pending_operation = None;
            QaTransition {
                effect: Some(QaEffect::StartBuild {
                    session,
                    request: preview.request,
                }),
                dialog: QaDialogUpdate::Close,
                notification: None,
            }
        }
        QaAction::AttachBackgroundJob {
            session,
            background_job,
        } => {
            if let Some(session) = session_mut(state, session)
                && !session.status.is_terminal()
            {
                session.background_job_id = Some(background_job);
            }
            QaTransition::none()
        }
        QaAction::SessionRunning(id) => {
            if let Some(session) = session_mut(state, id)
                && session.status == QaSessionStatus::Starting
            {
                session.status = QaSessionStatus::Running;
            }
            QaTransition::none()
        }
        QaAction::SessionOutput {
            session: id,
            stream,
            line,
            truncated,
        } => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
                && !line.is_empty()
                && line.len() <= MAX_QA_TEXT_BYTES
                && !line.contains('\0')
            {
                if session.output.len() == MAX_QA_SESSION_OUTPUT {
                    session.output.pop_front();
                    session.dropped_output = session.dropped_output.saturating_add(1);
                }
                session.output.push_back(QaOutputLine {
                    stream,
                    line,
                    truncated,
                });
            }
            QaTransition::none()
        }
        QaAction::CompleteSession {
            session: id,
            result_paths,
            finished_at,
        } => {
            let paths = normalize_paths(result_paths);
            let Some(session) = session_mut(state, id) else {
                return QaTransition::none();
            };
            if session.status.is_terminal() {
                return QaTransition::none();
            }
            session.status = QaSessionStatus::Succeeded;
            session.finished_at = Some(finished_at);
            session.result_paths = paths.clone();
            if paths.is_empty() {
                session.message = Some("no report supplied".into());
                return QaTransition::none();
            }
            match begin_report_request(state, paths) {
                Ok(effect) => QaTransition::effect(effect),
                Err(message) => QaTransition::notify(message),
            }
        }
        QaAction::FailSession {
            session: id,
            message,
            finished_at,
        } => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::Failed;
                session.finished_at = Some(finished_at);
                session.message = Some(message);
            }
            QaTransition::none()
        }
        QaAction::TimeoutSession {
            session: id,
            forced,
            finished_at,
        } => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::TimedOut;
                session.finished_at = Some(finished_at);
                session.message = Some(if forced {
                    "QA cancellation timed out; the process group was forced".into()
                } else {
                    "QA operation timed out".into()
                });
            }
            QaTransition::none()
        }
        QaAction::LoseSession {
            session: id,
            message,
            finished_at,
        } => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::Lost;
                session.finished_at = Some(finished_at);
                session.message = Some(message);
            }
            QaTransition::none()
        }
        QaAction::BeginCancellation => {
            let Some(session) = state.active_session() else {
                return QaTransition::notify("No active QA operation can be cancelled.");
            };
            let Some(background_job) = session.background_job_id else {
                return QaTransition::notify(
                    "The active QA operation is not attached to a managed build yet.",
                );
            };
            QaTransition {
                dialog: QaDialogUpdate::Open(Box::new(QaDialog::Cancellation {
                    session: session.id,
                    background_job,
                })),
                ..QaTransition::none()
            }
        }
        QaAction::ConfirmCancellation(id) => {
            let Some(session) = session_mut(state, id) else {
                return QaTransition::notify("The QA cancellation target is stale.");
            };
            let Some(background_job) = session.background_job_id else {
                return QaTransition::notify("The QA cancellation target has no managed job.");
            };
            if session.status.is_terminal() {
                return QaTransition::notify("The QA cancellation target is already complete.");
            }
            session.status = QaSessionStatus::Cancelling;
            QaTransition {
                effect: Some(QaEffect::CancelBuild {
                    session: id,
                    background_job,
                }),
                dialog: QaDialogUpdate::Close,
                notification: None,
            }
        }
        QaAction::RejectCancellation {
            session: id,
            message,
        } => {
            if let Some(session) = session_mut(state, id)
                && session.status == QaSessionStatus::Cancelling
            {
                session.status = QaSessionStatus::Running;
                session.message = Some(message);
            }
            QaTransition::none()
        }
        QaAction::CancelSession {
            session: id,
            finished_at,
        } => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::Cancelled;
                session.finished_at = Some(finished_at);
            }
            QaTransition::none()
        }
        _ => QaTransition::none(),
    }
}
