use super::*;
use yoctui_model::DaemonControl;
use yoctui_protocol::daemon::{
    ClientMessage, DaemonEvent, DaemonSnapshot, LifecycleState, ServerMessage,
    apply_sequenced_event,
};

pub(crate) fn busy(snapshot: &DaemonSnapshot) -> bool {
    let active = |state| {
        matches!(
            state,
            LifecycleState::Connecting | LifecycleState::Running | LifecycleState::Stopping
        )
    };
    snapshot.jobs.iter().any(|job| active(job.lifecycle))
        || snapshot
            .pty_sessions
            .iter()
            .any(|session| active(session.lifecycle))
        || snapshot
            .raw_executions
            .iter()
            .any(|execution| !execution.phase.is_terminal())
}

pub(crate) fn configuration(executable: &Path, build: &Path, source: &Path) -> Result<String> {
    let quote = |path: &Path| -> Result<String> {
        let value = path.to_str().context("non UTF-8 path")?;
        if value.chars().any(char::is_control) {
            anyhow::bail!("control characters in path");
        }
        Ok(format!(
            "\"{}\"",
            value
                .replace('%', "%%")
                .replace('$', "$$")
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
        ))
    };
    Ok(format!(
        "[Service]\nExecStart=\nExecStart={} --build-dir {} daemon foreground\nWorkingDirectory={}\nEnvironment={}\n",
        quote(executable)?,
        quote(build)?,
        quote(build)?.replace("$$", "$"),
        quote(&PathBuf::from(format!(
            "YOCTUI_SOURCE_DIR={}",
            source.display()
        )))?
        .replace("$$", "$")
    ))
}

pub(crate) async fn execute(action: DaemonControl, build: &str, source: &str) -> Result<String> {
    let properties = super::daemon_manager_io::service_properties().await?;
    if properties.get("LoadState").map(String::as_str) != Some("loaded") {
        anyhow::bail!("Install the local service first: yoctui daemon service install");
    }
    let active = properties
        .get("ActiveState")
        .is_some_and(|value| value == "active" || value == "activating");
    let pid = properties
        .get("MainPID")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    let record = daemon_is_available().ok();
    if let Some(record) = &record {
        if pid != record.pid {
            anyhow::bail!(
                "The connected daemon is not owned by yoctui.service; refusing to control another process"
            );
        }
        if action == DaemonControl::Start {
            anyhow::bail!("Daemon already running");
        }
        let (snapshot, clients) = tokio::task::spawn_blocking(|| {
            let (mut connection, mut snapshot) = daemon_connection_with_snapshot()?;
            let deadline = Instant::now() + Duration::from_secs(8);
            let clients = loop {
                anyhow::ensure!(
                    Instant::now() < deadline,
                    "fresh daemon telemetry was not received"
                );
                connection.set_timeout(Some(deadline.saturating_duration_since(Instant::now())))?;
                let ServerMessage::Event(event) = connection.receive()? else {
                    anyhow::bail!("daemon did not provide current incremental health");
                };
                apply_sequenced_event(&mut snapshot, &event)?;
                if let DaemonEvent::Telemetry(telemetry) = event.event {
                    break usize::from(telemetry.connected_clients);
                }
            };
            let _ = connection.send(&ClientMessage::Detach);
            Ok::<_, anyhow::Error>((snapshot, clients))
        })
        .await??;
        if busy(&snapshot) {
            anyhow::bail!(
                "Daemon has active work or terminal sessions; finish or close them before stopping/reconfiguring"
            );
        }
        if clients > 2 {
            anyhow::bail!("Other clients are attached; disconnect them before daemon control");
        }
    } else if active {
        anyhow::bail!(
            "Service is active but IPC health cannot be verified; refusing disruptive changes"
        );
    }
    if action == DaemonControl::Configure {
        let service_path = super::daemon_service::daemon_service_path()?;
        anyhow::ensure!(
            properties
                .get("FragmentPath")
                .is_some_and(|path| Path::new(path) == service_path),
            "Service configuration belongs to a different configuration root; refusing to write an ineffective drop-in"
        );
        let build_path = Path::new(build);
        if !build_path.is_absolute() {
            anyhow::bail!("Build directory must be absolute");
        }
        let canonical = build_path.canonicalize()?;
        if canonical == Path::new("/")
            || !canonical.join("conf/local.conf").is_file()
            || !canonical.join("conf/bblayers.conf").is_file()
        {
            anyhow::bail!(
                "Choose an initialized build directory with conf/local.conf and conf/bblayers.conf"
            );
        }
        let executable = env::current_exe()?.canonicalize()?;
        anyhow::ensure!(
            Path::new(source).is_absolute(),
            "Source directory must be absolute"
        );
        let source = Path::new(source).canonicalize()?;
        anyhow::ensure!(
            Path::new(source.to_str().context("non UTF-8 source")?).is_absolute()
                && source.join("oe-init-build-env").is_file(),
            "Source directory needs oe-init-build-env"
        );
        let contents = configuration(&executable, &canonical, &source)?;
        let destination = super::daemon_service::daemon_service_path()?
            .with_file_name("yoctui.service.d")
            .join("zz-yoctui-manager.conf");
        super::daemon_service::write_daemon_service(&destination, &contents)?;
        super::daemon_manager_io::command("systemctl", &["--user", "daemon-reload"]).await?;
    }
    let verb = match action {
        DaemonControl::Start => "start",
        DaemonControl::Stop => "stop",
        DaemonControl::Restart | DaemonControl::Configure => "restart",
    };
    super::daemon_manager_io::command("systemctl", &["--user", verb, "yoctui.service"]).await?;
    Ok(format!(
        "{} completed; client reconnects automatically when the daemon is ready",
        action.label()
    ))
}

#[cfg(test)]
#[path = "tests/daemon_manager_control.rs"]
mod tests;
