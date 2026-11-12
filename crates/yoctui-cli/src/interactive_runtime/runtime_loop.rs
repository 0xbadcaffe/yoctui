use super::*;

impl InteractiveRuntime {
    pub(super) async fn run(mut self) -> Result<()> {
        loop {
            // Check before rendering or reading input, even when the terminal
            // disappears without delivering SIGHUP (for example in a harness).
            #[cfg(unix)]
            if terminal_disconnected()? {
                break;
            }
            if self.poll_runtime().await? {
                break;
            }
            if self.handle_terminal_event().await? {
                continue;
            }
            self.poll_jobs().await;
            if self.app.should_quit {
                match self.flush_workbench().await {
                    Ok(()) => break,
                    Err(error) => {
                        self.app.should_quit = false;
                        self.app.notification = Some(format!(
                            "Could not save Workbench recovery: {error}. Your buffers remain open; fix storage and retry quitting."
                        ));
                        self.render_scheduler.invalidate(RenderCause::State);
                    }
                }
            }
        }
        self.shutdown().await
    }
}
