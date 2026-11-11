//! Terminal lifecycle.
use super::*;

/// A closed PTY remains readable forever; it must not reach the event reader.
#[cfg(unix)]
pub(crate) fn terminal_disconnected() -> io::Result<bool> {
    let mut descriptor = libc::pollfd {
        fd: libc::STDIN_FILENO,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: descriptor is valid for the single entry passed to poll.
    if unsafe { libc::poll(&mut descriptor, 1, 0) } < 0 {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            return Ok(false);
        }
        return Err(error);
    }
    Ok(descriptor.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0)
}

fn enter_raw_input_mode() -> io::Result<()> {
    enable_raw_mode()?;
    #[cfg(unix)]
    {
        // Crossterm's poll backend drains input until a read returns zero.
        // Use terminal read settings rather than O_NONBLOCK, which would also
        // change the file description inherited by the parent shell.
        let mut settings = std::mem::MaybeUninit::<libc::termios>::uninit();
        // SAFETY: tcgetattr initializes settings on success; stdin is open.
        if unsafe { libc::tcgetattr(libc::STDIN_FILENO, settings.as_mut_ptr()) } < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: tcgetattr succeeded above.
        let mut settings = unsafe { settings.assume_init() };
        settings.c_cc[libc::VMIN] = 0;
        settings.c_cc[libc::VTIME] = 0;
        // SAFETY: settings contains valid terminal attributes.
        if unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &settings) } < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

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
        enter_raw_input_mode()?;
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
        enter_raw_input_mode()?;
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

    pub(crate) fn stop_input_capture(&self) {
        let _ = execute!(io::stdout(), DisableBracketedPaste, DisableMouseCapture);
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
        let _ = execute!(io::stdout(), SetTitle(&self.shell_title));
    }
}

pub(crate) fn restore_terminal() {
    let _ = execute!(
        io::stdout(),
        Show,
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    );
    // Mouse reports may still be queued after the quit confirmation was read.
    // Restoring terminal modes alone leaves those bytes for the parent shell,
    // where their semicolons and '<' can be interpreted as shell commands.
    #[cfg(unix)]
    if io::stdin().is_terminal() {
        // SAFETY: stdin is the current process's open terminal descriptor.
        // Discard only pending input owned by the exiting UI, never pipe data.
        let _ = unsafe { libc::tcflush(libc::STDIN_FILENO, libc::TCIFLUSH) };
    }
    let _ = disable_raw_mode();
}

pub(crate) fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous(info);
    }));
}
