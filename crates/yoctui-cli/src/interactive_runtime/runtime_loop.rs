use super::*;

impl InteractiveRuntime {
    pub(super) async fn run(mut self) -> Result<()> {
        loop {
            if self.poll_runtime().await? {
                break;
            }
            if self.handle_terminal_event().await? {
                continue;
            }
            self.poll_jobs().await;
            if self.app.should_quit {
                break;
            }
        }
        self.shutdown().await
    }
}
