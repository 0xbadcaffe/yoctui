fn update_layer_session(state: &mut QaState, action: QaAction) -> QaTransition {
    match action {
        QaAction::LayerSessionRunning(id) => {
            if let Some(session) = layer_session_mut(state, id)
                && session.status == QaSessionStatus::Starting
            {
                session.status = QaSessionStatus::Running;
            }
            QaTransition::none()
        }
        QaAction::LayerSessionOutput {
            session: id,
            stream,
            line,
            truncated,
        } => {
            if let Some(session) = layer_session_mut(state, id)
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
        QaAction::CompleteLayerSession {
            session: id,
            exit_code,
            result_paths,
            finished_at,
        } => {
            let paths = normalize_paths(result_paths);
            let Some(session) = layer_session_mut(state, id) else {
                return QaTransition::none();
            };
            if session.status.is_terminal() {
                return QaTransition::none();
            }
            session.finished_at = Some(finished_at);
            session.exit_code = Some(exit_code);
            session.result_paths = paths.clone();
            if exit_code != 0 {
                session.status = QaSessionStatus::Failed;
                session.message = Some(format!("yocto-check-layer exited with status {exit_code}"));
                return QaTransition::none();
            }
            session.status = QaSessionStatus::Succeeded;
            if paths.is_empty() {
                session.message = Some("no report supplied".into());
                return QaTransition::none();
            }
            match begin_report_request(state, paths) {
                Ok(effect) => QaTransition::effect(effect),
                Err(message) => QaTransition::notify(message),
            }
        }
        QaAction::FailLayerSession {
            session: id,
            exit_code,
            message,
            finished_at,
        } => {
            if let Some(session) = layer_session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::Failed;
                session.finished_at = Some(finished_at);
                session.exit_code = exit_code;
                session.message = Some(message);
            }
            QaTransition::none()
        }
        QaAction::TimeoutLayerSession {
            session: id,
            forced,
            exit_code,
            finished_at,
        } => {
            if let Some(session) = layer_session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::TimedOut;
                session.finished_at = Some(finished_at);
                session.exit_code = exit_code;
                session.message = Some(if forced {
                    "Layer QA timed out and required forced termination".into()
                } else {
                    "Layer QA timed out".into()
                });
            }
            QaTransition::none()
        }
        QaAction::LoseLayerSession {
            session: id,
            message,
            finished_at,
        } => {
            if let Some(session) = layer_session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::Lost;
                session.finished_at = Some(finished_at);
                session.message = Some(message);
            }
            QaTransition::none()
        }
        QaAction::BeginLayerCancellation => {
            let Some(session) = state.active_layer_session() else {
                return QaTransition::notify("No active layer QA operation can be cancelled.");
            };
            QaTransition {
                dialog: QaDialogUpdate::Open(Box::new(QaDialog::LayerCancellation(session.id))),
                ..QaTransition::none()
            }
        }
        QaAction::ConfirmLayerCancellation(id) => {
            let Some(session) = layer_session_mut(state, id) else {
                return QaTransition::notify("The layer QA cancellation target is stale.");
            };
            if session.status.is_terminal() {
                return QaTransition::notify(
                    "The layer QA cancellation target is already complete.",
                );
            }
            session.status = QaSessionStatus::Cancelling;
            QaTransition {
                effect: Some(QaEffect::CancelLayerCheck(id)),
                dialog: QaDialogUpdate::Close,
                notification: None,
            }
        }
        QaAction::RejectLayerCancellation {
            session: id,
            message,
        } => {
            if let Some(session) = layer_session_mut(state, id)
                && session.status == QaSessionStatus::Cancelling
            {
                session.status = QaSessionStatus::Running;
                session.message = Some(message);
            }
            QaTransition::none()
        }
        QaAction::CancelLayerSession {
            session: id,
            forced,
            exit_code,
            finished_at,
        } => {
            if let Some(session) = layer_session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::Cancelled;
                session.finished_at = Some(finished_at);
                session.exit_code = exit_code;
                session.message = forced.then(|| "Layer QA required forced termination".into());
            }
            QaTransition::none()
        }
        QaAction::OpenSelectedLayerRoot => state.selected_layer().map_or_else(
            || QaTransition::notify("No exact configured layer is selected."),
            |layer| QaTransition::effect(QaEffect::OpenLayerRoot(layer.identity.clone())),
        ),        _ => QaTransition::none(),
    }
}
