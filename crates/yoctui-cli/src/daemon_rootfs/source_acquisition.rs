use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::sync::oneshot;
use yoctui_bitbake::BitBakeBackend;
use yoctui_model::DaemonCompatibilitySnapshot;
use yoctui_protocol::rootfs::{RootfsSourcesData, RootfsSourcesRequestData};

pub(super) async fn acquire(
    query: RootfsSourcesRequestData,
    build: PathBuf,
    compatibility: DaemonCompatibilitySnapshot,
    environment: BTreeMap<String, String>,
    mut cancelled: oneshot::Receiver<()>,
    query_timeout: Duration,
) -> Result<RootfsSourcesData> {
    if compatibility
        .implementations
        .get(&yoctui_model::CapabilityId::BitBakeGetVar)
        .is_some_and(|selected| {
            matches!(
                selected.id.as_str(),
                yoctui_bitbake::BITBAKE_GETVAR_UTILITY_IMPLEMENTATION
                    | yoctui_bitbake::BITBAKE_GETVAR_ENVIRONMENT_IMPLEMENTATION
            )
        })
    {
        let planner = yoctui_bitbake::BitBakeCommandPlanner::new(
            &compatibility,
            query.compatibility_generation,
            &build,
        )?;
        let deadline = tokio::time::Instant::now() + query_timeout;
        let mut values = Vec::with_capacity(3);
        for name in ["IMAGE_MANIFEST", "PKGDATA_DIR", "IMAGE_ROOTFS"] {
            let command = planner.get_variable(name, Some(&query.request.image.image))?;
            let utility =
                command.implementation == yoctui_bitbake::BITBAKE_GETVAR_UTILITY_IMPLEMENTATION;
            let output =
                run_source_command(command, &build, &environment, &mut cancelled, deadline).await?;
            let value = if utility {
                Some(output.trim())
            } else {
                let prefix = format!("{name}=");
                output
                    .lines()
                    .rev()
                    .find_map(|line| line.strip_prefix(&prefix))
                    .map(|value| {
                        value
                            .strip_prefix('"')
                            .and_then(|v| v.strip_suffix('"'))
                            .unwrap_or(value)
                    })
            };
            values.push(value.filter(|v| !v.is_empty()).map(str::to_owned));
        }
        let sources = RootfsSourcesData {
            query,
            image_manifest: values[0].take(),
            pkgdata_dir: values[1].take(),
            image_rootfs: values[2].take(),
        };
        sources.validate()?;
        return Ok(sources);
    }
    let python = environment
        .get("PYTHON")
        .map(String::as_str)
        .unwrap_or("python3");
    let startup = crate::spawn_configured_bridge_with_compatibility(
        python,
        build,
        Some(environment.clone()),
        compatibility,
    );
    let mut backend = tokio::select! {
        biased;
        _ = &mut cancelled => anyhow::bail!("rootfs metadata query cancelled"),
        result = tokio::time::timeout(Duration::from_secs(30), startup) => result.context("rootfs bridge handshake timed out")??,
    };
    let result = tokio::select! {
        biased;
        _ = &mut cancelled => Err(anyhow::anyhow!("rootfs metadata query cancelled")),
        result = tokio::time::timeout(query_timeout, async {
            let recipe = Some(query.request.image.image.clone());
            let image_manifest = backend.get_variable("IMAGE_MANIFEST".into(), recipe.clone()).await?.value;
            let pkgdata_dir = backend.get_variable("PKGDATA_DIR".into(), recipe.clone()).await?.value;
            let image_rootfs = backend.get_variable("IMAGE_ROOTFS".into(), recipe).await?.value;
            let sources = RootfsSourcesData { query, image_manifest, pkgdata_dir, image_rootfs };
            sources.validate()?;
            Ok::<_, anyhow::Error>(sources)
        }) => result.context("rootfs metadata query timed out").and_then(|result| result),
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

async fn run_source_command(
    command: yoctui_bitbake::AuthorizedBitBakeCommand,
    build: &Path,
    environment: &BTreeMap<String, String>,
    cancelled: &mut oneshot::Receiver<()>,
    deadline: tokio::time::Instant,
) -> Result<String> {
    use std::process::Stdio;
    use tokio::io::AsyncReadExt;
    async fn bounded(mut reader: impl tokio::io::AsyncRead + Unpin) -> Result<Vec<u8>> {
        const LIMIT: usize = 16 * 1024 * 1024;
        let mut bytes = Vec::new();
        let mut buffer = [0; 8192];
        loop {
            let count = reader.read(&mut buffer).await?;
            if count == 0 {
                return Ok(bytes);
            }
            ensure!(
                bytes.len() + count <= LIMIT,
                "rootfs metadata query exceeded the 16 MiB output bound"
            );
            bytes.extend_from_slice(&buffer[..count]);
        }
    }
    ensure!(
        tokio::time::Instant::now() < deadline,
        "rootfs metadata query timed out"
    );
    ensure!(
        matches!(
            cancelled.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ),
        "rootfs metadata query cancelled"
    );
    let mut child = tokio::process::Command::new(command.executable)
        .args(command.arguments)
        .current_dir(build)
        .env_clear()
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .context("could not start authorized rootfs variable query")?;
    let group = child
        .id()
        .context("rootfs query child identity unavailable")? as i32;
    let stdout = child.stdout.take().context("rootfs stdout unavailable")?;
    let stderr = child.stderr.take().context("rootfs stderr unavailable")?;
    let result = tokio::select! {
        biased;
        _ = cancelled => Err(anyhow::anyhow!("rootfs metadata query cancelled")),
        _ = tokio::time::sleep_until(deadline) => Err(anyhow::anyhow!("rootfs metadata query timed out")),
        result = async { tokio::try_join!(bounded(stdout), bounded(stderr), async { Ok::<_, anyhow::Error>(child.wait().await?) }) } => result,
    };
    let (stdout, stderr, status) = match result {
        Ok(output) => output,
        Err(error) => {
            // This process group belongs exclusively to this read-only query.
            let _ = unsafe { libc::kill(-group, libc::SIGINT) };
            if tokio::time::timeout(Duration::from_secs(5), child.wait())
                .await
                .is_err()
            {
                let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            return Err(error);
        }
    };
    ensure!(
        status.success(),
        "rootfs variable query exited with {status}: {}",
        String::from_utf8_lossy(&stderr[..stderr.len().min(4096)]).trim()
    );
    String::from_utf8(stdout).context("rootfs variable query returned non-UTF-8 output")
}
