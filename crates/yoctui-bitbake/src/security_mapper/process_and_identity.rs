use std::{
    collections::VecDeque,
    ffi::{OsStr, OsString},
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, SystemTime},
};

use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
    time::Instant,
};
use yoctui_model::{
    MAX_SECURITY_PATHS, MAX_SECURITY_TEXT_BYTES, SecurityOperation, SecurityOperationPreview,
    SecurityOutputStream, SecuritySessionId,
};

use crate::output_text;

const MAX_SECURITY_MAPPER_ARGUMENTS: usize = 64;
const MAX_SECURITY_MAPPER_LINE_BYTES: usize = MAX_SECURITY_TEXT_BYTES;
const SECURITY_MAPPER_EVENT_CHANNEL_CAPACITY: usize = 256;
const SECURITY_MAPPER_OPERATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const SECURITY_MAPPER_SPAWN_ATTEMPTS: usize = 4;
const SECURITY_MAPPER_SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);

#[cfg(unix)]
fn is_transient_security_mapper_spawn_error(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ETXTBSY)
}

#[cfg(not(unix))]
fn is_transient_security_mapper_spawn_error(_error: &io::Error) -> bool {
    false
}

async fn spawn_security_mapper_process(process: &mut Command) -> io::Result<Child> {
    for attempt in 1..=SECURITY_MAPPER_SPAWN_ATTEMPTS {
        match process.spawn() {
            Ok(child) => return Ok(child),
            Err(error)
                if attempt < SECURITY_MAPPER_SPAWN_ATTEMPTS
                    && is_transient_security_mapper_spawn_error(&error) =>
            {
                tokio::time::sleep(SECURITY_MAPPER_SPAWN_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the bounded Security package-mapping process spawn loop always returns")
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SecurityMapperAdapterError {
    #[error("Security package-mapping preview is invalid: {0}")]
    InvalidPreview(String),
    #[error("Security package-mapping preview was modified after confirmation")]
    PreviewMismatch,
    #[error("Security package-mapping executable is unsafe: {0}")]
    UnsafeExecutable(PathBuf),
    #[error("Security package-mapping input is unsafe: {0}")]
    UnsafeInput(PathBuf),
    #[error("Security package-mapping identity became stale: {0}")]
    StaleIdentity(PathBuf),
    #[error("a Security package-mapping process or unconsumed event is already active")]
    Busy,
    #[error("could not start Security package mapping: {0}")]
    Spawn(String),
    #[error("Security package-mapping process stream is unavailable: {0:?}")]
    StreamUnavailable(SecurityOutputStream),
    #[error("Security package-mapping runner is not active")]
    NotRunning,
    #[error("Security package-mapping process control failed: {0}")]
    ProcessControl(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileIdentity {
    path: PathBuf,
    size_bytes: u64,
    modified_at: SystemTime,
    directory: bool,
}

impl FileIdentity {
    fn executable(path: &Path) -> Result<Self, SecurityMapperAdapterError> {
        if path.file_name() != Some(OsStr::new("cve-check-map-pkgs")) {
            return Err(SecurityMapperAdapterError::UnsafeExecutable(path.into()));
        }
        let metadata = safe_metadata(path, false)
            .map_err(|_| SecurityMapperAdapterError::UnsafeExecutable(path.into()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 == 0 {
                return Err(SecurityMapperAdapterError::UnsafeExecutable(path.into()));
            }
        }
        Self::from_metadata(path, metadata, false)
            .map_err(|_| SecurityMapperAdapterError::UnsafeExecutable(path.into()))
    }

    fn input(path: &Path) -> Result<Self, SecurityMapperAdapterError> {
        let metadata = safe_metadata(path, true)
            .map_err(|_| SecurityMapperAdapterError::UnsafeInput(path.into()))?;
        let directory = metadata.is_dir();
        Self::from_metadata(path, metadata, directory)
            .map_err(|_| SecurityMapperAdapterError::UnsafeInput(path.into()))
    }

    fn from_metadata(path: &Path, metadata: fs::Metadata, directory: bool) -> Result<Self, ()> {
        let canonical = fs::canonicalize(path).map_err(|_| ())?;
        if canonical != path {
            return Err(());
        }
        Ok(Self {
            path: canonical,
            size_bytes: metadata.len(),
            modified_at: metadata.modified().map_err(|_| ())?,
            directory,
        })
    }

    fn revalidate_executable(&self) -> Result<(), SecurityMapperAdapterError> {
        let current = Self::executable(&self.path)?;
        if current != *self {
            return Err(SecurityMapperAdapterError::StaleIdentity(self.path.clone()));
        }
        Ok(())
    }

    fn revalidate_input(&self) -> Result<(), SecurityMapperAdapterError> {
        let current = Self::input(&self.path)?;
        if current != *self {
            return Err(SecurityMapperAdapterError::StaleIdentity(self.path.clone()));
        }
        Ok(())
    }
}

fn safe_metadata(path: &Path, allow_directory: bool) -> Result<fs::Metadata, ()> {
    if !path.is_absolute() || path == Path::new("/") {
        return Err(());
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if metadata.file_type().is_symlink()
        || (!metadata.is_file() && !(allow_directory && metadata.is_dir()))
    {
        return Err(());
    }
    Ok(metadata)
}
