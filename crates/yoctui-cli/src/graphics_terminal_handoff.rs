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
const XTERM_ARGUMENTS: [&str; 9] = [
    "-ti",
    "vt340",
    "-fa",
    "Monospace",
    "-fs",
    "14",
    "-geometry",
    "140x40",
    "-e",
];

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
    let status = graphics_terminal_command(&xterm, &executable, env::args_os().skip(1))
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

fn graphics_terminal_command<I, S>(xterm: &Path, executable: &Path, arguments: I) -> ProcessCommand
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = ProcessCommand::new(xterm);
    command
        .args(XTERM_ARGUMENTS)
        .arg(executable)
        .args(arguments)
        .env(HANDOFF_ENV, "1");
    command
}

#[cfg(test)]
#[path = "tests/graphics_terminal_handoff.rs"]
mod tests;
