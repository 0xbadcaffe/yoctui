use super::*;

impl InteractiveRuntime {
    pub(super) fn finish_setup_operation(
        &mut self,
        profile: yoctui_model::BuildEnvironmentProfile,
    ) {
        if let Some(effect) = update(
            &mut self.app,
            Action::BitBakeSetup(yoctui_model::BitBakeSetupAction::Manage(
                yoctui_model::SetupManageAction::Completed(profile),
            )),
        ) {
            self.bitbake_setup_io.submit(effect);
        }
    }
    pub(super) async fn poll_bitbake_setup(&mut self) {
        let (changed, effect) = self.bitbake_setup_io.poll(&mut self.app).await;
        self.render_scheduler
            .invalidate_if(changed, RenderCause::State);
        if let Some(effect) = effect {
            if matches!(effect, Effect::BitBakeSetup(_)) {
                self.bitbake_setup_io.submit(effect);
                return;
            }
            #[cfg(unix)]
            if submit_daemon_effect(&mut self.daemon_runtime, &mut self.app, &effect) != Some(true)
            {
                self.app.terminal.browser_open = true;
                self.app.terminal.awaiting_session = false;
                self.app
                    .notification
                    .get_or_insert("Connect to a daemon before starting setup.".into());
            }
        }
    }
}
