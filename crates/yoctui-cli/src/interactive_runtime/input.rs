use super::*;

impl InteractiveRuntime {
    pub(super) async fn handle_terminal_event(&mut self) -> Result<bool> {
        if !event::poll(self.frame_interval)? {
            return Ok(false);
        }
        let terminal_event = event::read()?;
        self.render_scheduler.invalidate(RenderCause::Input);
        if terminal_event_requires_full_redraw(&terminal_event) {
            self.terminal.clear()?;
            return Ok(true);
        }
        match terminal_event {
            Event::Paste(text) => {
                self.handle_paste(text)?;
                Ok(true)
            }
            Event::Mouse(mouse) => {
                self.handle_mouse(mouse).await?;
                Ok(true)
            }
            Event::Key(key) => self.handle_key(key).await,
            _ => Ok(false),
        }
    }
}
