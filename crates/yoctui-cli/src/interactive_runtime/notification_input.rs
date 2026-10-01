use super::*;

impl InteractiveRuntime {
    pub(super) fn route_notification_input(&mut self, input: Input) -> bool {
        let Some(action) = notification_input_action(
            self.app.notification.is_some(),
            self.app.build.status == BuildStatus::Failed
                && self.app.logs.diagnostics().next().is_some(),
            self.app.screen == Screen::Settings && self.app.settings_dirty,
            input,
        ) else {
            return false;
        };
        let _ = compatibility_workspace_action(&mut self.app, action);
        true
    }
}
