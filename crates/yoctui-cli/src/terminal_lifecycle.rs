//! Terminal lifecycle.
use super::*;

#[derive(Default)]
pub(crate) struct RedrawLatch(pub(crate) Cell<bool>);

impl RedrawLatch {
    pub(crate) fn request(&self) {
        self.0.set(true);
    }

    pub(crate) fn take(&self) -> bool {
        self.0.replace(false)
    }
}

pub(crate) struct TerminalGuard {
    pub(crate) redraw: RedrawLatch,
    pub(crate) application_title: String,
    pub(crate) shell_title: String,
}

impl TerminalGuard {
    pub(crate) fn enter() -> Result<Self> {
        let shell_title = env::var("USER").unwrap_or_else(|_| "terminal".into());
        let application_title = format!("yoctui · {shell_title}");
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            SetTitle(&application_title),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste,
            Hide
        )?;
        Ok(Self {
            redraw: RedrawLatch::default(),
            application_title,
            shell_title,
        })
    }
    pub(crate) fn suspend(&self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            io::stdout(),
            Show,
            DisableBracketedPaste,
            DisableMouseCapture,
            LeaveAlternateScreen,
            SetTitle(&self.shell_title)
        )?;
        Ok(())
    }
    pub(crate) fn resume(&self) -> Result<()> {
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            SetTitle(&self.application_title),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste,
            Hide
        )?;
        self.redraw.request();
        Ok(())
    }

    pub(crate) fn take_full_redraw_request(&self) -> bool {
        self.redraw.take()
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
        let _ = execute!(io::stdout(), SetTitle(&self.shell_title));
    }
}

pub(crate) fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        Show,
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    );
}

pub(crate) fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous(info);
    }));
}
