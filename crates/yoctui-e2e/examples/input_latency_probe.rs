use crossterm::{
    event::{self, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use std::{io, io::Write as _, time::SystemTime};
use yoctui_app::{Input, MouseInput, MouseKind, focus_action_for_app, mouse_action_for_app};
use yoctui_model::{App, FocusTarget, update};

const OBSERVATIONS_PER_KIND: usize = 100;

fn monotonic_ns() -> io::Result<u64> {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut value) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((value.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(value.tv_nsec as u64))
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            event::EnableMouseCapture
        )?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            event::DisableMouseCapture,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }
}

fn key_input(event: crossterm::event::KeyEvent) -> Option<Input> {
    match event.code {
        KeyCode::Up => Some(Input::Up),
        KeyCode::Down => Some(Input::Down),
        _ => None,
    }
}

fn mouse_input(event: crossterm::event::MouseEvent) -> Option<MouseInput> {
    let kind = match event.kind {
        MouseEventKind::ScrollUp => MouseKind::ScrollUp,
        MouseEventKind::ScrollDown => MouseKind::ScrollDown,
        _ => return None,
    };
    Some(MouseInput {
        kind,
        column: event.column,
        row: event.row,
    })
}

fn marker(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    kind: &str,
    sequence: usize,
    received_ns: u64,
    model_ns: u64,
    frame_ns: u64,
    selection: usize,
) -> io::Result<()> {
    write!(
        terminal.backend_mut(),
        "\x1b]777;yoctui-input-latency;{kind};{sequence};{received_ns};{model_ns};{frame_ns};{selection}\x07"
    )?;
    terminal.backend_mut().flush()
}

fn main() -> io::Result<()> {
    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new(128, 1024 * 1024);
    app.focus = FocusTarget::Navigator;
    terminal.draw(|frame| yoctui_ui::render_at(frame, &app, SystemTime::now()))?;
    write!(
        terminal.backend_mut(),
        "\x1b]777;yoctui-input-latency;ready;{OBSERVATIONS_PER_KIND}\x07"
    )?;
    terminal.backend_mut().flush()?;

    let mut keyboard = 0;
    let mut mouse = 0;
    while keyboard < OBSERVATIONS_PER_KIND || mouse < OBSERVATIONS_PER_KIND {
        let terminal_event = event::read()?;
        let received_ns = monotonic_ns()?;
        let (kind, sequence, action) = match terminal_event {
            Event::Key(key) if keyboard < OBSERVATIONS_PER_KIND => {
                let Some(input) = key_input(key) else {
                    continue;
                };
                let Some(action) = focus_action_for_app(&app, input) else {
                    continue;
                };
                keyboard += 1;
                ("keyboard", keyboard, action)
            }
            Event::Mouse(event) if mouse < OBSERVATIONS_PER_KIND => {
                let Some(input) = mouse_input(event) else {
                    continue;
                };
                let Some(action) = mouse_action_for_app(input, &app, 160, 50) else {
                    continue;
                };
                mouse += 1;
                ("mouse", mouse, action)
            }
            _ => continue,
        };
        let _ = update(&mut app, action);
        let model_ns = monotonic_ns()?;
        terminal.draw(|frame| yoctui_ui::render_at(frame, &app, SystemTime::now()))?;
        let frame_ns = monotonic_ns()?;
        marker(
            &mut terminal,
            kind,
            sequence,
            received_ns,
            model_ns,
            frame_ns,
            app.navigator_selection,
        )?;
    }
    Ok(())
}
