//! Daemon startup.
use super::*;

#[cfg(unix)]
pub(crate) async fn inspect_daemon_startup_workspace(
    startup_environment: &BTreeMap<String, String>,
    compatibility: Option<yoctui_model::DaemonCompatibilitySnapshot>,
    mut cancelled: tokio::sync::oneshot::Receiver<()>,
) -> Result<Option<yoctui_model::Workspace>> {
    let Some(compatibility) = compatibility else {
        return Ok(None);
    };
    let Some(build_dir) = compatibility
        .snapshot
        .environment
        .build_directory
        .value()
        .cloned()
    else {
        return Ok(None);
    };
    let python = startup_environment
        .get("PYTHON")
        .map(String::as_str)
        .unwrap_or("python3");
    let startup = spawn_configured_bridge_with_compatibility_at_priority(
        python,
        build_dir,
        Some(startup_environment.clone()),
        compatibility,
        BridgeProcessPriority::Background,
    );
    let mut backend = tokio::select! {
        biased;
        _ = &mut cancelled => return Err(anyhow::anyhow!("startup metadata scan cancelled")),
        result = tokio::time::timeout(Duration::from_secs(30), startup) =>
            result.context("startup metadata bridge handshake timed out")?
                .context("could not start the daemon-owned startup metadata bridge")?,
    };
    let result = tokio::select! {
        biased;
        _ = &mut cancelled => Err(anyhow::anyhow!("startup metadata scan cancelled")),
        result = tokio::time::timeout(Duration::from_secs(600), async {
            let mut workspace = backend.inspect_workspace().await?;
            workspace.recipes = backend.list_recipes(None).await?;
            workspace.layers = backend.list_layers().await?;
            Ok::<_, anyhow::Error>(Some(workspace))
        }) => result.context("startup metadata scan exceeded ten minutes").and_then(|result| result),
    };
    if result.is_err()
        || !matches!(
            tokio::time::timeout(Duration::from_secs(5), backend.shutdown()).await,
            Ok(Ok(()))
        )
    {
        backend.interrupt_metadata().await;
    }
    result
}
