enum ProbeProcessResult {
    Completed {
        success: bool,
        output: String,
        stdout: String,
        truncated: bool,
    },
    TimedOut,
    Failed(String),
}

/// Read-only, bounded discovery for the bundled backend, not release/version inference.
pub async fn probe_bundled_backend_capabilities(
    python: &Path,
    build_directory: &Path,
    environment: &BTreeMap<String, String>,
    bitbake_version: &str,
) -> Result<BTreeSet<String>, String> {
    // A custom bridge may implement different operations from the bundled one.
    if environment.contains_key("YOCTUI_BRIDGE_PATH") {
        return Err("custom bridge does not declare a startup capability probe".into());
    }
    let arguments = [
        "-c".into(),
        crate::BUNDLED_BRIDGE_SOURCE.into(),
        "--probe-capabilities".into(),
    ];
    match run_read_only(
        python,
        &arguments,
        build_directory,
        environment,
        Duration::from_secs(30),
        DEFAULT_PROBE_OUTPUT_LIMIT,
        true,
    )
    .await
    {
        ProbeProcessResult::Completed {
            success: true,
            stdout,
            truncated: false,
            ..
        } => parse_backend_capabilities(&stdout, build_directory, bitbake_version),
        ProbeProcessResult::Completed { .. } => {
            Err("backend capability probe failed or exceeded its output bound".into())
        }
        ProbeProcessResult::TimedOut => Err("backend capability probe timed out".into()),
        ProbeProcessResult::Failed(error) => Err(error),
    }
}

fn parse_backend_capabilities(
    output: &str,
    build_directory: &Path,
    bitbake_version: &str,
) -> Result<BTreeSet<String>, String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Report {
        schema: String,
        build_directory: PathBuf,
        bitbake_version: String,
        capabilities: Vec<String>,
    }
    let report: Report = serde_json::from_str(output)
        .map_err(|_| "backend capability probe returned an invalid report".to_owned())?;
    if report.schema != "yoctui.bridge-capability-probe.v1"
        || report.build_directory != build_directory
        || !report.build_directory.is_absolute()
        || report.bitbake_version != bitbake_version
        || report.capabilities.len() > 64
    {
        return Err("backend capability probe identity or bounds mismatch".into());
    }
    let known = yoctui_model::CapabilityCatalog::builtin()
        .entries
        .into_iter()
        .flat_map(|entry| entry.probes)
        .filter_map(|probe| match probe {
            CapabilityProbeSpec::BackendCapability { name } => Some(name),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let capabilities = report.capabilities.iter().cloned().collect::<BTreeSet<_>>();
    if capabilities.len() != report.capabilities.len() || !capabilities.is_subset(&known) {
        return Err("backend capability probe contains duplicate or unknown tokens".into());
    }
    Ok(capabilities)
}

async fn run_read_only(
    executable: &Path,
    arguments: &[String],
    cwd: &Path,
    environment: &BTreeMap<String, String>,
    timeout: Duration,
    output_limit: usize,
    background_priority: bool,
) -> ProbeProcessResult {
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .current_dir(cwd)
        .env_clear()
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = match spawn_probe_process(&mut command).await {
        Ok(child) => child,
        Err(error) => return ProbeProcessResult::Failed(error.to_string()),
    };
    if background_priority
        && let Some(pid) = child.id()
        && let Err(error) = yoctui_utils::lower_process_priority(pid, 10)
    {
        tracing::warn!(pid, %error, "could not lower capability probe priority");
    }
    let process_group = child.id().map(|id| id as i32);
    let mut group_guard = child.id().map(yoctui_utils::ProcessGroupGuard::new);
    let Some(stdout) = child.stdout.take() else {
        return ProbeProcessResult::Failed("stdout pipe is unavailable".into());
    };
    let Some(stderr) = child.stderr.take() else {
        return ProbeProcessResult::Failed("stderr pipe is unavailable".into());
    };
    let read = async {
        let mut stdout_bytes = Vec::new();
        let mut stderr_bytes = Vec::new();
        let mut bounded_stdout = stdout.take(output_limit as u64 + 1);
        let mut bounded_stderr = stderr.take(output_limit as u64 + 1);
        let stdout_read = bounded_stdout.read_to_end(&mut stdout_bytes);
        let stderr_read = bounded_stderr.read_to_end(&mut stderr_bytes);
        let (stdout_result, stderr_result, status) =
            tokio::join!(stdout_read, stderr_read, child.wait());
        stdout_result.map_err(|error| error.to_string())?;
        stderr_result.map_err(|error| error.to_string())?;
        let status = status.map_err(|error| error.to_string())?;
        let truncated = stdout_bytes.len() > output_limit || stderr_bytes.len() > output_limit;
        stdout_bytes.truncate(output_limit);
        stderr_bytes.truncate(output_limit);
        let output = format!(
            "{}\n{}",
            String::from_utf8_lossy(&stdout_bytes),
            String::from_utf8_lossy(&stderr_bytes)
        );
        Ok::<_, String>((
            status.success(),
            output,
            String::from_utf8_lossy(&stdout_bytes).into_owned(),
            truncated,
        ))
    };
    match tokio::time::timeout(timeout, read).await {
        Ok(Ok((success, output, stdout, truncated))) => {
            if let Some(guard) = &mut group_guard {
                guard.disarm();
            }
            ProbeProcessResult::Completed {
                success,
                output,
                stdout,
                truncated,
            }
        }
        Ok(Err(message)) => ProbeProcessResult::Failed(message),
        Err(_) => {
            #[cfg(unix)]
            if let Some(group) = process_group {
                // SAFETY: the child was placed in a new process group owned by this probe.
                let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
            }
            let _ = child.kill().await;
            let _ = child.wait().await;
            ProbeProcessResult::TimedOut
        }
    }
}
