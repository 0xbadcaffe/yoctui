//! Persistent editor choice with a cancelable, bounded command field.
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorSettingsState {
    pub draft: Option<String>,
    pub replace_selection: bool,
    pub error: Option<String>,
    pub embedded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSettingsAction {
    Begin,
    Append(char),
    Backspace,
    Clear,
    Save,
    Cancel,
    TogglePlacement,
    Launch(Box<TerminalLaunchRequest>),
}

pub fn validate_editor_command(command: &str) -> Result<(), String> {
    if command.trim().is_empty() || command.len() > 4096 || command.chars().any(char::is_control) {
        return Err("Use a single-line editor command (up to 4096 bytes), or builtin.".into());
    }
    Ok(())
}

pub(crate) fn reduce(app: &mut App, action: EditorSettingsAction) -> Option<Effect> {
    match action {
        EditorSettingsAction::Begin => {
            if app.screen != Screen::Settings || app.active_dialog().is_some() {
                return None;
            }
            app.editor_settings = EditorSettingsState {
                draft: Some(
                    app.preferences
                        .terminal_editor
                        .clone()
                        .unwrap_or_else(|| "builtin".into()),
                ),
                replace_selection: true,
                error: None,
                embedded: app.preferences.terminal_editor_embedded,
            };
            app.focus = FocusTarget::Workspace;
        }
        EditorSettingsAction::Append(character) => {
            app.editor_settings.error = None;
            if let Some(draft) = &mut app.editor_settings.draft {
                if app.editor_settings.replace_selection {
                    draft.clear();
                }
                app.editor_settings.replace_selection = false;
                if !character.is_control() && draft.len() + character.len_utf8() <= 4096 {
                    draft.push(character);
                }
            }
        }
        EditorSettingsAction::Backspace => {
            app.editor_settings.error = None;
            if let Some(draft) = &mut app.editor_settings.draft {
                if app.editor_settings.replace_selection {
                    draft.clear();
                } else {
                    draft.pop();
                }
                app.editor_settings.replace_selection = false;
            }
        }
        EditorSettingsAction::Clear => {
            app.editor_settings.error = None;
            if let Some(draft) = &mut app.editor_settings.draft {
                draft.clear();
            }
            app.editor_settings.replace_selection = false;
        }
        EditorSettingsAction::Save => {
            let command = app.editor_settings.draft.as_ref()?.trim().to_owned();
            if let Err(error) = validate_editor_command(&command) {
                app.editor_settings.error = Some(error);
                return None;
            }
            app.preferences.terminal_editor = if command.eq_ignore_ascii_case("builtin")
                || command.eq_ignore_ascii_case("built-in")
            {
                None
            } else {
                Some(command)
            };
            app.preferences.terminal_editor_embedded = app.editor_settings.embedded;
            app.editor_settings = EditorSettingsState::default();
            app.settings_dirty = true;
            return Some(Effect::PersistSettings);
        }
        EditorSettingsAction::Cancel => app.editor_settings = EditorSettingsState::default(),
        EditorSettingsAction::TogglePlacement => {
            app.editor_settings.embedded = !app.editor_settings.embedded
        }
        EditorSettingsAction::Launch(request) => {
            if !matches!(app.active_dialog(), Some(Dialog::RecipeEditor(editor)) if !editor.is_dirty())
            {
                return None;
            }
            app.dialogs
                .push_front(Dialog::TerminalLaunch(TerminalLaunchDialog {
                    request: *request,
                    destination: TerminalLaunchDestination::Embedded,
                    output_must_not_exist: None,
                }));
            return update(app, Action::ConfirmTerminalLaunch);
        }
    }
    None
}
