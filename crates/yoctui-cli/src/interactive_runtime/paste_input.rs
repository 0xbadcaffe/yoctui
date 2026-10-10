use super::*;

fn apply_paste_actions(
    app: &mut App,
    actions: Vec<Action>,
    build_dir: &Path,
    operation: &mut Option<GlobalContentSearchOperation>,
) {
    let search_edit = app.command_palette_open
        && app.command_palette_mode == yoctui_model::CommandPaletteMode::GlobalRegexSearch
        && actions
            .iter()
            .any(|action| matches!(action, Action::AppendCommandPaletteQuery(_)));
    for action in actions {
        let _ = compatibility_workspace_action(app, action);
    }
    if search_edit {
        begin_global_content_search(app, build_dir, operation);
    }
}

impl InteractiveRuntime {
    pub(super) fn insert_text_paste(
        &mut self,
        text: String,
        source: yoctui_model::TextAreaPasteSource,
    ) -> Result<()> {
        match yoctui_app::text_paste_actions(&self.app, text, source) {
            Ok(Some(actions)) => {
                apply_paste_actions(
                    &mut self.app,
                    actions,
                    &self.session_build_dir,
                    &mut self.global_content_search_operation,
                );
            }
            Ok(None) => {}
            Err(error) => self.app.notification = Some(error),
        }
        Ok(())
    }

    pub(super) fn handle_paste(&mut self, text: String) -> Result<()> {
        #[cfg(unix)]
        if self.app.screen == Screen::Daemons
            && self.app.daemon_manager.editing
            && !self.app.menu.is_open()
            && !self.app.command_palette_open
        {
            super::daemon_workspace::append_configuration_paste(
                &mut self.app.daemon_manager,
                &text,
            );
            return Ok(());
        }
        if yoctui_app::text_paste_active(&self.app) {
            return self.insert_text_paste(text, yoctui_model::TextAreaPasteSource::BracketedPaste);
        }
        if self.app.terminal.mode != yoctui_model::TerminalWorkbenchMode::Live
            || self.app.saved_builds.environment.loading
            || self.app.active_dialog().is_some()
            || self.app.menu.is_open()
            || self.app.command_palette_open
            || self.app.onboarding.open
        {
            return Ok(());
        }
        if self.app.screen == Screen::TerminalSessions
            && self.app.focus == yoctui_model::FocusTarget::Workspace
        {
            let _ = compatibility_workspace_action(&mut self.app, Action::TerminalStagePaste(text));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/interactive_runtime/paste_input.rs"]
mod tests;
