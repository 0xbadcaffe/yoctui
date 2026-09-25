use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::ConfirmKeymapCapture => {
            let row = app
                .keymap_preferences_ui
                .selected_row(&app.keymap_preferences, &app.effective_keymap)?;
            let Some(sequence) = app
                .keymap_preferences_ui
                .capture
                .as_ref()
                .and_then(KeymapCaptureState::sequence)
            else {
                app.keymap_preferences_ui.validation_error =
                    Some("Capture at least one key before saving.".into());
                return None;
            };
            let candidate = app.keymap_preferences.with_action_sequences(
                row.action_id,
                row.scope,
                vec![sequence],
            );
            match EffectiveKeymap::from_preferences(&candidate) {
                Ok(effective) => {
                    app.keymap_preferences = candidate;
                    app.effective_keymap = effective;
                    app.keymap_preferences_ui.capture = None;
                    app.keymap_preferences_ui.validation_error = None;
                    app.settings_dirty = true;
                    return Some(Effect::PersistSettings);
                }
                Err(error) => {
                    app.keymap_preferences_ui.validation_error = Some(error.to_string());
                }
            }
        }
        Action::CancelKeymapCapture => {
            app.keymap_preferences_ui.capture = None;
            app.keymap_preferences_ui.validation_error = None;
        }
        Action::RemoveKeymapBinding => {
            let row = app
                .keymap_preferences_ui
                .selected_row(&app.keymap_preferences, &app.effective_keymap)?;
            let candidate =
                app.keymap_preferences
                    .with_action_sequences(row.action_id, row.scope, Vec::new());
            match EffectiveKeymap::from_preferences(&candidate) {
                Ok(effective) => {
                    app.keymap_preferences = candidate;
                    app.effective_keymap = effective;
                    app.keymap_preferences_ui.validation_error = None;
                    app.settings_dirty = true;
                    return Some(Effect::PersistSettings);
                }
                Err(error) => {
                    app.keymap_preferences_ui.validation_error = Some(error.to_string());
                }
            }
        }
        Action::ResetKeymapBinding => {
            let row = app
                .keymap_preferences_ui
                .selected_row(&app.keymap_preferences, &app.effective_keymap)?;
            let candidate = app
                .keymap_preferences
                .reset_action(row.action_id, row.scope);
            if let Ok(effective) = EffectiveKeymap::from_preferences(&candidate) {
                app.keymap_preferences = candidate;
                app.effective_keymap = effective;
                app.keymap_preferences_ui.validation_error = None;
                app.settings_dirty = true;
                return Some(Effect::PersistSettings);
            }
        }
        Action::ResetAllKeymapBindings => {
            app.reset_keymap();
            app.keymap_preferences_ui.open = true;
            app.settings_dirty = true;
            return Some(Effect::PersistSettings);
        }
        Action::ExportEffectiveKeymap if app.keymap_preferences_ui.open => {
            return Some(Effect::CopyToClipboard(app.effective_keymap_report()));
        }
        Action::ExportEffectiveKeymap => {}
        Action::SettingsPersisted => {
            app.preferences = app.effective_preferences();
            app.settings_dirty = false;
            app.notification = None;
        }
        Action::SettingsPersistenceFailed(message) => {
            app.settings_dirty = true;
            app.notification = Some(format!(
                "Settings changed in memory but could not be saved: {message}"
            ));
        }
        Action::SetCompatibilityFilter(filter) => {
            app.compatibility_ui
                .set_filter(filter, app.workspace_compatibility.authority());
        }
        Action::SelectCompatibilityCapability { delta } => {
            app.compatibility_ui
                .select(delta, app.workspace_compatibility.authority());
        }
        Action::BeginCompatibilitySearch => app.compatibility_ui.begin_search(),
        Action::AppendCompatibilityQuery(character) if app.compatibility_ui.searching => {
            let _ = app
                .compatibility_ui
                .append_query(character, app.workspace_compatibility.authority());
        }
        Action::AppendCompatibilityQuery(_) => {}
        Action::BackspaceCompatibilityQuery if app.compatibility_ui.searching => {
            app.compatibility_ui
                .backspace_query(app.workspace_compatibility.authority());
        }
        Action::BackspaceCompatibilityQuery => {}
        Action::ClearCompatibilityQuery => app
            .compatibility_ui
            .clear_query(app.workspace_compatibility.authority()),
        Action::FinishCompatibilitySearch => app.compatibility_ui.finish_search(),
        Action::RawMode(RawModeAction::ConfirmPreview) => {
            app.raw_request_generation = app.raw_request_generation.wrapping_add(1).max(1);
            let nanos = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let request_id = RawRequestId::new(format!(
                "raw-request:client-{nanos}-{}",
                app.raw_request_generation
            ))
            .expect("generated Raw request identity is bounded and valid");
            let authority = app.workspace_compatibility.authority().cloned();
            match confirmed_raw_execution_request(
                &app.raw_mode,
                builtin_raw_catalog(),
                authority.as_ref(),
                request_id,
            ) {
                Ok(request) => {
                    reduce_raw_mode(
                        &mut app.raw_mode,
                        builtin_raw_catalog(),
                        authority.as_ref(),
                        RawModeAction::OpenExecution(request.command.clone()),
                    );
                    app.raw_mode.output.request = Some(request.id.clone());
                    return Some(Effect::StartRaw(request));
                }
                Err(error) => app.raw_mode.notification = Some(error.to_string()),
            }
        }
        Action::RawMode(RawModeAction::CancelExecution(request_id)) => {
            let cancellable = app
                .raw_mode
                .execution_states
                .get(&request_id)
                .is_some_and(|state| !state.phase.is_terminal() && !state.cancellation_requested);
            if cancellable {
                return Some(Effect::CancelRaw(request_id));
            }
            app.raw_mode.notification =
                Some("Raw execution is unknown, terminal, or already cancelling.".into());
        }
        Action::RawMode(RawModeAction::SetExecutionAttachment {
            request,
            attachment,
        }) => {
            let available = app
                .raw_mode
                .execution_states
                .get(&request)
                .is_some_and(|state| !state.phase.is_terminal() && state.attachment != attachment);
            if available {
                return Some(Effect::SetRawAttachment {
                    request,
                    attached: attachment == RawAttachmentState::Attached,
                });
            }
            app.raw_mode.notification = Some(
                "Raw execution is terminal, unknown, or already has that attachment state.".into(),
            );
        }
        Action::RawMode(RawModeAction::CloseExecution) => {
            let detach = app
                .raw_mode
                .selected_execution()
                .filter(|state| {
                    !state.phase.is_terminal() && state.attachment == RawAttachmentState::Attached
                })
                .map(|state| state.request.id.clone());
            let authority = app.workspace_compatibility.authority().cloned();
            reduce_raw_mode(
                &mut app.raw_mode,
                builtin_raw_catalog(),
                authority.as_ref(),
                RawModeAction::CloseExecution,
            );
            if let Some(request) = detach {
                return Some(Effect::SetRawAttachment {
                    request,
                    attached: false,
                });
            }
        }
        Action::RawMode(action) => {
            let authority = app.workspace_compatibility.authority().cloned();
            let persist_favorites = matches!(
                action,
                RawModeAction::ToggleFavorite
                    | RawModeAction::RenameFavorite { .. }
                    | RawModeAction::MoveFavorite { .. }
                    | RawModeAction::RemoveFavorite
                    | RawModeAction::ConfirmFavorite
            );
            reduce_raw_mode(
                &mut app.raw_mode,
                builtin_raw_catalog(),
                authority.as_ref(),
                action,
            );
            if persist_favorites {
                return Some(Effect::PersistSettings);
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
