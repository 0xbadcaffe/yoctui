//! Secure runtime records and process classification for daemon lifecycle.
use crate::{
    daemon::DaemonInstanceId,
    daemon_ipc::{IpcError, RuntimePaths, SOCKET_MODE},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::ffi::OsStrExt,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use thiserror::Error;

const MAX_RUNTIME_RECORD_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonRuntimeRecord {
    pub pid: u32,
    pub daemon_instance_id: DaemonInstanceId,
    pub started_unix_ms: u64,
    pub boot_id: String,
    pub executable: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRecordState {
    Current,
    Stale,
    ForeignProcess,
}

#[derive(Debug, Error)]
pub enum LifecycleRecordError {
    #[error(transparent)]
    Ipc(#[from] IpcError),
    #[error("unsafe daemon runtime record {path}: {reason}")]
    Unsafe { path: PathBuf, reason: String },
    #[error("daemon runtime record exceeds the 64 KiB limit")]
    TooLarge,
    #[error("invalid daemon runtime record: {0}")]
    Invalid(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub fn runtime_record_path(paths: &RuntimePaths) -> PathBuf {
    paths.directory.join("daemon.json")
}

pub fn read_runtime_record(
    paths: &RuntimePaths,
) -> Result<Option<DaemonRuntimeRecord>, LifecycleRecordError> {
    let path = runtime_record_path(paths);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    validate_record_metadata(&path, &metadata)?;
    if metadata.len() > MAX_RUNTIME_RECORD_BYTES {
        return Err(LifecycleRecordError::TooLarge);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(&path)?
        .take(MAX_RUNTIME_RECORD_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_RUNTIME_RECORD_BYTES {
        return Err(LifecycleRecordError::TooLarge);
    }
    Ok(Some(serde_json::from_slice(&bytes)?))
}

pub fn write_runtime_record(
    paths: &RuntimePaths,
    record: &DaemonRuntimeRecord,
) -> Result<(), LifecycleRecordError> {
    let destination = runtime_record_path(paths);
    if let Ok(metadata) = fs::symlink_metadata(&destination) {
        validate_record_metadata(&destination, &metadata)?;
    }
    let bytes = serde_json::to_vec(record)?;
    if bytes.len() as u64 > MAX_RUNTIME_RECORD_BYTES {
        return Err(LifecycleRecordError::TooLarge);
    }
    let temporary = paths
        .directory
        .join(format!("daemon.json.{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(SOCKET_MODE)
        .open(&temporary)?;
    let result = (|| -> Result<(), io::Error> {
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    Ok(())
}

pub fn remove_runtime_record(
    paths: &RuntimePaths,
    expected: DaemonInstanceId,
) -> Result<(), LifecycleRecordError> {
    let Some(record) = read_runtime_record(paths)? else {
        return Ok(());
    };
    if record.daemon_instance_id != expected {
        return Err(unsafe_record(
            &runtime_record_path(paths),
            "record belongs to another daemon instance",
        ));
    }
    fs::remove_file(runtime_record_path(paths))?;
    Ok(())
}

pub fn classify_runtime_record(
    record: &DaemonRuntimeRecord,
    current_boot_id: &str,
) -> RuntimeRecordState {
    if record.boot_id != current_boot_id || !process_exists(record.pid) {
        return RuntimeRecordState::Stale;
    }
    match process_executable(record.pid) {
        Some(executable) if executable_matches_record(&executable, &record.executable) => {
            RuntimeRecordState::Current
        }
        _ => RuntimeRecordState::ForeignProcess,
    }
}

fn executable_matches_record(process: &Path, recorded: &Path) -> bool {
    process == recorded
        || process
            .as_os_str()
            .as_bytes()
            .strip_suffix(b" (deleted)")
            .is_some_and(|path| path == recorded.as_os_str().as_bytes())
}

pub fn read_boot_id() -> Result<String, io::Error> {
    Ok(fs::read_to_string("/proc/sys/kernel/random/boot_id")?
        .trim()
        .to_owned())
}

fn process_exists(pid: u32) -> bool {
    let Ok(pid) = i32::try_from(pid) else {
        return false;
    };
    // SAFETY: kill with signal 0 does not deliver a signal; it probes PID access.
    unsafe {
        libc::kill(pid, 0) == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
}

fn process_executable(pid: u32) -> Option<PathBuf> {
    fs::read_link(format!("/proc/{pid}/exe")).ok()
}

fn validate_record_metadata(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<(), LifecycleRecordError> {
    // SAFETY: geteuid has no preconditions and does not modify memory.
    let uid = unsafe { libc::geteuid() };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(unsafe_record(path, "expected a non-symlink regular file"));
    }
    if metadata.uid() != uid {
        return Err(unsafe_record(path, "record is owned by another UID"));
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(unsafe_record(path, "record permissions are not private"));
    }
    Ok(())
}

fn unsafe_record(path: &Path, reason: impl Into<String>) -> LifecycleRecordError {
    LifecycleRecordError::Unsafe {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon_ipc::{DaemonListener, RUNTIME_DIRECTORY_MODE, runtime_paths_for};
    use std::{env, os::unix::fs::DirBuilderExt, time::SystemTime};

    fn paths(name: &str) -> RuntimePaths {
        let root = env::temp_dir().join(format!(
            "yoctui-daemon-lifecycle-{name}-{}",
            std::process::id(),
        ));
        let _ = fs::remove_dir_all(&root);
        fs::DirBuilder::new()
            .mode(RUNTIME_DIRECTORY_MODE)
            .create(&root)
            .unwrap();
        // SAFETY: geteuid has no preconditions.
        runtime_paths_for(root, unsafe { libc::geteuid() }).unwrap()
    }

    #[test]
    fn daemon_lifecycle_runtime_record_is_private_atomic_and_instance_guarded() {
        let paths = paths("record");
        let listener = DaemonListener::bind(&paths).unwrap();
        let record = DaemonRuntimeRecord {
            pid: std::process::id(),
            daemon_instance_id: DaemonInstanceId([4; 16]),
            started_unix_ms: SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis() as u64,
            boot_id: read_boot_id().unwrap(),
            executable: fs::read_link("/proc/self/exe").unwrap(),
        };
        write_runtime_record(&paths, &record).unwrap();
        assert_eq!(read_runtime_record(&paths).unwrap(), Some(record.clone()));
        assert_eq!(
            fs::symlink_metadata(runtime_record_path(&paths))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            SOCKET_MODE
        );
        assert_eq!(
            classify_runtime_record(&record, &record.boot_id),
            RuntimeRecordState::Current
        );
        assert_eq!(
            classify_runtime_record(&record, "different-boot"),
            RuntimeRecordState::Stale
        );
        assert!(remove_runtime_record(&paths, DaemonInstanceId([8; 16])).is_err());
        remove_runtime_record(&paths, record.daemon_instance_id).unwrap();
        drop(listener);
        fs::remove_dir_all(paths.directory.parent().unwrap()).unwrap();
    }

    #[test]
    fn daemon_lifecycle_accepts_only_exact_replaced_executable_identity() {
        let recorded = Path::new("/opt/yoctui/bin/yoctui");
        assert!(executable_matches_record(recorded, recorded));
        assert!(executable_matches_record(
            Path::new("/opt/yoctui/bin/yoctui (deleted)"),
            recorded
        ));
        assert!(!executable_matches_record(
            Path::new("/tmp/yoctui (deleted)"),
            recorded
        ));
        assert!(!executable_matches_record(
            Path::new("/opt/yoctui/bin/yoctui (deleted) (deleted)"),
            recorded
        ));
    }

    #[test]
    fn daemon_lifecycle_rejects_symlink_and_oversized_runtime_records() {
        let paths = paths("unsafe");
        let listener = DaemonListener::bind(&paths).unwrap();
        let path = runtime_record_path(&paths);
        std::os::unix::fs::symlink("/tmp", &path).unwrap();
        assert!(matches!(
            read_runtime_record(&paths),
            Err(LifecycleRecordError::Unsafe { .. })
        ));
        fs::remove_file(&path).unwrap();
        fs::write(&path, vec![b'x'; MAX_RUNTIME_RECORD_BYTES as usize + 1]).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(SOCKET_MODE)).unwrap();
        assert!(matches!(
            read_runtime_record(&paths),
            Err(LifecycleRecordError::TooLarge)
        ));
        fs::remove_file(path).unwrap();
        drop(listener);
        fs::remove_dir_all(paths.directory.parent().unwrap()).unwrap();
    }
}
