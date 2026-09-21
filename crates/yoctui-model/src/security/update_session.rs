fn update_session(state: &mut SecurityState, action: SecurityAction) -> SecurityTransition {
    match action {
        SecurityAction::AttachBackgroundJob {
            id,
            background_job_id,
        } => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
            {
                session.background_job_id = Some(background_job_id);
            }
            SecurityTransition::none()
        }
        SecurityAction::SessionRunning(id) => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && session.status == SecuritySessionStatus::Starting
            {
                session.status = SecuritySessionStatus::Running;
            }
            SecurityTransition::none()
        }
        SecurityAction::SessionOutput {
            id,
            stream,
            line,
            truncated,
        } => {
            if line.len() <= MAX_SECURITY_TEXT_BYTES
                && !line.chars().any(char::is_control)
                && let Some(session) = state
                    .sessions
                    .iter_mut()
                    .find(|session| session.preview.id == id)
                && !session.status.is_terminal()
            {
                session.output.push(SecurityOutputLine {
                    stream,
                    line,
                    truncated,
                });
                if session.output.len() > MAX_SECURITY_SESSION_OUTPUT {
                    session.output.remove(0);
                }
            }
            SecurityTransition::none()
        }
        SecurityAction::CompleteSession {
            id,
            mut result_paths,
            finished_at,
        } => {
            result_paths.retain(|path| absolute_normal_path(path));
            result_paths.sort();
            result_paths.dedup();
            result_paths.truncate(MAX_SECURITY_PATHS);
            let mut refresh_paths = Vec::new();
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && !session.status.is_terminal()
            {
                session.status = SecuritySessionStatus::Succeeded;
                session.finished_at = Some(finished_at);
                session.result_paths = result_paths;
                refresh_paths = if session.result_paths.is_empty() {
                    session.preview.report_roots.clone()
                } else {
                    session.result_paths.clone()
                };
            }
            match begin_report_request(state, refresh_paths) {
                Ok(effect) => SecurityTransition::effect(effect),
                Err(_) => SecurityTransition::notify(
                    "Security operation succeeded, but no exact report path was reported.",
                ),
            }
        }
        SecurityAction::FailSession {
            id,
            message,
            finished_at,
        } => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && !session.status.is_terminal()
            {
                session.status = SecuritySessionStatus::Failed;
                session.finished_at = Some(finished_at);
                session.message = Some(message);
            }
            SecurityTransition::none()
        }
        SecurityAction::LoseSession {
            id,
            message,
            finished_at,
        } => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && !session.status.is_terminal()
            {
                session.status = SecuritySessionStatus::Lost;
                session.finished_at = Some(finished_at);
                session.message = Some(message);
            }
            SecurityTransition::none()
        }
        SecurityAction::TimeoutSession { id, finished_at } => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && !session.status.is_terminal()
            {
                session.status = SecuritySessionStatus::TimedOut;
                session.finished_at = Some(finished_at);
            }
            SecurityTransition::none()
        }
        SecurityAction::BeginCancellation => {
            let Some(session) = state.active_session() else {
                return SecurityTransition::notify("No Security operation is active.");
            };
            SecurityTransition {
                dialog: SecurityDialogUpdate::Open(SecurityDialog::Cancellation(
                    session.preview.id,
                )),
                ..SecurityTransition::none()
            }
        }
        SecurityAction::ConfirmCancellation(id) => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && matches!(
                    session.status,
                    SecuritySessionStatus::Starting | SecuritySessionStatus::Running
                )
            {
                session.status = SecuritySessionStatus::Cancelling;
                return SecurityTransition {
                    effect: Some(SecurityEffect::CancelSession(id)),
                    dialog: SecurityDialogUpdate::Close,
                    notification: None,
                };
            }
            SecurityTransition::notify("The Security cancellation request is stale.")
        }
        SecurityAction::RejectCancellation { id, message } => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && session.status == SecuritySessionStatus::Cancelling
            {
                session.status = SecuritySessionStatus::Running;
                session.message = Some(message);
            }
            SecurityTransition::none()
        }
        SecurityAction::CancelSession { id, finished_at } => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && !session.status.is_terminal()
            {
                session.status = SecuritySessionStatus::Cancelled;
                session.finished_at = Some(finished_at);
            }
            SecurityTransition::none()
        }
        _ => SecurityTransition::none(),
    }
}
