//! Backend startup.
use super::*;

pub(crate) async fn select_backend(
    backend: Backend,
    build_dir: PathBuf,
) -> Result<Box<dyn BitBakeBackend>> {
    select_backend_with_timeout(backend, build_dir, None).await
}

pub(crate) async fn select_backend_with_timeout(
    backend: Backend,
    build_dir: PathBuf,
    cancellation_timeout: Option<Duration>,
) -> Result<Box<dyn BitBakeBackend>> {
    select_backend_with_environment(backend, build_dir, cancellation_timeout, None).await
}

pub(crate) async fn select_backend_with_environment(
    backend: Backend,
    build_dir: PathBuf,
    cancellation_timeout: Option<Duration>,
    environment: Option<BTreeMap<String, String>>,
) -> Result<Box<dyn BitBakeBackend>> {
    let compatibility = current_daemon_compatibility(&build_dir)?;
    match backend {
        Backend::Process => {
            let backend = ProcessBackend::new(build_dir).with_compatibility(compatibility)?;
            let backend = if let Some(environment) = environment {
                backend.with_environment(environment)
            } else {
                backend
            };
            let backend = if let Some(timeout) = cancellation_timeout {
                backend.with_cancellation_timeout(timeout)
            } else {
                backend
            };
            Ok(Box::new(backend))
        }
        Backend::Bridge => {
            let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
            let bridge = spawn_configured_bridge_with_compatibility(
                &python,
                build_dir,
                environment,
                compatibility,
            )
            .await;
            bridge
                .map(|backend| Box::new(backend) as Box<dyn BitBakeBackend>)
                .context("could not start the capability-authorized BitBake bridge; start Yoctui's daemon from the initialized build environment")
        }
    }
}

#[cfg(unix)]
pub(crate) fn current_daemon_compatibility(
    build_dir: &std::path::Path,
) -> Result<yoctui_model::DaemonCompatibilitySnapshot> {
    use yoctui_protocol::daemon::ClientMessage;

    let (mut connection, snapshot) = daemon_connection_with_snapshot().context(
        "BitBake operations require the daemon-owned compatibility snapshot; start Yoctui's daemon from the initialized build environment",
    )?;
    let _ = connection.send(&ClientMessage::Detach);
    let wire = snapshot.compatibility.context(
        "the running daemon has no current initialized-environment compatibility authority",
    )?;
    let compatibility = yoctui_app::compatibility_model_snapshot(&wire)
        .map_err(anyhow::Error::msg)
        .context("the daemon compatibility snapshot is invalid")?;
    if compatibility
        .snapshot
        .environment
        .build_directory
        .value()
        .map(std::path::PathBuf::as_path)
        != Some(build_dir)
    {
        anyhow::bail!(
            "selected build directory {} does not match the daemon compatibility authority",
            build_dir.display()
        );
    }
    Ok(compatibility)
}

#[cfg(not(unix))]
pub(crate) fn current_daemon_compatibility(
    _build_dir: &std::path::Path,
) -> Result<yoctui_model::DaemonCompatibilitySnapshot> {
    anyhow::bail!("daemon-owned compatibility authority currently requires Unix local IPC")
}

pub(crate) async fn spawn_configured_bridge_with_compatibility(
    python: &str,
    build_dir: PathBuf,
    environment: Option<BTreeMap<String, String>>,
    compatibility: yoctui_model::DaemonCompatibilitySnapshot,
) -> Result<BridgeBackend, yoctui_bitbake::BackendError> {
    spawn_configured_bridge_with_compatibility_at_priority(
        python,
        build_dir,
        environment,
        compatibility,
        BridgeProcessPriority::Inherited,
    )
    .await
}

pub(crate) async fn spawn_configured_bridge_with_compatibility_at_priority(
    python: &str,
    build_dir: PathBuf,
    environment: Option<BTreeMap<String, String>>,
    compatibility: yoctui_model::DaemonCompatibilitySnapshot,
    priority: BridgeProcessPriority,
) -> Result<BridgeBackend, yoctui_bitbake::BackendError> {
    let environment = environment.unwrap_or_default();
    let generation = compatibility.snapshot.generation;
    if let Some(script) = bridge_path_override(env::var_os("YOCTUI_BRIDGE_PATH")) {
        BridgeBackend::spawn_with_compatibility_at_priority(
            python,
            script,
            build_dir,
            environment,
            compatibility,
            generation,
            priority,
        )
        .await
    } else {
        BridgeBackend::spawn_bundled_with_compatibility_at_priority(
            python,
            build_dir,
            environment,
            compatibility,
            generation,
            priority,
        )
        .await
    }
}

pub(crate) async fn spawn_configured_bridge(
    python: &str,
    build_dir: PathBuf,
    environment: Option<BTreeMap<String, String>>,
) -> Result<BridgeBackend, yoctui_bitbake::BackendError> {
    let environment = environment.unwrap_or_default();
    if let Some(script) = bridge_path_override(env::var_os("YOCTUI_BRIDGE_PATH")) {
        BridgeBackend::spawn_with_environment(python, script, build_dir, environment).await
    } else {
        BridgeBackend::spawn_bundled_with_environment(python, build_dir, environment).await
    }
}

pub(crate) fn bridge_path_override(value: Option<std::ffi::OsString>) -> Option<PathBuf> {
    value.map(PathBuf::from)
}
