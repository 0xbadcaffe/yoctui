use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
    time::Duration,
};

use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::{process::Command, time::timeout};

const MAX_SETUP_BYTES: u64 = 256 * 1024;
const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;
const MAX_VARIABLES: usize = 4_096;
const MAX_VALUE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkShellPreview {
    pub identity: String,
    pub root: PathBuf,
    pub setup_file: PathBuf,
    pub shell: PathBuf,
    setup_digest: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkShellEnvironment {
    pub identity: String,
    pub root: PathBuf,
    pub setup_file: PathBuf,
    pub shell: PathBuf,
    pub environment: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SdkShellAdapter {
    timeout: Duration,
    capture_shell: PathBuf,
}

impl Default for SdkShellAdapter {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            capture_shell: fs::canonicalize("/bin/bash")
                .unwrap_or_else(|_| PathBuf::from("/bin/bash")),
        }
    }
}

impl SdkShellAdapter {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            ..Self::default()
        }
    }

    pub fn inspect(
        &self,
        identity: String,
        root: PathBuf,
        shell: PathBuf,
    ) -> Result<SdkShellPreview, SdkShellError> {
        validate_identity(&identity)?;
        let root = canonical_directory(&root)?;
        let shell = canonical_executable(&shell)?;
        let setup_file = find_setup_file(&root)?;
        let setup_digest = digest_setup(&setup_file)?;
        canonical_executable(&self.capture_shell)?;
        Ok(SdkShellPreview {
            identity,
            root,
            setup_file,
            shell,
            setup_digest,
        })
    }

    pub async fn capture(
        &self,
        preview: &SdkShellPreview,
    ) -> Result<SdkShellEnvironment, SdkShellError> {
        validate_identity(&preview.identity)?;
        if canonical_directory(&preview.root)? != preview.root
            || canonical_executable(&preview.shell)? != preview.shell
            || find_setup_file(&preview.root)? != preview.setup_file
            || digest_setup(&preview.setup_file)? != preview.setup_digest
        {
            return Err(SdkShellError::ChangedAfterPreview);
        }
        let capture_shell = canonical_executable(&self.capture_shell)?;
        let mut command = Command::new(capture_shell);
        command
            .arg("--noprofile")
            .arg("--norc")
            .arg("-c")
            .arg("set -e; . \"$1\"; env -0")
            .arg("yoctui-sdk-environment")
            .arg(&preview.setup_file)
            .current_dir(&preview.root)
            .env_clear()
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        for name in [
            "HOME", "USER", "LOGNAME", "PATH", "LANG", "LC_ALL", "TERM", "TMPDIR",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let child = command
            .spawn()
            .map_err(|error| SdkShellError::Capture(error.to_string()))?;
        let output = timeout(self.timeout, child.wait_with_output())
            .await
            .map_err(|_| SdkShellError::Timeout)?
            .map_err(|error| SdkShellError::Capture(error.to_string()))?;
        if output.stdout.len() > MAX_OUTPUT_BYTES || output.stderr.len() > MAX_OUTPUT_BYTES {
            return Err(SdkShellError::OutputTooLarge);
        }
        if !output.status.success() {
            return Err(SdkShellError::Capture(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ));
        }
        let environment = parse_environment(&output.stdout)?;
        if !environment.contains_key("PATH") {
            return Err(SdkShellError::MissingPath);
        }
        Ok(SdkShellEnvironment {
            identity: preview.identity.clone(),
            root: preview.root.clone(),
            setup_file: preview.setup_file.clone(),
            shell: preview.shell.clone(),
            environment,
        })
    }
}

fn validate_identity(identity: &str) -> Result<(), SdkShellError> {
    if identity.is_empty()
        || identity.len() > 128
        || identity.chars().any(|character| character.is_control())
    {
        return Err(SdkShellError::InvalidIdentity);
    }
    Ok(())
}

fn canonical_directory(path: &Path) -> Result<PathBuf, SdkShellError> {
    if !path.is_absolute()
        || path == Path::new("/")
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(SdkShellError::UnsafePath(path.into()));
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| SdkShellError::UnsafePath(path.into()))?;
    let canonical = fs::canonicalize(path).map_err(|_| SdkShellError::UnsafePath(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || canonical != path {
        return Err(SdkShellError::UnsafePath(path.into()));
    }
    Ok(canonical)
}

fn canonical_executable(path: &Path) -> Result<PathBuf, SdkShellError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| SdkShellError::UnsafePath(path.into()))?;
    let canonical = fs::canonicalize(path).map_err(|_| SdkShellError::UnsafePath(path.into()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(SdkShellError::UnsafePath(path.into()));
        }
    }
    if metadata.file_type().is_symlink() || !metadata.is_file() || canonical != path {
        return Err(SdkShellError::UnsafePath(path.into()));
    }
    Ok(canonical)
}

fn find_setup_file(root: &Path) -> Result<PathBuf, SdkShellError> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(root).map_err(|_| SdkShellError::UnsafePath(root.into()))? {
        let path = entry
            .map_err(|_| SdkShellError::UnsafePath(root.into()))?
            .path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("environment-setup-") {
            continue;
        }
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| SdkShellError::UnsafePath(path.clone()))?;
        let canonical =
            fs::canonicalize(&path).map_err(|_| SdkShellError::UnsafePath(path.clone()))?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > MAX_SETUP_BYTES
            || canonical != path
            || canonical.parent() != Some(root)
        {
            return Err(SdkShellError::UnsafePath(path));
        }
        matches.push(canonical);
    }
    matches.sort();
    if matches.len() != 1 {
        return Err(SdkShellError::SetupCount(matches.len()));
    }
    Ok(matches.remove(0))
}

fn digest_setup(path: &Path) -> Result<[u8; 32], SdkShellError> {
    let bytes = fs::read(path).map_err(|_| SdkShellError::UnsafePath(path.into()))?;
    if bytes.len() as u64 > MAX_SETUP_BYTES {
        return Err(SdkShellError::UnsafePath(path.into()));
    }
    Ok(Sha256::digest(bytes).into())
}

fn parse_environment(bytes: &[u8]) -> Result<BTreeMap<String, String>, SdkShellError> {
    let mut environment = BTreeMap::new();
    for record in bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let record = std::str::from_utf8(record).map_err(|_| SdkShellError::InvalidOutput)?;
        let Some((name, value)) = record.split_once('=') else {
            return Err(SdkShellError::InvalidOutput);
        };
        if !valid_name(name)
            || value.len() > MAX_VALUE_BYTES
            || matches!(name, "BASH_ENV" | "ENV" | "CDPATH" | "GLOBIGNORE")
        {
            return Err(SdkShellError::UnsafeVariable(name.into()));
        }
        if environment.insert(name.into(), value.into()).is_some()
            || environment.len() > MAX_VARIABLES
        {
            return Err(SdkShellError::TooManyVariables);
        }
    }
    Ok(environment)
}

fn valid_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(byte) if byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SdkShellError {
    #[error("invalid SDK environment identity")]
    InvalidIdentity,
    #[error("unsafe SDK shell path: {0}")]
    UnsafePath(PathBuf),
    #[error("expected exactly one environment-setup-* file, found {0}")]
    SetupCount(usize),
    #[error("SDK environment changed after preview")]
    ChangedAfterPreview,
    #[error("SDK environment capture timed out")]
    Timeout,
    #[error("SDK environment capture failed: {0}")]
    Capture(String),
    #[error("SDK environment output exceeded bounds")]
    OutputTooLarge,
    #[error("SDK environment output was invalid")]
    InvalidOutput,
    #[error("unsafe SDK environment variable: {0}")]
    UnsafeVariable(String),
    #[error("SDK environment contains too many variables")]
    TooManyVariables,
    #[error("SDK environment did not provide PATH")]
    MissingPath,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pty_sdk_shell_captures_child_only_environment_and_detects_changes() {
        let root = std::env::temp_dir().join(format!(
            "yoctui-sdk-shell-{}-{}",
            std::process::id(),
            std::time::SystemTime::UNIX_EPOCH
                .elapsed()
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let setup = root.join("environment-setup-test");
        fs::write(
            &setup,
            "export YOCTUI_SDK_VALUE=ready\nexport PATH=/sdk/bin:$PATH\n",
        )
        .unwrap();
        let adapter = SdkShellAdapter::default();
        let preview = adapter
            .inspect(
                "sdk-test".into(),
                root.clone(),
                fs::canonicalize("/bin/bash").unwrap(),
            )
            .unwrap();
        let captured = adapter.capture(&preview).await.unwrap();
        assert_eq!(
            captured.environment.get("YOCTUI_SDK_VALUE"),
            Some(&"ready".into())
        );
        assert!(captured.environment["PATH"].starts_with("/sdk/bin:"));
        assert!(std::env::var_os("YOCTUI_SDK_VALUE").is_none());
        fs::write(&setup, "export YOCTUI_SDK_VALUE=changed\n").unwrap();
        assert_eq!(
            adapter.capture(&preview).await,
            Err(SdkShellError::ChangedAfterPreview)
        );
        fs::remove_dir_all(root).unwrap();
    }
}
