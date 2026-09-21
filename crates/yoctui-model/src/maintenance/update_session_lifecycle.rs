fn update_session_lifecycle(
    state: &mut MaintenanceState,
    action: MaintenanceAction,
) -> MaintenanceTransition {
    match action {
        MaintenanceAction::SessionRunning { id, started_at } => {
            if let Some(session) = session_mut(state, id)
                && session.status == MaintenanceSessionStatus::Queued
            {
                session.status = MaintenanceSessionStatus::Running;
                session.started_at = Some(started_at);
            }
        }
        MaintenanceAction::SessionOutput { id, stream, text } => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.append_output(stream, text);
            }
        }
        MaintenanceAction::CompleteSession {
            id,
            exit_code,
            mut evidence,
            finished_at,
        } => {
            if exit_code != 0 {
                let _ = terminal_session(
                    state,
                    id,
                    MaintenanceSessionStatus::Failed,
                    Some(exit_code),
                    Some(format!(
                        "Maintenance operation exited with status {exit_code}"
                    )),
                    finished_at,
                );
            } else if terminal_session(
                state,
                id,
                MaintenanceSessionStatus::Succeeded,
                Some(0),
                None,
                finished_at,
            ) {
                evidence.retain(|item| item.identity.is_valid() && bounded_text(&item.label));
                evidence.sort_by(|left, right| left.identity.path.cmp(&right.identity.path));
                evidence.dedup_by(|left, right| left.identity.path == right.identity.path);
                evidence.truncate(MAX_MAINTENANCE_EVIDENCE);
                state.evidence = evidence;
                state.evidence_selection = 0;
            }
        }
        MaintenanceAction::FailSession {
            id,
            message,
            exit_code,
            finished_at,
        } => {
            let _ = terminal_session(
                state,
                id,
                MaintenanceSessionStatus::Failed,
                exit_code,
                Some(message),
                finished_at,
            );
        }
        MaintenanceAction::TimeoutSession { id, finished_at } => {
            let _ = terminal_session(
                state,
                id,
                MaintenanceSessionStatus::TimedOut,
                None,
                Some("Maintenance operation timed out".into()),
                finished_at,
            );
        }
        MaintenanceAction::LoseSession {
            id,
            message,
            finished_at,
        } => {
            let _ = terminal_session(
                state,
                id,
                MaintenanceSessionStatus::Lost,
                None,
                Some(message),
                finished_at,
            );
        }
        MaintenanceAction::BeginCancellation => {
            if let Some(session) = state.active_session() {
                return MaintenanceTransition {
                    dialog: MaintenanceDialogUpdate::Open(Box::new(
                        MaintenanceDialog::ConfirmCancellation(session.id),
                    )),
                    ..MaintenanceTransition::none()
                };
            }
        }
        MaintenanceAction::ConfirmCancellation(id) => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = MaintenanceSessionStatus::Cancelling;
                return MaintenanceTransition {
                    effect: Some(MaintenanceEffect::CancelOperation(id)),
                    dialog: MaintenanceDialogUpdate::Close,
                    notification: None,
                };
            }
        }
        MaintenanceAction::RejectCancellation { id, message } => {
            if let Some(session) = session_mut(state, id)
                && session.status == MaintenanceSessionStatus::Cancelling
            {
                session.status = MaintenanceSessionStatus::Running;
                session.message = Some(message);
            }
        }
        MaintenanceAction::CancelSession { id, finished_at } => {
            let _ = terminal_session(
                state,
                id,
                MaintenanceSessionStatus::Cancelled,
                None,
                None,
                finished_at,
            );
        }
        MaintenanceAction::SelectEvidence(delta) => {
            state.evidence_selection = if delta.is_negative() {
                state
                    .evidence_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                state
                    .evidence_selection
                    .saturating_add(delta as usize)
                    .min(state.evidence.len().saturating_sub(1))
            };
        }
        MaintenanceAction::OpenSelectedEvidence => {
            if let Some(evidence) = state.selected_evidence() {
                return MaintenanceTransition::effect(MaintenanceEffect::OpenEvidence(
                    evidence.identity.clone(),
                ));
            }
        }
        MaintenanceAction::OpenSignatures => {
            return MaintenanceTransition::effect(MaintenanceEffect::Navigate(Screen::Signatures));
        }
        MaintenanceAction::OpenSecurity => {
            return MaintenanceTransition::effect(MaintenanceEffect::Navigate(Screen::Security));
        }
        MaintenanceAction::OpenQa => {
            return MaintenanceTransition::effect(MaintenanceEffect::Navigate(Screen::Qa));
        }
        MaintenanceAction::OpenRecipes => {
            return MaintenanceTransition::effect(MaintenanceEffect::Navigate(Screen::Recipes));
        }
        _ => {}
    }
    MaintenanceTransition::none()
}
