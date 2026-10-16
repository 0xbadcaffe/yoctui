use super::*;
use yoctui_model::EditorSettingsAction;

impl InteractiveRuntime {
    pub(super) async fn editor_preferences_key(&mut self, input: Input) -> Result<bool> {
        if self.app.editor_settings.draft.is_none() {
            return Ok(false);
        }
        if input == Input::CtrlV {
            match crate::clipboard::read_system_clipboard().await {
                Ok(text) => self.editor_preferences_paste(&text),
                Err(error) => self.app.editor_settings.error = Some(error.to_string()),
            }
            return Ok(true);
        }
        let action = match input {
            Input::Tab => EditorSettingsAction::TogglePlacement,
            Input::Char(character) => EditorSettingsAction::Append(character),
            Input::Backspace => EditorSettingsAction::Backspace,
            Input::CtrlU => EditorSettingsAction::Clear,
            Input::Esc => EditorSettingsAction::Cancel,
            Input::Enter => {
                if let Some(command) = &self.app.editor_settings.draft
                    && let Err(error) = yoctui_bitbake::parse_utility_arguments(command)
                {
                    self.app.editor_settings.error = Some(error);
                    return Ok(true);
                }
                EditorSettingsAction::Save
            }
            _ => return Ok(true),
        };
        if matches!(
            update(&mut self.app, Action::EditorSettings(action)),
            Some(Effect::PersistSettings)
        ) {
            let result = persist_settings(
                self.session_path.as_deref(),
                &mut self.session,
                &self.app,
                !self.color_forced_off,
            );
            update(
                &mut self.app,
                match result {
                    Ok(()) => Action::SettingsPersisted,
                    Err(error) => Action::SettingsPersistenceFailed(error.to_string()),
                },
            );
        }
        self.render_scheduler.invalidate(RenderCause::State);
        Ok(true)
    }

    pub(super) fn editor_preferences_paste(&mut self, text: &str) {
        let existing = if self.app.editor_settings.replace_selection {
            0
        } else {
            self.app
                .editor_settings
                .draft
                .as_ref()
                .map_or(0, String::len)
        };
        if text.chars().any(char::is_control) || existing + text.len() > 4096 {
            self.app.editor_settings.error =
                Some("Paste a single-line editor command up to 4096 bytes.".into());
            return;
        }
        for character in text.chars() {
            update(
                &mut self.app,
                Action::EditorSettings(EditorSettingsAction::Append(character)),
            );
        }
        self.render_scheduler.invalidate(RenderCause::State);
    }
}
