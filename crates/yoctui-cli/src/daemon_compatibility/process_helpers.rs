use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
};

use tokio::{io::AsyncReadExt, process::Command};

use super::{
    DaemonCompatibilityError, STARTUP_ENVIRONMENT_LIMIT, STARTUP_ENVIRONMENT_VALUE_LIMIT,
    STARTUP_FINGERPRINT_LIMIT, STARTUP_QUERY_OUTPUT_LIMIT, STARTUP_QUERY_TIMEOUT,
};

/// BitBake may write reconnect/status diagnostics to the same captured stream
/// as a successful `bitbake-getvar` value. Keep only the final meaningful line
/// so diagnostics cannot corrupt authoritative identity fields.
pub(super) fn authoritative_value(output: &str) -> Option<String> {
    output
        .lines()
        .map(strip_terminal_escapes)
        .map(|line| line.trim().trim_matches('"').trim_matches('\'').to_owned())
        .filter(|line| !line.is_empty())
        .rfind(|line| !line.starts_with("NOTE:") && !line.starts_with("WARNING:"))
}

pub(super) fn strip_terminal_escapes(value: &str) -> String {
    yoctui_utils::strip_ansi(value)
        .chars()
        .filter(|character| !character.is_control())
        .collect()
}

pub(super) fn valid_identity_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'.' | b'-'))
}

pub(super) fn authoritative_token(value: &str) -> Option<String> {
    value
        .lines()
        .map(strip_terminal_escapes)
        .map(|line| line.trim().trim_matches('"').trim_matches('\'').to_owned())
        .rfind(|line| valid_identity_token(line))
}

pub(super) fn canonical_initialized_build(
    value: &str,
) -> Result<PathBuf, DaemonCompatibilityError> {
    let configured = Path::new(value);
    let build = fs::canonicalize(configured).map_err(|error| {
        DaemonCompatibilityError::InvalidStartupEnvironment(format!(
            "BUILDDIR {} cannot be resolved: {error}",
            configured.display()
        ))
    })?;
    if !build.is_dir()
        || build == Path::new("/")
        || !build.join("conf/local.conf").is_file()
        || !build.join("conf/bblayers.conf").is_file()
    {
        return Err(DaemonCompatibilityError::InvalidStartupEnvironment(
            format!(
                "BUILDDIR {} is not an initialized Yocto build",
                build.display()
            ),
        ));
    }
    Ok(build)
}

pub(super) fn discover_executable(path: &str, name: &str) -> Option<PathBuf> {
    std::env::split_paths(path).find_map(|directory| {
        if !directory.is_absolute() {
            return None;
        }
        let directory = fs::canonicalize(directory).ok()?;
        // Keep the requested basename: sibling aliases can select behavior by argv[0].
        let candidate = directory.join(name);
        let metadata = fs::symlink_metadata(&candidate).ok()?;
        let executable_metadata = if metadata.file_type().is_symlink() {
            let target = fs::read_link(&candidate).ok()?;
            if target.is_absolute()
                || target
                    .components()
                    .any(|component| !matches!(component, std::path::Component::Normal(_)))
            {
                return None;
            }
            let canonical_target = fs::canonicalize(directory.join(target)).ok()?;
            if canonical_target.parent()? != directory {
                return None;
            }
            fs::metadata(canonical_target).ok()?
        } else {
            metadata
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            (executable_metadata.is_file() && executable_metadata.permissions().mode() & 0o111 != 0)
                .then_some(candidate)
        }
        #[cfg(not(unix))]
        {
            executable_metadata.is_file().then_some(candidate)
        }
    })
}

pub(super) async fn run_read_only(
    executable: &Path,
    arguments: &[&str],
    build_directory: &Path,
    environment: &BTreeMap<String, String>,
) -> Result<String, DaemonCompatibilityError> {
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .current_dir(build_directory)
        .env_clear()
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(|error| {
        DaemonCompatibilityError::StartupProbe(format!(
            "could not start {}: {error}",
            executable.display()
        ))
    })?;
    if let Some(pid) = child.id()
        && let Err(error) = yoctui_utils::lower_process_priority(pid, 10)
    {
        tracing::warn!(pid, %error, "could not lower startup compatibility query priority");
    }
    let mut group_guard = child.id().map(yoctui_utils::ProcessGroupGuard::new);
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| DaemonCompatibilityError::StartupProbe("stdout unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| DaemonCompatibilityError::StartupProbe("stderr unavailable".into()))?;
    let read = async {
        let (stdout, stderr, status) = tokio::join!(
            read_bounded_stream(stdout),
            read_bounded_stream(stderr),
            child.wait()
        );
        Ok::<_, DaemonCompatibilityError>((
            stdout?,
            stderr?,
            status.map_err(|error| {
                DaemonCompatibilityError::StartupProbe(format!(
                    "could not wait for {}: {error}",
                    executable.display()
                ))
            })?,
        ))
    };
    let ((stdout, stdout_truncated), (stderr, stderr_truncated), status) =
        match tokio::time::timeout(STARTUP_QUERY_TIMEOUT, read).await {
            Ok(result) => result?,
            Err(_) => {
                drop(group_guard.take());
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(DaemonCompatibilityError::StartupProbe(format!(
                    "read-only query timed out for {}",
                    executable.display()
                )));
            }
        };
    if let Some(guard) = &mut group_guard {
        guard.disarm();
    }
    if stdout_truncated || stderr_truncated {
        return Err(DaemonCompatibilityError::StartupProbe(format!(
            "read-only query output exceeded {} bytes per stream",
            STARTUP_QUERY_OUTPUT_LIMIT
        )));
    }
    if !status.success() {
        return Err(DaemonCompatibilityError::StartupProbe(format!(
            "read-only query failed for {}: {}",
            executable.display(),
            String::from_utf8_lossy(&stderr).trim()
        )));
    }
    let mut output = String::from_utf8_lossy(&stdout).into_owned();
    if output.trim().is_empty() {
        output = String::from_utf8_lossy(&stderr).into_owned();
    }
    Ok(output)
}

async fn read_bounded_stream<R>(mut stream: R) -> Result<(Vec<u8>, bool), DaemonCompatibilityError>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut retained = Vec::new();
    let mut buffer = [0_u8; 8 * 1024];
    let mut truncated = false;
    loop {
        let count = stream.read(&mut buffer).await.map_err(|error| {
            DaemonCompatibilityError::StartupProbe(format!("could not read query output: {error}"))
        })?;
        if count == 0 {
            break;
        }
        let remaining = STARTUP_QUERY_OUTPUT_LIMIT.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..count.min(remaining)]);
        truncated |= count > remaining;
    }
    Ok((retained, truncated))
}

pub(super) fn parse_bitbake_version(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let line = line.trim();
        line.contains("BitBake")
            .then(|| line.split_whitespace().next_back())
            .flatten()
            .filter(|value| {
                value
                    .chars()
                    .next()
                    .is_some_and(|value| value.is_ascii_digit())
                    && value.chars().all(|value| {
                        value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '+')
                    })
            })
            .map(str::to_owned)
    })
}

pub(super) fn bounded_process_environment(
    environment: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let valid = |key: &str, value: &str| {
        !key.is_empty()
            && !value.is_empty()
            && key.len() <= STARTUP_ENVIRONMENT_VALUE_LIMIT
            && value.len() <= STARTUP_ENVIRONMENT_VALUE_LIMIT
            && !key.contains('\0')
            && !value.contains('\0')
    };
    let mut bounded = BTreeMap::new();
    for key in ["BUILDDIR", "PATH", "BBPATH", "PYTHONPATH", "HOME"] {
        if let Some(value) = environment.get(key)
            && valid(key, value)
        {
            bounded.insert(key.to_owned(), value.clone());
        }
    }
    for (key, value) in environment {
        if bounded.len() >= STARTUP_ENVIRONMENT_LIMIT {
            break;
        }
        if valid(key, value) {
            bounded.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
    bounded
}

pub(super) fn read_bounded(path: &Path) -> Result<Vec<u8>, DaemonCompatibilityError> {
    let metadata = fs::metadata(path).map_err(|error| {
        DaemonCompatibilityError::InvalidStartupEnvironment(format!(
            "could not inspect {}: {error}",
            path.display()
        ))
    })?;
    if metadata.len() > STARTUP_FINGERPRINT_LIMIT as u64 {
        return Err(DaemonCompatibilityError::InvalidStartupEnvironment(
            format!(
                "{} exceeds the startup fingerprint safety bound",
                path.display()
            ),
        ));
    }
    fs::read(path).map_err(|error| {
        DaemonCompatibilityError::InvalidStartupEnvironment(format!(
            "could not read {}: {error}",
            path.display()
        ))
    })
}

pub(super) fn fingerprint_environment(
    environment: &BTreeMap<String, String>,
) -> Result<Vec<u8>, DaemonCompatibilityError> {
    let mut encoded = Vec::new();
    for (key, value) in environment {
        if encoded
            .len()
            .saturating_add(key.len())
            .saturating_add(value.len())
            .saturating_add(2)
            > STARTUP_FINGERPRINT_LIMIT
        {
            return Err(DaemonCompatibilityError::InvalidStartupEnvironment(
                "initialized environment exceeds the startup fingerprint safety bound".into(),
            ));
        }
        encoded.extend_from_slice(key.as_bytes());
        encoded.push(b'=');
        encoded.extend_from_slice(value.as_bytes());
        encoded.push(0);
    }
    Ok(encoded)
}
