use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::CloseKeymapPreferences => {
            app.keymap_preferences_ui.open = false;
            app.keymap_preferences_ui.searching = false;
            app.keymap_preferences_ui.capture = None;
            app.keymap_preferences_ui.validation_error = None;
        }
        Action::SelectKeymapPreference { delta } if app.keymap_preferences_ui.open => {
            let count = keymap_preference_rows(
                &app.keymap_preferences,
                &app.effective_keymap,
                &app.keymap_preferences_ui.query,
            )
            .len();
            app.keymap_preferences_ui.selection = if delta.is_negative() {
                app.keymap_preferences_ui
                    .selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.keymap_preferences_ui
                    .selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
            app.keymap_preferences_ui.validation_error = None;
        }
        Action::SelectKeymapPreference { .. } => {}
        Action::BeginKeymapPreferenceSearch if app.keymap_preferences_ui.open => {
            app.keymap_preferences_ui.searching = true;
            app.keymap_preferences_ui.validation_error = None;
        }
        Action::BeginKeymapPreferenceSearch => {}
        Action::AppendKeymapPreferenceQuery(character)
            if app.keymap_preferences_ui.open
                && app.keymap_preferences_ui.searching
                && app.keymap_preferences_ui.query.len() < 128 =>
        {
            app.keymap_preferences_ui.query.push(character);
            app.keymap_preferences_ui.selection = 0;
        }
        Action::AppendKeymapPreferenceQuery(_) => {}
        Action::BackspaceKeymapPreferenceQuery
            if app.keymap_preferences_ui.open && app.keymap_preferences_ui.searching =>
        {
            app.keymap_preferences_ui.query.pop();
            app.keymap_preferences_ui.selection = 0;
        }
        Action::BackspaceKeymapPreferenceQuery => {}
        Action::ClearKeymapPreferenceQuery if app.keymap_preferences_ui.open => {
            app.keymap_preferences_ui.query.clear();
            app.keymap_preferences_ui.selection = 0;
        }
        Action::ClearKeymapPreferenceQuery => {}
        Action::FinishKeymapPreferenceSearch if app.keymap_preferences_ui.open => {
            app.keymap_preferences_ui.searching = false;
        }
        Action::FinishKeymapPreferenceSearch => {}
        Action::BeginKeymapCapture if app.keymap_preferences_ui.open => {
            if app
                .keymap_preferences_ui
                .selected_row(&app.keymap_preferences, &app.effective_keymap)
                .is_some()
            {
                app.keymap_preferences_ui.capture = Some(KeymapCaptureState::default());
                app.keymap_preferences_ui.validation_error = None;
            }
        }
        Action::BeginKeymapCapture => {}
        Action::AppendKeymapCapture(stroke) => {
            if let Some(capture) = app.keymap_preferences_ui.capture.as_mut() {
                if capture.strokes.len() < MAX_KEY_SEQUENCE_STROKES {
                    capture.strokes.push(stroke);
                    app.keymap_preferences_ui.validation_error = None;
                } else {
                    app.keymap_preferences_ui.validation_error = Some(format!(
                        "A key sequence may contain at most {MAX_KEY_SEQUENCE_STROKES} strokes."
                    ));
                }
            }
        }
        Action::BackspaceKeymapCapture => {
            if let Some(capture) = app.keymap_preferences_ui.capture.as_mut() {
                capture.strokes.pop();
                app.keymap_preferences_ui.validation_error = None;
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
