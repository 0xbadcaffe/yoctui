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
        let remaining = MAX_SIGNATURE_OUTPUT_BYTES.saturating_sub(bytes.len());
        bytes.extend_from_slice(&buffer[..read.min(remaining)]);
        truncated |= read > remaining;
    }
    Ok(BoundedOutput { bytes, truncated })
}

async fn run_signature_command(
    spec: SignatureCommandSpec,
    build_dir: &Path,
    timeout: Duration,
    cancellation: &SignatureCancellation,
) -> Result<Vec<u8>, SignatureAdapterError> {
    if cancellation.is_cancelled() {
        return Err(SignatureAdapterError::Cancelled);
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
            SignatureAdapterError::MissingTool(spec.executable.clone())
        } else {
            SignatureAdapterError::Spawn(error.to_string())
        }
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| SignatureAdapterError::Spawn("stdout is unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| SignatureAdapterError::Spawn("stderr is unavailable".into()))?;
    let stdout_task = tokio::spawn(read_bounded(stdout));
    let stderr_task = tokio::spawn(read_bounded(stderr));
    let terminal = tokio::select! {
        status = child.wait() => status.map_err(|error| SignatureAdapterError::Io(error.to_string())),
        _ = cancellation.cancelled() => {
            terminate_signature_child(&mut child).await;
            Err(SignatureAdapterError::Cancelled)
        }
        _ = tokio::time::sleep(timeout) => {
            terminate_signature_child(&mut child).await;
            Err(SignatureAdapterError::Timeout(timeout.as_secs()))
        }
    };
    let stdout = stdout_task
        .await
        .map_err(|error| SignatureAdapterError::Io(error.to_string()))?
        .map_err(|error| SignatureAdapterError::Io(error.to_string()))?;
    let stderr = stderr_task
        .await
        .map_err(|error| SignatureAdapterError::Io(error.to_string()))?
        .map_err(|error| SignatureAdapterError::Io(error.to_string()))?;
    let status = terminal?;
    if stdout.truncated || stderr.truncated {
        return Err(SignatureAdapterError::OutputLimit(
            MAX_SIGNATURE_OUTPUT_BYTES,
        ));
    }
    if !status.success() {
        return Err(SignatureAdapterError::NonZero {
            exit_code: status.code(),
            message: bounded_error_message(&stderr.bytes, &stdout.bytes),
        });
    }
    Ok(stdout.bytes)
}

async fn terminate_signature_child(child: &mut tokio::process::Child) {
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
    let message = text.lines().next().unwrap_or("no diagnostic output");
    message.chars().take(512).collect()
}
