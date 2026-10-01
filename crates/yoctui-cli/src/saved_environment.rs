//! Saved paths are untrusted input; only fresh daemon authority becomes live.
use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use yoctui_model::{
    BuildEnvironmentProfile, SavedBuild, SavedEnvironmentMode, SavedEnvironmentPlan,
    SavedEnvironmentRequest,
};
use yoctui_protocol::daemon::{DaemonSnapshot, LifecycleState, ProtocolErrorCode};

pub(crate) enum Outcome {
    Prepared(SavedEnvironmentPlan),
    Loaded {
        profile: BuildEnvironmentProfile,
        runtime: Box<client_runtime::InteractiveDaemonRuntime>,
        client_id: [u8; 16],
    },
}
pub(crate) struct Operation {
    pub(crate) generation: u64,
    pub(crate) loading: bool,
    pub(crate) task: tokio::task::JoinHandle<Result<Outcome>>,
    cancelled: Arc<AtomicBool>,
}
impl Drop for Operation {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.task.abort();
    }
}

pub(crate) fn start(request: SavedEnvironmentRequest) -> Operation {
    let (generation, loading) = match &request {
        SavedEnvironmentRequest::Prepare { generation, .. } => (*generation, false),
        SavedEnvironmentRequest::Load { generation, .. } => (*generation, true),
    };
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = cancelled.clone();
    let task = tokio::spawn(async move {
        match request {
            SavedEnvironmentRequest::Prepare { record, .. } => {
                tokio::task::spawn_blocking(move || prepare(&record))
                    .await?
                    .map(Outcome::Prepared)
            }
            SavedEnvironmentRequest::Load { plan, .. } => load(plan, worker_cancelled).await,
        }
    });
    Operation {
        generation,
        loading,
        task,
        cancelled,
    }
}

fn saved_profile(record: &SavedBuild) -> Result<BuildEnvironmentProfile> {
    let build = record
        .build_dir
        .as_deref()
        .context("this saved build has no recorded build directory")?;
    anyhow::ensure!(
        Path::new(build).is_absolute(),
        "saved build directory must be absolute"
    );
    let build_dir = Path::new(build)
        .canonicalize()
        .with_context(|| format!("saved build directory is missing: {build}"))?;
    anyhow::ensure!(
        build_dir.is_dir()
            && build_dir.join("conf/local.conf").is_file()
            && build_dir.join("conf/bblayers.conf").is_file(),
        "saved build configuration is missing; restore its directory/configuration before loading"
    );
    let profile = if let Some(source) = &record.source {
        anyhow::ensure!(
            Path::new(source).is_absolute(),
            "saved source directory must be absolute"
        );
        let source_dir = Path::new(source)
            .canonicalize()
            .context("saved source directory is missing")?;
        let init_script = source_dir
            .join("oe-init-build-env")
            .canonicalize()
            .context("saved source has no oe-init-build-env script")?;
        BuildEnvironmentProfile {
            source_dir,
            build_dir,
            init_script,
        }
    } else {
        inferred_build_environment_profile(&build_dir)
            .context("saved source was not recorded and oe-init-build-env could not be located")?
    };
    Ok(BuildEnvironmentAdapter::default().validate(&profile)?)
}

fn revalidate_profile(profile: &BuildEnvironmentProfile) -> Result<()> {
    let record = SavedBuild {
        id: String::new(),
        target: String::new(),
        machine: None,
        source: Some(profile.source_dir.display().to_string()),
        build_dir: Some(profile.build_dir.display().to_string()),
        outcome: yoctui_model::SavedBuildOutcome::Incomplete,
        saved_unix_ms: 0,
        started_unix_ms: None,
        finished_unix_ms: None,
        logs: Vec::new(),
        tasks: Vec::new(),
        limitations: Vec::new(),
    };
    anyhow::ensure!(
        saved_profile(&record)? == *profile,
        "saved environment paths changed after review; refresh and retry"
    );
    Ok(())
}

fn current_snapshot() -> Result<Option<DaemonSnapshot>> {
    use yoctui_protocol::{
        daemon_ipc::{DaemonConnection, runtime_paths},
        daemon_lifecycle::{
            RuntimeRecordState, classify_runtime_record, read_boot_id, read_runtime_record,
        },
    };
    let paths = runtime_paths()?;
    let record = read_runtime_record(&paths)?;
    let boot_id = read_boot_id()?;
    match record
        .as_ref()
        .map(|record| classify_runtime_record(record, &boot_id))
    {
        None | Some(RuntimeRecordState::Stale) => {
            anyhow::ensure!(
                DaemonConnection::connect(&paths, Duration::from_millis(50)).is_err(),
                "a daemon is starting or has inconsistent runtime identity; retry when it is ready"
            );
            Ok(None)
        }
        Some(RuntimeRecordState::ForeignProcess) => {
            anyhow::bail!("daemon PID belongs to another process; refusing to replace it")
        }
        Some(RuntimeRecordState::Current) => {
            let (_, snapshot) = daemon_connection_with_snapshot()
                .context("live daemon is not responding; it will not be stopped")?;
            anyhow::ensure!(
                Some(snapshot.daemon_instance_id) == record.map(|record| record.daemon_instance_id),
                "daemon instance changed; refresh and retry"
            );
            Ok(Some(snapshot))
        }
    }
}

fn snapshot_build(snapshot: &DaemonSnapshot) -> Option<&str> {
    snapshot.compatibility.as_ref().and_then(|compatibility| {
        match &compatibility.environment.build_directory {
            yoctui_protocol::daemon::CompatibilityDetected::Detected { value, .. } => {
                Some(value.as_str())
            }
            yoctui_protocol::daemon::CompatibilityDetected::Unknown => None,
        }
    })
}

fn prepare(record: &SavedBuild) -> Result<SavedEnvironmentPlan> {
    let profile = saved_profile(record)?;
    let mode = reviewed_mode(&profile, current_snapshot()?)?;
    Ok(SavedEnvironmentPlan {
        profile,
        target: record.target.clone(),
        machine: record.machine.clone(),
        mode,
    })
}

fn reviewed_mode(
    profile: &BuildEnvironmentProfile,
    snapshot: Option<DaemonSnapshot>,
) -> Result<SavedEnvironmentMode> {
    Ok(match snapshot {
        None => SavedEnvironmentMode::Start,
        Some(snapshot) if snapshot_build(&snapshot) == profile.build_dir.to_str() => {
            SavedEnvironmentMode::Attach {
                instance: snapshot.daemon_instance_id.0,
            }
        }
        Some(snapshot) => {
            anyhow::ensure!(
                snapshot_build(&snapshot).is_some(),
                "daemon environment discovery is pending; wait and retry"
            );
            if let Some((_, error)) = replacement_denial(&snapshot, None) {
                anyhow::bail!(error);
            }
            SavedEnvironmentMode::Restart {
                instance: snapshot.daemon_instance_id.0,
            }
        }
    })
}

pub(crate) fn replacement_denial(
    snapshot: &DaemonSnapshot,
    generation: Option<u64>,
) -> Option<(ProtocolErrorCode, String)> {
    if generation.is_some_and(|generation| generation != snapshot.generation) {
        return Some((
            ProtocolErrorCode::StaleGeneration,
            "daemon changed after review; refresh and retry".into(),
        ));
    }
    let active = |lifecycle| {
        matches!(
            lifecycle,
            LifecycleState::Connecting | LifecycleState::Running | LifecycleState::Stopping
        )
    };
    let jobs = snapshot
        .jobs
        .iter()
        .filter(|job| active(job.lifecycle))
        .count();
    let terminals = snapshot
        .pty_sessions
        .iter()
        .filter(|pty| active(pty.lifecycle))
        .count();
    if jobs > 0 || terminals > 0 {
        Some((
            ProtocolErrorCode::LimitExceeded,
            format!(
                "daemon has {jobs} active job(s) and {terminals} active terminal(s); stop them explicitly before switching environments"
            ),
        ))
    } else {
        None
    }
}

fn check_review(plan: &SavedEnvironmentPlan) -> Result<()> {
    let snapshot = current_snapshot()?;
    match (&plan.mode, snapshot) {
        (SavedEnvironmentMode::Start, None) => Ok(()),
        (SavedEnvironmentMode::Attach { instance }, Some(snapshot))
            if snapshot.daemon_instance_id.0 == *instance
                && snapshot_build(&snapshot) == plan.profile.build_dir.to_str() =>
        {
            Ok(())
        }
        (SavedEnvironmentMode::Restart { instance }, Some(snapshot))
            if snapshot.daemon_instance_id.0 == *instance =>
        {
            if let Some((_, error)) = replacement_denial(&snapshot, None) {
                anyhow::bail!(error);
            }
            Ok(())
        }
        _ => anyhow::bail!("daemon changed after environment review; refresh and retry"),
    }
}

async fn load(mut plan: SavedEnvironmentPlan, cancelled: Arc<AtomicBool>) -> Result<Outcome> {
    let checked = plan.clone();
    tokio::task::spawn_blocking(move || {
        revalidate_profile(&checked.profile)?;
        check_review(&checked)
    })
    .await??;
    check_cancelled(&cancelled)?;
    if !matches!(plan.mode, SavedEnvironmentMode::Attach { .. }) {
        let initialized = BuildEnvironmentAdapter::default()
            .initialize(plan.profile.clone())
            .await?;
        anyhow::ensure!(
            initialized
                .environment
                .get("BUILDDIR")
                .map(PathBuf::from)
                .and_then(|path| path.canonicalize().ok())
                .as_ref()
                == Some(&plan.profile.build_dir),
            "init script selected a different BUILDDIR; refusing to load"
        );
        let checked = plan.clone();
        let checked_cancelled = cancelled.clone();
        tokio::task::spawn_blocking(move || {
            check_cancelled(&checked_cancelled)?;
            revalidate_profile(&checked.profile)?;
            check_review(&checked)?;
            if let SavedEnvironmentMode::Restart { instance } = checked.mode {
                stop_reviewed_daemon(instance)?;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await??;
        check_cancelled(&cancelled)?;
        let record = start_daemon_with_environment(Some(initialized.environment), false).await?;
        plan.mode = SavedEnvironmentMode::Attach {
            instance: record.daemon_instance_id.0,
        };
    }
    tokio::task::spawn_blocking(move || attach_loaded(plan, &cancelled)).await?
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<()> {
    anyhow::ensure!(
        !cancelled.load(Ordering::Relaxed),
        "saved environment loading cancelled by client exit"
    );
    Ok(())
}

fn stop_reviewed_daemon(instance: [u8; 16]) -> Result<()> {
    let (mut connection, snapshot) = daemon_connection_with_snapshot()?;
    request_reviewed_shutdown(&mut connection, &snapshot, instance)?;
    let paths = yoctui_protocol::daemon_ipc::runtime_paths()?;
    let deadline = Instant::now() + Duration::from_secs(15);
    while paths.socket.exists() {
        anyhow::ensure!(
            Instant::now() < deadline,
            "daemon did not stop within 15 seconds; no replacement started"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(())
}

fn request_reviewed_shutdown(
    connection: &mut yoctui_protocol::daemon_ipc::DaemonConnection,
    snapshot: &DaemonSnapshot,
    instance: [u8; 16],
) -> Result<()> {
    use yoctui_protocol::daemon::{
        ClientMessage, CommandOutcome, CommandRequest, DaemonCommand, RequestId, ServerMessage,
    };
    anyhow::ensure!(
        snapshot.daemon_instance_id.0 == instance,
        "daemon instance changed before shutdown; refusing replacement"
    );
    if let Some((_, error)) = replacement_denial(snapshot, None) {
        anyhow::bail!(error);
    }
    connection.send(&ClientMessage::Command(CommandRequest {
        request_id: RequestId(1),
        expected_generation: Some(snapshot.generation),
        command: DaemonCommand::PrepareShutdown,
    }))?;
    loop {
        match connection.receive::<ServerMessage>()? {
            ServerMessage::Event(_) => {}
            ServerMessage::CommandResult(result) => {
                anyhow::ensure!(
                    result.request_id == RequestId(1)
                        && result.outcome == CommandOutcome::Completed,
                    "daemon refused safe environment replacement: {:?}",
                    result.outcome
                );
                break;
            }
            response => anyhow::bail!("unexpected shutdown response: {response:?}"),
        }
    }
    Ok(())
}

fn attach_loaded(plan: SavedEnvironmentPlan, cancelled: &AtomicBool) -> Result<Outcome> {
    check_cancelled(cancelled)?;
    // Compatibility discovery has its own bounded 600-second probe window;
    // leave room for workspace inventory without blocking the input reactor.
    let discovery_timeout = Duration::from_secs(720);
    let deadline = Instant::now() + discovery_timeout;
    let mut app = App::new(32, 4096);
    let mut runtime = client_runtime::InteractiveDaemonRuntime::connect(
        &mut app,
        client_runtime::INITIAL_DAEMON_ATTACH_TIMEOUT,
    )
    .context("daemon is started, but initial attach failed; inspect daemon.log and retry")?;
    loop {
        check_cancelled(cancelled)?;
        runtime.poll(&mut app).context(
            "daemon disconnected during environment discovery; inspect daemon.log and retry",
        )?;
        let snapshot = runtime.snapshot();
        let SavedEnvironmentMode::Attach { instance } = plan.mode else {
            unreachable!("startup records the new daemon instance")
        };
        anyhow::ensure!(
            snapshot.daemon_instance_id.0 == instance,
            "daemon changed during environment loading; refresh and retry"
        );
        if snapshot_build(snapshot) == plan.profile.build_dir.to_str()
            && snapshot.workspace.as_ref().is_some_and(|workspace| {
                workspace.canonical_build == plan.profile.build_dir.to_string_lossy()
            })
        {
            return Ok(Outcome::Loaded {
                profile: plan.profile,
                runtime: Box::new(runtime),
                client_id: app
                    .terminal
                    .client_id
                    .context("attached client has no identity")?,
            });
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "daemon started, but fresh environment discovery did not finish within {} seconds; inspect daemon.log and retry loading",
            discovery_timeout.as_secs()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
#[path = "tests/saved_environment.rs"]
mod tests;
