fn update_operation_confirmation(
    state: &mut MaintenanceState,
    action: MaintenanceAction,
) -> MaintenanceTransition {
    match action {
        MaintenanceAction::BeginOperation(preview)
            if state.active_session().is_none()
                && !state
                    .sessions
                    .iter()
                    .any(|session| session.id.0 == preview.id)
                && state.capability.request() == Some(preview.capability_request)
                && state
                    .capability
                    .snapshot()
                    .is_some_and(|snapshot| snapshot.supports(preview.operation.tool())) =>
        {
            state.pending = Some(preview.clone());
            let dialog = if preview.operation.cleanup_phrase().is_some() {
                MaintenanceDialog::CleanupPhrase {
                    preview,
                    input: String::new(),
                }
            } else {
                MaintenanceDialog::Confirm(preview)
            };
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(dialog)),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::UpdateCleanupPhrase { preview, input }
            if exact_preview(state, &preview)
                && input.len() <= MAX_MAINTENANCE_TEXT_BYTES
                && !input.chars().any(char::is_control) =>
        {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(MaintenanceDialog::CleanupPhrase {
                    preview,
                    input,
                })),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmCleanupPhrase { preview, input }
            if exact_preview(state, &preview)
                && preview.operation.cleanup_phrase().as_deref() == Some(input.as_str()) =>
        {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(MaintenanceDialog::Confirm(
                    preview,
                ))),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmOperation(preview) if exact_preview(state, &preview) => {
            if preview.operation.network_side_effect() {
                return MaintenanceTransition {
                    dialog: MaintenanceDialogUpdate::Open(Box::new(
                        MaintenanceDialog::ConfirmNetworkPush(preview),
                    )),
                    ..MaintenanceTransition::none()
                };
            }
            return MaintenanceTransition {
                effect: Some(begin_session(state, preview)),
                dialog: MaintenanceDialogUpdate::Close,
                notification: None,
            };
        }
        MaintenanceAction::ConfirmNetworkPush(preview)
            if exact_preview(state, &preview) && preview.operation.network_side_effect() =>
        {
            return MaintenanceTransition {
                effect: Some(begin_session(state, preview)),
                dialog: MaintenanceDialogUpdate::Close,
                notification: None,
            };
        }
        MaintenanceAction::CancelDialog => {
            state.pending = None;
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Close,
                ..MaintenanceTransition::none()
            };
        }
        _ => {}
    }
    MaintenanceTransition::none()
}
