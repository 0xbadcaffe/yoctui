use super::*;

impl InteractiveRuntime {
    pub(super) async fn poll_bitbake_setup(&mut self) {
        let (changed, effect) = self.bitbake_setup_io.poll(&mut self.app).await;
        self.render_scheduler
            .invalidate_if(changed, RenderCause::State);
        if let Some(effect) = effect {
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
