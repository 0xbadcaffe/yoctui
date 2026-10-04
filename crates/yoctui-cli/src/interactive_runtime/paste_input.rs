use super::*;

impl InteractiveRuntime {
    pub(super) fn insert_text_paste(
        &mut self,
        text: String,
        source: yoctui_model::TextAreaPasteSource,
    ) -> Result<()> {
        match yoctui_app::text_paste_actions(&self.app, text, source) {
            Ok(Some(actions)) => {
                for action in actions {
                    let _ = compatibility_workspace_action(&mut self.app, action);
                }
            }
            Ok(None) => {}
            Err(error) => self.app.notification = Some(error),
        }
        Ok(())
    }

    pub(super) fn handle_paste(&mut self, text: String) -> Result<()> {
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
