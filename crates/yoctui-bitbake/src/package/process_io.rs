async fn canonical_directory(path: &Path) -> Result<PathBuf, PackageDataAdapterError> {
    if !path.is_absolute() {
        return Err(PackageDataAdapterError::PathEscape(path.to_owned()));
    }
    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|_| PackageDataAdapterError::InvalidPath(path.to_owned()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PackageDataAdapterError::InvalidPath(path.to_owned()));
    }
    tokio::fs::canonicalize(path)
        .await
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))
}

async fn canonical_regular_file(path: &Path) -> Result<PathBuf, PackageDataAdapterError> {
    if !path.is_absolute() {
        return Err(PackageDataAdapterError::PathEscape(path.to_owned()));
    }
    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|_| PackageDataAdapterError::MissingTool(path.to_owned()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(PackageDataAdapterError::InvalidPath(path.to_owned()));
    }
    tokio::fs::canonicalize(path)
        .await
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))
}

struct BoundedOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

struct CommandOutput {
    stdout: Vec<u8>,
    truncated: bool,
}

async fn read_bounded<R>(mut reader: R) -> Result<BoundedOutput, std::io::Error>
where
    R: AsyncRead + Unpin,
{
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 16 * 1024];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let remaining = MAX_PACKAGE_OUTPUT_BYTES.saturating_sub(bytes.len());
        bytes.extend_from_slice(&buffer[..read.min(remaining)]);
        truncated |= read > remaining;
    }
    Ok(BoundedOutput { bytes, truncated })
}

async fn run_package_command(
    spec: PackageDataCommandSpec,
    build_dir: &Path,
    timeout: Duration,
    cancellation: &PackageDataCancellation,
) -> Result<CommandOutput, PackageDataAdapterError> {
    if cancellation.is_cancelled() {
        return Err(PackageDataAdapterError::Cancelled);
    }
    let mut command = Command::new(&spec.executable);
    command
        .args(&spec.arguments)
        .current_dir(build_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            PackageDataAdapterError::MissingTool(spec.executable.clone())
        } else {
            PackageDataAdapterError::Spawn(error.to_string())
        }
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| PackageDataAdapterError::Spawn("stdout is unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| PackageDataAdapterError::Spawn("stderr is unavailable".into()))?;
    let stdout_task = tokio::spawn(read_bounded(stdout));
    let stderr_task = tokio::spawn(read_bounded(stderr));
    let terminal = tokio::select! {
        status = child.wait() => status.map_err(|error| PackageDataAdapterError::Io(error.to_string())),
        _ = cancellation.cancelled() => {
            terminate_package_child(&mut child).await;
            Err(PackageDataAdapterError::Cancelled)
        }
        _ = tokio::time::sleep(timeout) => {
            terminate_package_child(&mut child).await;
            Err(PackageDataAdapterError::Timeout(timeout.as_secs()))
        }
    };
    let stdout = stdout_task
        .await
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))?
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))?;
    let stderr = stderr_task
        .await
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))?
        .map_err(|error| PackageDataAdapterError::Io(error.to_string()))?;
    let status = terminal?;
    if !status.success() {
        return Err(PackageDataAdapterError::NonZero {
            exit_code: status.code(),
            message: bounded_error_message(&stderr.bytes, &stdout.bytes),
        });
    }
    Ok(CommandOutput {
        stdout: stdout.bytes,
        truncated: stdout.truncated || stderr.truncated,
    })
}

async fn terminate_package_child(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    if let Some(id) = child.id() {
        // SAFETY: the process group is the child PID created by `process_group(0)`.
        let _ = unsafe { libc::kill(-(id as i32), libc::SIGTERM) };
        if tokio::time::timeout(Duration::from_millis(500), child.wait())
            .await
            .is_ok()
        {
            return;
        }
        // SAFETY: same child-owned process group as the graceful signal above.
        let _ = unsafe { libc::kill(-(id as i32), libc::SIGKILL) };
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

fn bounded_error_message(stderr: &[u8], stdout: &[u8]) -> String {
    let bytes = if stderr.is_empty() { stdout } else { stderr };
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .next()
        .unwrap_or("no diagnostic output")
        .chars()
        .take(512)
        .collect()
}

fn push_limitation(limitations: &mut Vec<String>, limitation: String) {
    yoctui_utils::push_unique_bounded(limitations, limitation, MAX_PACKAGE_LIMITATIONS);
}
