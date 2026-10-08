//! Automatic handoff from terminals without native PDF graphics to XTerm.

use super::*;
use std::ffi::OsStr;

const HANDOFF_ENV: &str = "YOCTUI_GRAPHICS_TERMINAL_HANDOFF";
pub(crate) fn uses_interactive_terminal(cli: &Cli) -> bool {
    !cli.headless
        && matches!(
            cli.command,
            None | Some(Command::Attach | Command::Build { .. })
        )
}
const XTERM_ARGUMENTS: [&str; 6] = ["-ti", "vt340", "-fa", "Monospace", "-fs", "14"];

pub(crate) fn handoff_if_needed() -> Result<Option<std::process::ExitStatus>> {
    if handoff_disabled_by_environment()
        || terminal_graphics::detect_hardware_graphics_capability()
            == yoctui_model::HardwareGraphicsCapability::Sixel
    {
        return Ok(None);
    }
    let Some(xterm) = executable_on_initialized_path("xterm") else {
        return Ok(None);
    };
    let executable = env::current_exe().context("could not resolve the Yoctui executable")?;
    let size = crossterm::terminal::size().unwrap_or((80, 24));
    let status = graphics_terminal_command(&xterm, &executable, env::args_os().skip(1), size)
        .status()
        .with_context(|| format!("could not open graphics terminal {}", xterm.display()))?;
    if !status.success() {
        anyhow::bail!("graphics terminal {} exited with {status}", xterm.display());
    }
    Ok(Some(status))
}

fn handoff_disabled_by_environment() -> bool {
    env::var_os(HANDOFF_ENV).is_some()
        || env::var_os("YOCTUI_TERMINAL_GRAPHICS").is_some()
        || env::var_os("DISPLAY").is_none()
}

fn graphics_terminal_command<I, S>(
    xterm: &Path,
    executable: &Path,
    arguments: I,
    size: (u16, u16),
) -> ProcessCommand
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = ProcessCommand::new(xterm);
    command
        .args(XTERM_ARGUMENTS)
        .args([
            "-geometry",
            &format!("{}x{}", size.0.clamp(80, 140), size.1.clamp(24, 40)),
            "-e",
        ])
        .arg(executable)
        .args(arguments)
        .env(HANDOFF_ENV, "1");
    command
}

#[cfg(test)]
#[path = "tests/graphics_terminal_handoff.rs"]
mod tests;
