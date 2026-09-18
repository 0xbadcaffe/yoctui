//! Daemon service.
use super::*;

#[cfg(unix)]
pub(crate) fn daemon_service(command: DaemonServiceCommand) -> Result<()> {
    match command {
        DaemonServiceCommand::Install => {
            require_systemd_user()?;
            let destination = daemon_service_path()?;
            let executable = env::current_exe()?.canonicalize()?;
            write_daemon_service(&destination, &daemon_service_unit(&executable)?)?;
            run_systemctl_user(&["daemon-reload"])?;
            println!("Installed {}", destination.display());
            println!("Enable auto-start with: systemctl --user enable --now yoctui.service");
            Ok(())
        }
        DaemonServiceCommand::Uninstall => {
            require_systemd_user()?;
            let destination = daemon_service_path()?;
            let _ = run_systemctl_user(&["disable", "--now", "yoctui.service"]);
            match fs::symlink_metadata(&destination) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                    anyhow::bail!(
                        "refusing to remove unsafe service file {}",
                        destination.display()
                    )
                }
                Ok(_) => fs::remove_file(&destination)?,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            run_systemctl_user(&["daemon-reload"])?;
            println!("Removed {}", destination.display());
            Ok(())
        }
        DaemonServiceCommand::Start => run_systemctl_user(&["start", "yoctui.service"]),
        DaemonServiceCommand::Stop => run_systemctl_user(&["stop", "yoctui.service"]),
        DaemonServiceCommand::Restart => run_systemctl_user(&["restart", "yoctui.service"]),
        DaemonServiceCommand::Status => {
            run_systemctl_user(&["status", "--no-pager", "--lines=20", "yoctui.service"])
        }
    }
}

#[cfg(unix)]
pub(crate) fn daemon_service_path() -> Result<PathBuf> {
    let root = yoctui_utils::config_dir()
        .context("XDG_CONFIG_HOME or HOME is required for systemd user service installation")?;
    if !root.is_absolute() {
        anyhow::bail!("systemd user configuration path must be absolute");
    }
    Ok(root.join("systemd/user/yoctui.service"))
}

#[cfg(unix)]
pub(crate) fn daemon_service_unit(executable: &Path) -> Result<String> {
    let executable = executable
        .to_str()
        .context("Yoctui executable path is not valid UTF-8")?;
    if executable.chars().any(char::is_control) {
        anyhow::bail!("Yoctui executable path contains control characters");
    }
    let escaped = executable
        .replace('%', "%%")
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    Ok(format!(
        "[Unit]\nDescription=Yoctui persistent daemon\nDocumentation=https://github.com/0xbadcaffe/yoctui\n\n[Service]\nType=simple\nExecStart=\"{escaped}\" daemon foreground\nRestart=on-failure\nRestartSec=2s\nNoNewPrivileges=true\n\n[Install]\nWantedBy=default.target\n"
    ))
}

#[cfg(unix)]
pub(crate) fn write_daemon_service(destination: &Path, contents: &str) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let directory = destination
        .parent()
        .context("service destination has no parent")?;
    fs::create_dir_all(directory)?;
    if let Ok(metadata) = fs::symlink_metadata(destination)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        anyhow::bail!(
            "refusing to replace unsafe service file {}",
            destination.display()
        );
    }
    let temporary = directory.join(format!("yoctui.service.{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    let result = (|| -> Result<(), io::Error> {
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    Ok(())
}

#[cfg(unix)]
pub(crate) fn require_systemd_user() -> Result<()> {
    let output = ProcessCommand::new("systemctl")
        .args(["--user", "show-environment"])
        .stdin(Stdio::null())
        .output();
    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => anyhow::bail!(
            "systemd user manager is unavailable (exit {}): {}\nUse `yoctui daemon start` for the direct-process fallback.",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => anyhow::bail!(
            "systemctl --user is unavailable: {error}\nUse `yoctui daemon start` for the direct-process fallback."
        ),
    }
}

#[cfg(unix)]
pub(crate) fn run_systemctl_user(arguments: &[&str]) -> Result<()> {
    require_systemd_user()?;
    let status = ProcessCommand::new("systemctl")
        .arg("--user")
        .args(arguments)
        .status()?;
    if !status.success() {
        anyhow::bail!(
            "systemctl --user {} exited with {status}\nUse `yoctui daemon start` for the direct-process fallback.",
            arguments.join(" ")
        );
    }
    Ok(())
}
