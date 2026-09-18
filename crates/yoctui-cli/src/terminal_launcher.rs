//! Terminal launcher.
use super::*;

pub(crate) fn initialized_path_directories() -> Vec<PathBuf> {
    env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetachedTerminalLauncher {
    pub(crate) program: PathBuf,
    pub(crate) command_separator: &'static str,
}

pub(crate) fn executable_on_initialized_path(name: &str) -> Option<PathBuf> {
    initialized_path_directories()
        .into_iter()
        .map(|directory| directory.join(name))
        .find(|candidate| fs::metadata(candidate).is_ok_and(|metadata| executable(&metadata)))
}

pub(crate) fn executable(metadata: &fs::Metadata) -> bool {
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub(crate) fn detect_detached_terminal_launcher() -> Result<DetachedTerminalLauncher, String> {
    if env::var_os("DISPLAY").is_none() && env::var_os("WAYLAND_DISPLAY").is_none() {
        return Err("no DISPLAY or WAYLAND_DISPLAY is available".into());
    }
    for (name, separator) in [
        ("x-terminal-emulator", "-e"),
        ("gnome-terminal", "--"),
        ("konsole", "-e"),
        ("kgx", "--"),
        ("xterm", "-e"),
    ] {
        if let Some(program) = executable_on_initialized_path(name) {
            return Ok(DetachedTerminalLauncher {
                program,
                command_separator: separator,
            });
        }
    }
    Err("no supported terminal emulator was found on PATH".into())
}

pub(crate) fn detached_terminal_availability() -> yoctui_model::DetachedTerminalAvailability {
    match detect_detached_terminal_launcher() {
        Ok(launcher) => yoctui_model::DetachedTerminalAvailability::Available {
            launcher: launcher
                .program
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("terminal")
                .into(),
        },
        Err(reason) => yoctui_model::DetachedTerminalAvailability::Unavailable { reason },
    }
}

pub(crate) fn ssh_client_capability() -> yoctui_model::SshClientCapability {
    let Some(executable) = executable_on_initialized_path("ssh") else {
        return yoctui_model::SshClientCapability::Missing;
    };
    match executable.canonicalize() {
        Ok(executable) if executable.is_absolute() => {
            yoctui_model::SshClientCapability::Available { executable }
        }
        Ok(_) => yoctui_model::SshClientCapability::Failed {
            message: "resolved ssh executable is not absolute".into(),
        },
        Err(error) => yoctui_model::SshClientCapability::Failed {
            message: format!("could not resolve ssh executable: {error}"),
        },
    }
}

pub(crate) fn detached_terminal_command(
    launcher: &DetachedTerminalLauncher,
    request: &yoctui_model::TerminalLaunchRequest,
) -> Result<ProcessCommand> {
    if !request.cwd.is_absolute() || !request.cwd.is_dir() {
        anyhow::bail!(
            "detached terminal working directory is unavailable: {}",
            request.cwd.display()
        );
    }
    if !request.program.is_absolute() {
        anyhow::bail!("detached terminal command must be an absolute executable");
    }
    let program = request
        .program
        .canonicalize()
        .with_context(|| format!("could not resolve {}", request.program.display()))?;
    let metadata = fs::metadata(&program)?;
    if !metadata.is_file() {
        anyhow::bail!("detached terminal command is not a regular file");
    }
    let mut command = ProcessCommand::new(&launcher.program);
    command
        .arg(launcher.command_separator)
        .arg(program)
        .args(&request.arguments)
        .current_dir(&request.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    Ok(command)
}

pub(crate) fn launch_detached_terminal(
    request: &yoctui_model::TerminalLaunchRequest,
) -> Result<()> {
    let launcher = detect_detached_terminal_launcher().map_err(anyhow::Error::msg)?;
    detached_terminal_command(&launcher, request)?
        .spawn()
        .with_context(|| {
            format!(
                "could not spawn detached terminal {}",
                launcher.program.display()
            )
        })?;
    Ok(())
}
