//! One client-local handoff per file open; keep editor buffers and daemon work intact.
use super::*;
#[cfg(test)]
#[path = "../tests/interactive_runtime/preferred_editor.rs"]
mod tests;

pub(crate) fn preferred_editor_request(
    command: &str,
    editor: &yoctui_model::RecipeEditor,
) -> Result<yoctui_model::TerminalLaunchRequest> {
    yoctui_model::validate_editor_command(command).map_err(anyhow::Error::msg)?;
    let mut argv = yoctui_bitbake::parse_utility_arguments(command).map_err(anyhow::Error::msg)?;
    if argv.is_empty() {
        anyhow::bail!("Editor command has no executable");
    }
    let program = executable_on_initialized_path(&argv.remove(0))
        .context("Editor executable is not installed or not on PATH; use its full path")?;
    let path = editor
        .selected_path()
        .context("Select a source file first")?;
    if !path.is_absolute() || !editor.root.is_absolute() {
        anyhow::bail!("Editor paths must be absolute");
    }
    let filename: String = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .chars()
        .take(20)
        .collect();
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = format!("Editor · {filename} · {nonce}");
    argv.push(
        path.to_str()
            .context("Editor filename must be valid UTF-8")?
            .into(),
    );
    Ok(yoctui_model::TerminalLaunchRequest {
        name: name.clone(),
        kind: yoctui_model::TerminalCreationKind::Utility,
        cwd: editor.root.clone(),
        program,
        arguments: argv,
        completion: Some(yoctui_model::TerminalCompletion::ReturnEditor { path, name }),
    })
}

impl InteractiveRuntime {
    pub(super) async fn poll_preferred_editor(&mut self) -> bool {
        let Some(Dialog::RecipeEditor(editor)) = self.app.active_dialog() else {
            if self.app.suspended_recipe_editor.is_none()
                && !self
                    .app
                    .dialogs
                    .iter()
                    .any(|dialog| matches!(dialog, Dialog::RecipeEditor(_)))
            {
                self.preferred_editor_attempt = None;
            }
            return false;
        };
        let Some(command) = self.app.preferences.terminal_editor.clone() else {
            return false;
        };
        let Some(path) = editor.selected_path() else {
            return false;
        };
        let key = (command.clone(), path.clone());
        if self.preferred_editor_attempt.as_ref() == Some(&key)
            || editor.is_dirty()
            || self.app.suspended_recipe_editor.is_some()
        {
            return false;
        }
        self.preferred_editor_attempt = Some(key);
        let editor = editor.clone();
        let checked_path = path.clone();
        match tokio::task::spawn_blocking(move || read_source_editor_file(&editor, &checked_path))
            .await
        {
            Ok(Ok(_)) => {}
            result => {
                self.app.notification = Some(format!(
                    "Cannot open terminal editor: {result:?}. Built-in editor remains available."
                ));
                return true;
            }
        }
        let editor = match self.app.active_dialog() {
            Some(Dialog::RecipeEditor(editor)) => editor,
            _ => return false,
        };
        let request = match preferred_editor_request(&command, editor) {
            Ok(request) => request,
            Err(error) => {
                self.app.notification = Some(format!(
                    "Cannot open terminal editor: {error}. Built-in editor remains available."
                ));
                return true;
            }
        };
        if !self.app.preferences.terminal_editor_embedded || self.daemon_runtime.is_none() {
            open_in_editor(&self.guard, &mut self.app, path.clone(), Some(&command)).await;
            load_recipe_editor_file(&mut self.app, path).await;
            return true;
        }
        if let Some(effect) = update(
            &mut self.app,
            Action::EditorSettings(yoctui_model::EditorSettingsAction::Launch(Box::new(
                request,
            ))),
        ) {
            let _ = submit_daemon_effect(&mut self.daemon_runtime, &mut self.app, &effect);
        }
        true
    }

    pub(super) async fn finish_preferred_editor(&mut self, path: PathBuf, name: String) {
        let foreground = self.app.screen == Screen::TerminalSessions
            && self
                .app
                .selected_terminal_session()
                .is_some_and(|session| session.name == name);
        let Some((screen, editor)) = self.app.suspended_recipe_editor.as_ref() else {
            return;
        };
        if editor.selected_path().as_ref() != Some(&path) || editor.is_dirty() {
            return;
        }
        let origin = *screen;
        let editor = editor.clone();
        let checked_path = path.clone();
        match tokio::task::spawn_blocking(move || read_source_editor_file(&editor, &checked_path))
            .await
        {
            Ok(Ok(content)) => {
                if let Some((_, editor)) = &mut self.app.suspended_recipe_editor {
                    editor.document.accept_external_text(content);
                    editor.refresh_language_and_validation();
                }
            }
            result => {
                self.app.notification =
                    Some(format!("Editor closed; source reload failed: {result:?}"))
            }
        }
        // Never steal focus from a different terminal or another workspace.
        if !foreground {
            self.app.notification =
                Some("Terminal editor finished. Ctrl+B e restores the retained workbench.".into());
            return;
        }
        if origin == Screen::Devtool {
            update(&mut self.app, Action::RestoreRecipeEditor);
        } else if self.app.active_dialog().is_none() {
            self.app.suspended_recipe_editor = None;
            self.app.screen = origin;
            self.app.focus = yoctui_model::FocusTarget::Workspace;
            self.app.focus_return = None;
            if self
                .app
                .layer_browser
                .as_ref()
                .and_then(yoctui_model::LayerBrowser::selected_entry)
                .is_some_and(|entry| entry.path == path)
            {
                self.layer_browser_io.preview(&mut self.app, path);
            }
        }
    }
}
