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
    pub(crate) prefix_arguments: Vec<String>,
}

const DETACHED_TERMINAL_STARTUP_PROBE: Duration = Duration::from_millis(400);

pub(crate) struct DetachedTerminalOperation {
    pub(crate) name: String,
    pub(crate) handle: tokio::task::JoinHandle<Result<()>>,
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
    for (name, prefix_arguments) in [
        ("gnome-terminal", &["--wait", "--"][..]),
        ("kgx", &["--wait", "--"][..]),
        ("konsole", &["--nofork", "-e"][..]),
        ("terminator", &["--no-dbus", "--execute"][..]),
        ("xterm", &["-e"][..]),
        ("x-terminal-emulator", &["-e"][..]),
    ] {
        if let Some(program) = executable_on_initialized_path(name) {
            return Ok(DetachedTerminalLauncher {
                program,
                prefix_arguments: prefix_arguments
                    .iter()
                    .map(|argument| (*argument).into())
                    .collect(),
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
    let (requested_program, arguments) =
        if matches!(request.kind, yoctui_model::TerminalCreationKind::Menuconfig) {
            crate::menuconfig_relay::command(&request.program, &request.arguments)?
        } else {
            (request.program.clone(), request.arguments.clone())
        };
    let program = requested_program
        .canonicalize()
        .with_context(|| format!("could not resolve {}", requested_program.display()))?;
    let metadata = fs::metadata(&program)?;
    if !metadata.is_file() {
        anyhow::bail!("detached terminal command is not a regular file");
    }
    let mut command = ProcessCommand::new(&launcher.program);
    command
        .args(&launcher.prefix_arguments)
        .arg(program)
        .args(arguments)
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
    let command = detached_terminal_command(&launcher, request)?;
    launch_and_probe_detached_terminal(command, &launcher.program, DETACHED_TERMINAL_STARTUP_PROBE)
}

pub(crate) fn launch_and_probe_detached_terminal(
    mut command: ProcessCommand,
    launcher: &Path,
    startup_probe: Duration,
) -> Result<()> {
    let mut child = command
        .spawn()
        .with_context(|| format!("could not spawn detached terminal {}", launcher.display()))?;
    std::thread::sleep(startup_probe);
    if let Some(status) = child
        .try_wait()
        .with_context(|| format!("could not inspect detached terminal {}", launcher.display()))?
    {
        anyhow::bail!(
            "detached terminal {} exited during startup ({status})",
            launcher.display()
        );
    }
    Ok(())
}

pub(crate) fn begin_detached_terminal_launch(
    app: &mut yoctui_model::App,
    operation: &mut Option<DetachedTerminalOperation>,
    request: yoctui_model::TerminalLaunchRequest,
) {
    if operation.is_some() {
        app.notification = Some("A detached terminal is already starting.".into());
        return;
    }
    let name = request.name.clone();
    app.notification = Some(format!("Opening detached terminal for {name}…"));
    let handle = tokio::task::spawn_blocking(move || launch_detached_terminal(&request));
    *operation = Some(DetachedTerminalOperation { name, handle });
}

pub(crate) async fn poll_detached_terminal_launch(
    app: &mut yoctui_model::App,
    operation: &mut Option<DetachedTerminalOperation>,
) {
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(operation) = operation.take() else {
        return;
    };
    app.notification = Some(match operation.handle.await {
        Ok(Ok(())) => format!("Detached terminal opened for {}.", operation.name),
        Ok(Err(error)) => format!("Could not open detached terminal: {error}"),
        Err(error) => format!("Could not open detached terminal: {error}"),
    });
}
