//! Daemon commands.
use super::*;

#[cfg(unix)]
pub(crate) async fn daemon_cli(command: DaemonCliCommand) -> Result<()> {
    match command {
        DaemonCliCommand::Start => start_daemon().await,
        DaemonCliCommand::Build { targets } => daemon_start_build(targets),
        DaemonCliCommand::Status => daemon_status(),
        DaemonCliCommand::Stop => stop_daemon(),
        DaemonCliCommand::Restart => {
            if daemon_is_available().is_ok() {
                stop_daemon()?;
            }
            start_daemon().await
        }
        DaemonCliCommand::Foreground => {
            let mut termination = termination_receiver()?;
            run_daemon_foreground(&mut termination).await
        }
        DaemonCliCommand::Service { command } => daemon_service(command),
    }
}

#[cfg(not(unix))]
pub(crate) async fn daemon_cli(_command: DaemonCliCommand) -> Result<()> {
    anyhow::bail!("Yoctui daemon mode currently requires secure Unix peer credentials")
}

#[cfg(unix)]
pub(crate) const DAEMON_STARTUP_TIMEOUT: Duration = Duration::from_secs(180);

#[cfg(unix)]
pub(crate) struct DaemonStartupChild {
    pub(crate) child: std::process::Child,
    pub(crate) ready: bool,
}

#[cfg(unix)]
impl Drop for DaemonStartupChild {
    fn drop(&mut self) {
        if self.ready || self.child.try_wait().ok().flatten().is_some() {
            return;
        }
        // Only the unreaped foreground child spawned by this startup attempt.
        unsafe {
            libc::kill(self.child.id() as i32, libc::SIGTERM);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(unix)]
pub(crate) async fn start_daemon() -> Result<()> {
    use yoctui_protocol::daemon_ipc::{DaemonConnection, runtime_paths};
    let paths = runtime_paths()?;
    if DaemonConnection::connect(&paths, Duration::from_millis(50)).is_ok() {
        anyhow::bail!(
            "Yoctui daemon is already running at {}",
            paths.socket.display()
        );
    }
    let executable = env::current_exe().context("could not resolve the Yoctui executable")?;
    let mut command = ProcessCommand::new(executable);
    command.args(["daemon", "foreground"]).stdin(Stdio::null());
    if env::var_os("BUILDDIR").is_none()
        && let Some(profile) = inferred_build_environment_profile(&env::current_dir()?)
    {
        let initialized = BuildEnvironmentAdapter::default()
            .initialize(profile)
            .await
            .context(
                "could not initialize the Yocto environment from the current build directory",
            )?;
        command.env_clear().envs(initialized.environment);
    }
    if let Some(log_path) = env::var_os("YOCTUI_DAEMON_LOG") {
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .context("could not open YOCTUI_DAEMON_LOG")?;
        command
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log));
    } else {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }
    command.process_group(0);
    let child = command
        .spawn()
        .context("could not start the Yoctui daemon")?;
    let mut child = DaemonStartupChild {
        child,
        ready: false,
    };
    let deadline = Instant::now() + DAEMON_STARTUP_TIMEOUT;
    let interactive = io::stderr().is_terminal();
    let mut indicator_phase = 0usize;
    let mut next_indicator_frame = Instant::now();
    loop {
        if interactive && Instant::now() >= next_indicator_frame {
            eprint!(
                "\r{} Starting Yoctui daemon…",
                yoctui_ui::startup_activity_symbol(indicator_phase)
            );
            let _ = io::stderr().flush();
            indicator_phase = indicator_phase.wrapping_add(1);
            next_indicator_frame = Instant::now() + Duration::from_millis(80);
        }
        if let Some(status) = child.child.try_wait()? {
            if interactive {
                eprint!("\r\x1b[2K");
            }
            anyhow::bail!("Yoctui daemon exited during startup with {status}");
        }
        if let Ok(record) = daemon_is_available() {
            anyhow::ensure!(
                record.pid == child.child.id(),
                "another daemon owns the startup socket"
            );
            child.ready = true;
            if interactive {
                eprint!("\r\x1b[2K");
            }
            println!(
                "Yoctui daemon started (pid {}, instance {})",
                record.pid,
                format_instance(record.daemon_instance_id)
            );
            return Ok(());
        }
        if Instant::now() >= deadline {
            if interactive {
                eprint!("\r\x1b[2K");
            }
            anyhow::bail!(
                "Yoctui daemon did not become available at {} within {} seconds",
                paths.socket.display(),
                DAEMON_STARTUP_TIMEOUT.as_secs()
            );
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[cfg(unix)]
pub(crate) fn inferred_build_environment_profile(
    working_directory: &Path,
) -> Option<yoctui_model::BuildEnvironmentProfile> {
    let build_dir = working_directory.canonicalize().ok()?;
    if !build_dir.join("conf/local.conf").is_file()
        || !build_dir.join("conf/bblayers.conf").is_file()
    {
        return None;
    }
    let source_dir = build_dir
        .ancestors()
        .find(|candidate| candidate.join("oe-init-build-env").is_file())?
        .to_path_buf();
    let init_script = source_dir.join("oe-init-build-env").canonicalize().ok()?;
    Some(yoctui_model::BuildEnvironmentProfile {
        source_dir,
        build_dir,
        init_script,
    })
}

#[cfg(all(test, unix))]
mod startup_environment_tests {
    use super::*;

    #[test]
    fn daemon_start_infers_initialized_build_directory_and_canonical_script() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!(
            "yoctui-daemon-cwd-{}-{}",
            std::process::id(),
            yoctui_utils::unix_ms()
        ));
        let build = root.join("build/romulus");
        let upstream = root.join("upstream");
        std::fs::create_dir_all(build.join("conf")).unwrap();
        std::fs::create_dir_all(&upstream).unwrap();
        std::fs::write(build.join("conf/local.conf"), "MACHINE = \"romulus\"\n").unwrap();
        std::fs::write(build.join("conf/bblayers.conf"), "BBLAYERS = \"\"\n").unwrap();
        std::fs::write(upstream.join("oe-init-build-env"), "#!/bin/sh\n").unwrap();
        symlink("upstream/oe-init-build-env", root.join("oe-init-build-env")).unwrap();

        let profile = inferred_build_environment_profile(&build).unwrap();
        assert_eq!(profile.source_dir, root.canonicalize().unwrap());
        assert_eq!(profile.build_dir, build.canonicalize().unwrap());
        assert_eq!(
            profile.init_script,
            upstream.join("oe-init-build-env").canonicalize().unwrap()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(unix)]
pub(crate) fn daemon_status() -> Result<()> {
    let record = daemon_is_available()?;
    println!("status: running");
    println!("pid: {}", record.pid);
    println!("instance: {}", format_instance(record.daemon_instance_id));
    println!("started_unix_ms: {}", record.started_unix_ms);
    println!(
        "socket: {}",
        yoctui_protocol::daemon_ipc::runtime_paths()?
            .socket
            .display()
    );
    if let Ok((mut connection, snapshot)) = daemon_connection_with_snapshot() {
        for job in snapshot.jobs {
            println!(
                "job {} {:?} {:?} exit_code={:?}",
                job.id.0, job.lifecycle, job.label, job.exit_code
            );
        }
        for event in &snapshot.build_events {
            match event {
                yoctui_protocol::daemon::DaemonBuildEvent::Reset { targets } => {
                    println!("build targets: {}", targets.join(" "));
                }
                yoctui_protocol::daemon::DaemonBuildEvent::CommandFailed { code, message } => {
                    println!("build error {code}: {message}");
                }
                yoctui_protocol::daemon::DaemonBuildEvent::Completed {
                    success, exit_code, ..
                } => {
                    println!("build completed success={success} exit_code={exit_code:?}");
                }
                _ => {}
            }
        }
        for log in snapshot.recent_logs.iter().rev().take(8).rev() {
            println!("log {:?} {}: {}", log.severity, log.source, log.message);
        }
        let _ = connection.send(&yoctui_protocol::daemon::ClientMessage::Detach);
        let _ = connection.receive::<yoctui_protocol::daemon::ServerMessage>();
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn daemon_start_build(targets: Vec<String>) -> Result<()> {
    anyhow::ensure!(
        !targets.is_empty(),
        "daemon build requires at least one target"
    );
    let (mut connection, snapshot) = daemon_connection_with_snapshot()?;
    let result = daemon_build::start(&mut connection, snapshot, targets);
    // Closing this one-shot client is cleanup, not a second build outcome.
    let _ = connection.send(&yoctui_protocol::daemon::ClientMessage::Detach);
    println!("daemon build: {:?}", result?);
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn daemon_start_build(_targets: Vec<String>) -> Result<()> {
    anyhow::bail!("daemon BitBake builds currently require Unix local IPC")
}

#[cfg(unix)]
pub(crate) fn daemon_connection_with_snapshot() -> Result<(
    yoctui_protocol::daemon_ipc::DaemonConnection,
    yoctui_protocol::daemon::DaemonSnapshot,
)> {
    use yoctui_protocol::{
        daemon::{
            Capability, ClientHello, ClientId, ClientMessage, ProtocolVersion, ServerMessage,
            Subscription,
        },
        daemon_ipc::{DaemonConnection, runtime_paths},
    };
    let paths = runtime_paths()?;
    let mut connection = DaemonConnection::connect(&paths, Duration::from_secs(1))?;
    // A busy BitBake build can produce a near-limit snapshot while the
    // daemon is serializing and writing it. Keep the lifecycle handshake
    // bounded, but allow enough time for a full local snapshot to arrive.
    connection.set_timeout(Some(Duration::from_secs(10)))?;
    connection.send(&ClientMessage::Hello(ClientHello {
        minimum_version: ProtocolVersion::CURRENT,
        maximum_version: ProtocolVersion::CURRENT,
        client_id: ClientId([2; 16]),
        client_name: "yoctui-cli".into(),
        capabilities: vec![
            Capability::StateSnapshots,
            Capability::IncrementalEvents,
            Capability::PtySessions,
            Capability::EnvironmentCompatibility,
            Capability::RawExecution,
        ],
    }))?;
    let ServerMessage::Hello(_) = connection.receive()? else {
        anyhow::bail!("daemon returned an unexpected hello response");
    };
    connection.send(&ClientMessage::Attach {
        workspace: None,
        subscription: Subscription {
            state: true,
            jobs: true,
            logs: true,
            pty_sessions: Vec::new(),
        },
        resume: None,
    })?;
    let ServerMessage::Attached { snapshot, .. } = connection.receive()? else {
        anyhow::bail!("daemon returned an unexpected attach response");
    };
    Ok((connection, snapshot))
}

#[cfg(unix)]
pub(crate) fn daemon_sessions() -> Result<()> {
    let (mut connection, snapshot) = daemon_connection_with_snapshot()?;
    if snapshot.pty_sessions.is_empty() {
        println!("no daemon terminal sessions");
    } else {
        for session in snapshot.pty_sessions {
            println!(
                "{}\t{}\t{:?}\t{} viewer(s)",
                session.id.0, session.name, session.lifecycle, session.viewers
            );
        }
    }
    connection.send(&yoctui_protocol::daemon::ClientMessage::Detach)?;
    let _ = connection.receive::<yoctui_protocol::daemon::ServerMessage>()?;
    Ok(())
}

#[cfg(unix)]
pub(crate) fn daemon_session_command(command: SessionCliCommand) -> Result<()> {
    match command {
        SessionCliCommand::Attach { id } => {
            let (mut connection, snapshot) = daemon_connection_with_snapshot()?;
            anyhow::ensure!(
                snapshot
                    .pty_sessions
                    .iter()
                    .any(|session| session.id.0 == id),
                "daemon PTY session {id} was not found"
            );
            println!("session {id} is available; start `yoctui attach` for the interactive client");
            connection.send(&yoctui_protocol::daemon::ClientMessage::Detach)?;
            let _ = connection.receive::<yoctui_protocol::daemon::ServerMessage>()?;
            Ok(())
        }
        SessionCliCommand::Kill { id, force } => {
            anyhow::ensure!(force, "session kill is destructive; repeat with --force");
            let (mut connection, snapshot) = daemon_connection_with_snapshot()?;
            anyhow::ensure!(
                snapshot
                    .pty_sessions
                    .iter()
                    .any(|session| session.id.0 == id),
                "daemon PTY session {id} was not found"
            );
            use yoctui_protocol::daemon::{
                ClientMessage, CommandRequest, DaemonCommand, PtySessionId, RequestId,
                ServerMessage,
            };
            connection.send(&ClientMessage::Command(CommandRequest {
                request_id: RequestId(1),
                expected_generation: Some(snapshot.generation),
                command: DaemonCommand::TerminatePty {
                    session_id: PtySessionId(id),
                    force: true,
                    confirmation: None,
                },
            }))?;
            loop {
                match connection.receive::<ServerMessage>()? {
                    ServerMessage::CommandResult(result) => {
                        println!("session {id}: {:?}", result.outcome);
                        break;
                    }
                    ServerMessage::Event(_) => {}
                    response => anyhow::bail!("unexpected daemon response: {response:?}"),
                }
            }
            connection.send(&ClientMessage::Detach)?;
            let _ = connection.receive::<ServerMessage>()?;
            Ok(())
        }
    }
}

#[cfg(not(unix))]
pub(crate) fn daemon_sessions() -> Result<()> {
    anyhow::bail!("daemon sessions currently require Unix local IPC")
}

#[cfg(not(unix))]
pub(crate) fn daemon_session_command(_command: SessionCliCommand) -> Result<()> {
    anyhow::bail!("daemon sessions currently require Unix local IPC")
}

#[cfg(unix)]
pub(crate) fn daemon_is_available() -> Result<yoctui_protocol::daemon_lifecycle::DaemonRuntimeRecord>
{
    use yoctui_protocol::{
        daemon::{
            Capability, ClientHello, ClientId, ClientMessage, ProtocolVersion, ServerMessage,
        },
        daemon_ipc::{DaemonConnection, runtime_paths},
        daemon_lifecycle::{
            RuntimeRecordState, classify_runtime_record, read_boot_id, read_runtime_record,
        },
    };
    let paths = runtime_paths()?;
    let record = read_runtime_record(&paths)?
        .context("Yoctui daemon is not running (runtime record is absent)")?;
    match classify_runtime_record(&record, &read_boot_id()?) {
        RuntimeRecordState::Current => {}
        RuntimeRecordState::Stale => anyhow::bail!("Yoctui daemon runtime record is stale"),
        RuntimeRecordState::ForeignProcess => {
            anyhow::bail!("Yoctui daemon PID belongs to another process")
        }
    }
    let mut connection = DaemonConnection::connect(&paths, Duration::from_secs(2))?;
    connection.set_timeout(Some(Duration::from_secs(10)))?;
    connection.send(&ClientMessage::Hello(ClientHello {
        minimum_version: ProtocolVersion::CURRENT,
        maximum_version: ProtocolVersion::CURRENT,
        client_id: ClientId([0; 16]),
        client_name: "yoctui-lifecycle".into(),
        capabilities: vec![Capability::GracefulShutdown],
    }))?;
    let ServerMessage::Hello(hello) = connection.receive()? else {
        anyhow::bail!("Yoctui daemon returned an unexpected lifecycle handshake")
    };
    if hello.daemon_instance_id != record.daemon_instance_id {
        anyhow::bail!("Yoctui daemon runtime record does not match the live instance")
    }
    Ok(record)
}

#[cfg(unix)]
pub(crate) fn stop_daemon() -> Result<()> {
    use yoctui_protocol::{
        daemon::{ClientMessage, CommandRequest, DaemonCommand, RequestId, ServerMessage},
        daemon_ipc::{DaemonConnection, runtime_paths},
    };
    let record = daemon_is_available()?;
    let paths = runtime_paths()?;
    let mut connection = DaemonConnection::connect(&paths, Duration::from_secs(1))?;
    connection.set_timeout(Some(Duration::from_secs(10)))?;
    connection.send(&ClientMessage::Command(CommandRequest {
        request_id: RequestId(1),
        expected_generation: None,
        command: DaemonCommand::PrepareShutdown,
    }))?;
    let response: ServerMessage = connection.receive()?;
    match response {
        ServerMessage::CommandResult(yoctui_protocol::daemon::CommandResult {
            outcome: yoctui_protocol::daemon::CommandOutcome::Completed,
            ..
        }) => {}
        response => anyhow::bail!("Yoctui daemon refused graceful shutdown: {response:?}"),
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    while paths.socket.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    if paths.socket.exists() {
        anyhow::bail!("Yoctui daemon did not stop within 15 seconds");
    }
    println!(
        "Yoctui daemon stopped (instance {})",
        format_instance(record.daemon_instance_id)
    );
    Ok(())
}
