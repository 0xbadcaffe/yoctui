//! Bounded, client-local inspection and controls for the single local daemon.
use super::*;
use tokio::io::AsyncReadExt;
use yoctui_model::{DaemonControl, DaemonManagerState};

pub(crate) async fn command(program: &str, arguments: &[&str]) -> Result<String> {
    let mut child = tokio::process::Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut stdout = child
        .stdout
        .take()
        .context("missing stdout")?
        .take(128 * 1024);
    let mut stderr = child.stderr.take().context("missing stderr")?.take(8192);
    let read = async {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let (a, b, status) = tokio::join!(
            stdout.read_to_end(&mut out),
            stderr.read_to_end(&mut err),
            child.wait()
        );
        a?;
        b?;
        if !status?.success() {
            anyhow::bail!("{}: {}", program, clean(&String::from_utf8_lossy(&err)));
        }
        Ok::<_, anyhow::Error>(clean(&String::from_utf8_lossy(&out)))
    };
    tokio::time::timeout(Duration::from_secs(15), read)
        .await
        .context("daemon manager command timed out")?
}

fn clean(value: &str) -> String {
    yoctui_utils::strip_ansi(value)
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

pub(crate) async fn service_properties() -> Result<BTreeMap<String, String>> {
    let output = command("systemctl", &["--user", "show", "yoctui.service", "--no-pager",
        "--property=LoadState,ActiveState,SubState,MainPID,UnitFileState,Result,ExecMainStatus,FragmentPath,DropInPaths,MemoryCurrent,CPUUsageNSec,ActiveEnterTimestamp"]).await?;
    Ok(output
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.into(), v.into()))
        .collect())
}

pub(crate) async fn inspect() -> DaemonManagerState {
    let mut state = DaemonManagerState::default();
    let host = command("hostname", &[])
        .await
        .unwrap_or_else(|error| format!("unknown ({error})"));
    let ips = command("hostname", &["-I"])
        .await
        .unwrap_or_else(|_| "unknown".into());
    state.details.extend([
        format!("Host: {}", host.trim()),
        format!("Local IP addresses: {}", ips.trim()),
        "Transport: local Unix socket (no TCP listener / remote daemon in this version)".into(),
    ]);
    if let Ok(paths) = yoctui_protocol::daemon_ipc::runtime_paths() {
        state.details.push(format!("Runtime: {paths:?}"));
    }
    match service_properties().await {
        Ok(properties) => {
            state.service_active = properties
                .get("ActiveState")
                .is_some_and(|value| value == "active");
            state.service_installed = properties
                .get("LoadState")
                .is_some_and(|value| value == "loaded");
            state.details.extend(
                properties
                    .into_iter()
                    .map(|(key, value)| format!("{key}: {value}")),
            );
        }
        Err(error) => state
            .details
            .push(format!("systemd user service unavailable: {error}")),
    }
    // Health/activity comes from the existing live client replica. Do not
    // request another potentially large workspace snapshot every refresh.
    if let Ok(record) = daemon_is_available() {
        state.details.push(format!(
            "Daemon PID: {} · started {} ms since epoch",
            record.pid, record.started_unix_ms
        ));
        state
            .details
            .push(format!("Executable: {}", record.executable.display()));
        let executable = format!("/proc/{}/exe", record.pid);
        if Path::new(&executable).exists() {
            let version = command(&executable, &["--version"])
                .await
                .unwrap_or_else(|_| "unknown".into());
            state
                .details
                .push(format!("Running daemon binary: {}", version.trim()));
        }
    } else {
        state
            .details
            .push("Daemon runtime record: unavailable".into());
    }
    state.logs = match command(
        "journalctl",
        &[
            "--user",
            "-u",
            "yoctui.service",
            "-n",
            "200",
            "--no-pager",
            "-o",
            "short-iso",
        ],
    )
    .await
    {
        Ok(logs) => logs.lines().map(str::to_owned).collect(),
        Err(error) => vec![format!("Daemon service journal unavailable: {error}")],
    };
    state.details.extend(
        state
            .logs
            .iter()
            .rev()
            .filter(|line| line.contains("daemon discovery:"))
            .take(3)
            .map(|line| format!("Last discovery report: {line}")),
    );
    state
}

pub(crate) async fn control(
    action: DaemonControl,
    build: String,
    source: String,
) -> Result<String> {
    super::daemon_manager_control::execute(action, &build, &source).await
}
