//! Config query.
use super::*;

pub(crate) fn config_value_from_authorized_output(
    name: &str,
    implementation: &str,
    output: &str,
) -> Result<String> {
    let value = if implementation == yoctui_bitbake::BITBAKE_GETVAR_UTILITY_IMPLEMENTATION {
        output.trim().to_owned()
    } else {
        let prefix = format!("{name}=");
        let assignment = output
            .lines()
            .rev()
            .find_map(|line| line.strip_prefix(&prefix))
            .with_context(|| {
                format!("{name} is absent from the authorized BitBake environment dump")
            })?;
        assignment
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(assignment)
            .to_owned()
    };
    if value.is_empty() {
        anyhow::bail!("{name} is not available from the selected environment");
    }
    Ok(value)
}

pub(crate) async fn run_bounded_config_query(
    command: yoctui_bitbake::AuthorizedBitBakeCommand,
    build_dir: &Path,
) -> Result<String> {
    use tokio::io::AsyncReadExt;

    const OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
    async fn drain_bounded<R: tokio::io::AsyncRead + Unpin>(mut reader: R) -> io::Result<Vec<u8>> {
        let mut retained = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let count = reader.read(&mut buffer).await?;
            if count == 0 {
                return Ok(retained);
            }
            let available = OUTPUT_LIMIT.saturating_sub(retained.len());
            retained.extend_from_slice(&buffer[..count.min(available)]);
        }
    }

    let mut process = tokio::process::Command::new(&command.executable);
    process
        .args(&command.arguments)
        .current_dir(build_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = process.spawn().with_context(|| {
        format!(
            "could not start capability-authorized variable query {}",
            command.executable.display()
        )
    })?;
    let stdout = child
        .stdout
        .take()
        .context("variable query stdout unavailable")?;
    let stderr = child
        .stderr
        .take()
        .context("variable query stderr unavailable")?;
    let stdout_task = tokio::spawn(drain_bounded(stdout));
    let stderr_task = tokio::spawn(drain_bounded(stderr));
    let status = match tokio::time::timeout(Duration::from_secs(120), child.wait()).await {
        Ok(status) => status?,
        Err(_) => {
            let _ = child.kill().await;
            anyhow::bail!("capability-authorized variable query timed out after 120 seconds");
        }
    };
    let stdout = stdout_task
        .await
        .context("variable query stdout task failed")??;
    let stderr = stderr_task
        .await
        .context("variable query stderr task failed")??;
    if !status.success() {
        anyhow::bail!(
            "capability-authorized variable query exited with {status}: {}",
            String::from_utf8_lossy(&stderr).trim()
        );
    }
    if stdout.len() == OUTPUT_LIMIT || stderr.len() == OUTPUT_LIMIT {
        anyhow::bail!("capability-authorized variable query exceeded the 16 MiB output bound");
    }
    String::from_utf8(stdout)
        .context("capability-authorized variable query returned non-UTF-8 output")
}
