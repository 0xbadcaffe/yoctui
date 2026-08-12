//! Safe, bounded persistence for reconstructable daemon metadata.
use crate::daemon::{
    BitBakeCapability, BitBakeState, DaemonInstanceId, DaemonSnapshot, JobSummary, LifecycleState,
    LogRecord, ProjectProfileSummary, PtyKind, TerminalDimensions, WorkspaceIdentity,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use thiserror::Error;

pub const DAEMON_PERSIST_SCHEMA_VERSION: u32 = 1;
pub const MAX_DAEMON_PERSIST_BYTES: u64 = 4 * 1024 * 1024;
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonPersistPaths {
    pub directory: PathBuf,
    pub state: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonPersistedState {
    pub schema_version: u32,
    pub saved_unix_ms: u64,
    pub previous_daemon_instance_id: DaemonInstanceId,
    pub previous_boot_id: String,
    pub last_sequence: u64,
    pub last_generation: u64,
    pub workspace: Option<WorkspaceIdentity>,
    pub project_profile: ProjectProfileSummary,
    pub bitbake: PersistedBitBakeMetadata,
    pub job_history: Vec<JobSummary>,
    pub terminal_sessions: Vec<PersistedTerminalSession>,
    pub recent_logs: Vec<LogRecord>,
    pub recovery_warnings: Vec<String>,
    pub client_layouts: Vec<PersistedClientLayout>,
    pub preferences: PersistedPreferences,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedBitBakeMetadata {
    pub version: Option<String>,
    pub capabilities: Vec<BitBakeCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedTerminalSession {
    pub id: u64,
    pub name: String,
    pub kind: PtyKind,
    pub cwd: String,
    pub previous_lifecycle: LifecycleState,
    pub dimensions: TerminalDimensions,
    pub exit_code: Option<i32>,
    pub restartable: bool,
    /// Always false: process identity and liveness are never serialized.
    pub live_process_persisted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedClientLayout {
    pub client_key: String,
    pub layout_revision: u64,
    pub session_names: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedPreferences {
    pub theme: Option<String>,
    pub prefix_key: Option<String>,
    pub recent_logs_enabled: bool,
}

impl DaemonPersistedState {
    pub fn capture(
        snapshot: &DaemonSnapshot,
        saved_unix_ms: u64,
        previous_boot_id: String,
        client_layouts: Vec<PersistedClientLayout>,
        preferences: PersistedPreferences,
    ) -> Self {
        let recent_logs = if preferences.recent_logs_enabled {
            snapshot.recent_logs.clone()
        } else {
            Vec::new()
        };
        Self {
            schema_version: DAEMON_PERSIST_SCHEMA_VERSION,
            saved_unix_ms,
            previous_daemon_instance_id: snapshot.daemon_instance_id,
            previous_boot_id,
            last_sequence: snapshot.sequence,
            last_generation: snapshot.generation,
            workspace: snapshot.workspace.clone(),
            project_profile: snapshot.project_profile.clone(),
            bitbake: PersistedBitBakeMetadata {
                version: snapshot.bitbake.version.clone(),
                capabilities: snapshot.bitbake.capabilities.clone(),
            },
            job_history: snapshot.jobs.clone(),
            terminal_sessions: snapshot
                .pty_sessions
                .iter()
                .map(|session| PersistedTerminalSession {
                    id: session.id.0,
                    name: session.name.clone(),
                    kind: session.kind,
                    cwd: session.cwd.clone(),
                    previous_lifecycle: session.lifecycle,
                    dimensions: session.dimensions,
                    exit_code: session.exit_code,
                    restartable: session.restartable,
                    live_process_persisted: false,
                })
                .collect(),
            recent_logs,
            recovery_warnings: snapshot.recovery_warnings.clone(),
            client_layouts,
            preferences,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonRecoveryReport {
    pub previous_boot_changed: bool,
    pub lost_jobs: usize,
    pub lost_terminal_sessions: usize,
    pub bitbake_reconnect_recommended: bool,
}

pub fn recover_persisted_snapshot(
    mut current: DaemonSnapshot,
    persisted: &DaemonPersistedState,
    current_boot_id: &str,
) -> (DaemonSnapshot, DaemonRecoveryReport) {
    let previous_boot_changed = persisted.previous_boot_id != current_boot_id;
    let mut lost_jobs = 0;
    let mut lost_terminal_sessions = 0;
    current.workspace.clone_from(&persisted.workspace);
    current
        .project_profile
        .clone_from(&persisted.project_profile);
    let bitbake_reconnect_recommended =
        persisted.bitbake.version.is_some() || !persisted.bitbake.capabilities.is_empty();
    current.bitbake = BitBakeState {
        lifecycle: LifecycleState::Disconnected,
        version: persisted.bitbake.version.clone(),
        capabilities: persisted.bitbake.capabilities.clone(),
        diagnostic: bitbake_reconnect_recommended
            .then(|| "persisted BitBake identity requires a supported reconnect probe".into()),
    };
    current.jobs = persisted
        .job_history
        .iter()
        .cloned()
        .map(|mut job| {
            if !matches!(
                job.lifecycle,
                LifecycleState::Exited | LifecycleState::Failed | LifecycleState::Lost
            ) {
                job.lifecycle = LifecycleState::Lost;
                lost_jobs += 1;
            }
            job
        })
        .collect();
    current.pty_sessions = persisted
        .terminal_sessions
        .iter()
        .map(|session| {
            let lifecycle = if matches!(
                session.previous_lifecycle,
                LifecycleState::Exited | LifecycleState::Failed | LifecycleState::Lost
            ) {
                session.previous_lifecycle
            } else {
                lost_terminal_sessions += 1;
                LifecycleState::Lost
            };
            crate::daemon::PtySessionSummary {
                id: crate::daemon::PtySessionId(session.id),
                name: session.name.clone(),
                kind: session.kind,
                cwd: session.cwd.clone(),
                lifecycle,
                dimensions: session.dimensions,
                writer: None,
                writer_epoch: 0,
                viewers: 0,
                exit_code: session.exit_code,
                restartable: session.restartable,
            }
        })
        .collect();
    current.recent_logs.clone_from(&persisted.recent_logs);
    let current_warnings = std::mem::take(&mut current.recovery_warnings);
    current
        .recovery_warnings
        .clone_from(&persisted.recovery_warnings);
    for warning in current_warnings {
        if !current.recovery_warnings.contains(&warning) {
            current.recovery_warnings.push(warning);
        }
    }
    current.recovery_warnings.push(if previous_boot_changed {
        "host boot changed; all previously nonterminal jobs and PTYs were classified Lost".into()
    } else {
        "daemon instance restarted; all unrecoverable nonterminal jobs and PTYs were classified Lost"
            .into()
    });
    current.clients.clear();
    (
        current,
        DaemonRecoveryReport {
            previous_boot_changed,
            lost_jobs,
            lost_terminal_sessions,
            bitbake_reconnect_recommended,
        },
    )
}

pub fn persist_paths_for(root: &Path) -> Result<DaemonPersistPaths, DaemonPersistError> {
    if !root.is_absolute() {
        return Err(DaemonPersistError::Unsafe {
            path: root.to_path_buf(),
            reason: "state root must be absolute".into(),
        });
    }
    if !root.exists() {
        fs::create_dir_all(root)?;
    }
    let canonical = root.canonicalize()?;
    if canonical != root {
        return Err(DaemonPersistError::Unsafe {
            path: root.to_path_buf(),
            reason: "state root contains a symlink or non-canonical component".into(),
        });
    }
    validate_directory(root, false)?;
    let directory = root.join("yoctui");
    if directory.exists() {
        validate_directory(&directory, true)?;
    } else {
        fs::DirBuilder::new()
            .mode(PRIVATE_DIRECTORY_MODE)
            .create(&directory)?;
    }
    Ok(DaemonPersistPaths {
        state: directory.join("daemon-state.json"),
        directory,
    })
}

pub fn read_persisted_state(
    paths: &DaemonPersistPaths,
) -> Result<Option<DaemonPersistedState>, DaemonPersistError> {
    let metadata = match fs::symlink_metadata(&paths.state) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    validate_state_file(&paths.state, &metadata)?;
    if metadata.len() > MAX_DAEMON_PERSIST_BYTES {
        return Err(DaemonPersistError::TooLarge);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(&paths.state)?
        .take(MAX_DAEMON_PERSIST_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_DAEMON_PERSIST_BYTES {
        return Err(DaemonPersistError::TooLarge);
    }
    let state: DaemonPersistedState = serde_json::from_slice(&bytes)?;
    if state.schema_version != DAEMON_PERSIST_SCHEMA_VERSION {
        return Err(DaemonPersistError::UnsupportedSchema(state.schema_version));
    }
    if state
        .terminal_sessions
        .iter()
        .any(|session| session.live_process_persisted)
    {
        return Err(DaemonPersistError::Unsafe {
            path: paths.state.clone(),
            reason: "persisted terminal metadata claims a live process survived".into(),
        });
    }
    Ok(Some(state))
}

pub fn write_persisted_state(
    paths: &DaemonPersistPaths,
    state: &DaemonPersistedState,
) -> Result<(), DaemonPersistError> {
    if state.schema_version != DAEMON_PERSIST_SCHEMA_VERSION {
        return Err(DaemonPersistError::UnsupportedSchema(state.schema_version));
    }
    if state
        .terminal_sessions
        .iter()
        .any(|session| session.live_process_persisted)
    {
        return Err(DaemonPersistError::Unsafe {
            path: paths.state.clone(),
            reason: "live process identity cannot be persisted".into(),
        });
    }
    validate_directory(&paths.directory, true)?;
    if let Ok(metadata) = fs::symlink_metadata(&paths.state) {
        validate_state_file(&paths.state, &metadata)?;
    }
    let bytes = serde_json::to_vec(state)?;
    if bytes.len() as u64 > MAX_DAEMON_PERSIST_BYTES {
        return Err(DaemonPersistError::TooLarge);
    }
    let temporary = paths.directory.join(format!(
        "daemon-state.{}.{}.tmp",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE_MODE)
        .open(&temporary)?;
    let result = (|| -> Result<(), io::Error> {
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &paths.state)?;
        fs::File::open(&paths.directory)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    Ok(())
}

fn validate_directory(path: &Path, private: bool) -> Result<(), DaemonPersistError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DaemonPersistError::Unsafe {
            path: path.to_path_buf(),
            reason: "expected a non-symlink directory".into(),
        });
    }
    if metadata.uid() != effective_uid() {
        return Err(DaemonPersistError::Unsafe {
            path: path.to_path_buf(),
            reason: "directory is owned by another UID".into(),
        });
    }
    if private && metadata.permissions().mode() & 0o077 != 0 {
        return Err(DaemonPersistError::Unsafe {
            path: path.to_path_buf(),
            reason: "daemon state directory permissions are not private".into(),
        });
    }
    Ok(())
}

fn validate_state_file(path: &Path, metadata: &fs::Metadata) -> Result<(), DaemonPersistError> {
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DaemonPersistError::Unsafe {
            path: path.to_path_buf(),
            reason: "expected a non-symlink regular state file".into(),
        });
    }
    if metadata.uid() != effective_uid() {
        return Err(DaemonPersistError::Unsafe {
            path: path.to_path_buf(),
            reason: "state file is owned by another UID".into(),
        });
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(DaemonPersistError::Unsafe {
            path: path.to_path_buf(),
            reason: "state file permissions are not private".into(),
        });
    }
    Ok(())
}

fn effective_uid() -> u32 {
    // SAFETY: geteuid has no preconditions and does not modify memory.
    unsafe { libc::geteuid() }
}

#[derive(Debug, Error)]
pub enum DaemonPersistError {
    #[error("unsafe daemon persistence path {path}: {reason}")]
    Unsafe { path: PathBuf, reason: String },
    #[error("daemon persisted state exceeds the 4 MiB limit")]
    TooLarge,
    #[error("unsupported daemon persisted-state schema {0}")]
    UnsupportedSchema(u32),
    #[error("invalid daemon persisted state: {0}")]
    Invalid(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::{BitBakeState, LifecycleState};
    use std::os::unix::fs::DirBuilderExt;

    fn root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "yoctui-daemon-persist-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::DirBuilder::new()
            .mode(PRIVATE_DIRECTORY_MODE)
            .create(&path)
            .unwrap();
        path
    }

    fn snapshot() -> DaemonSnapshot {
        DaemonSnapshot {
            daemon_instance_id: DaemonInstanceId([3; 16]),
            sequence: 8,
            generation: 5,
            workspace: None,
            project_profile: ProjectProfileSummary::Absent,
            bitbake: BitBakeState {
                lifecycle: LifecycleState::Running,
                version: Some("2.8.1".into()),
                capabilities: vec![BitBakeCapability::WorkspaceInspection],
                diagnostic: None,
            },
            jobs: Vec::new(),
            pty_sessions: Vec::new(),
            clients: Vec::new(),
            recent_logs: vec![LogRecord {
                source: "test".into(),
                severity: crate::daemon::LogSeverity::Info,
                message: "retained".into(),
                unix_ms: 1,
            }],
            recovery_warnings: Vec::new(),
        }
    }

    #[test]
    fn daemon_persist_round_trips_private_bounded_metadata_without_processes() {
        let root = root("round-trip");
        let paths = persist_paths_for(&root).unwrap();
        let persisted = DaemonPersistedState::capture(
            &snapshot(),
            99,
            "boot".into(),
            vec![PersistedClientLayout {
                client_key: "terminal".into(),
                layout_revision: 2,
                session_names: vec!["build".into()],
            }],
            PersistedPreferences {
                theme: Some("dark".into()),
                prefix_key: Some("Ctrl-b".into()),
                recent_logs_enabled: true,
            },
        );
        write_persisted_state(&paths, &persisted).unwrap();
        assert_eq!(read_persisted_state(&paths).unwrap(), Some(persisted));
        assert_eq!(
            fs::symlink_metadata(&paths.state)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            PRIVATE_FILE_MODE
        );
        let text = fs::read_to_string(&paths.state).unwrap();
        assert!(!text.contains("\"pid\""));
        assert!(!text.contains("process_group"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn daemon_persist_rejects_symlinks_oversize_and_live_process_claims() {
        let root = root("unsafe");
        let paths = persist_paths_for(&root).unwrap();
        std::os::unix::fs::symlink("/tmp", &paths.state).unwrap();
        assert!(matches!(
            read_persisted_state(&paths),
            Err(DaemonPersistError::Unsafe { .. })
        ));
        fs::remove_file(&paths.state).unwrap();
        fs::write(
            &paths.state,
            vec![b'x'; MAX_DAEMON_PERSIST_BYTES as usize + 1],
        )
        .unwrap();
        fs::set_permissions(&paths.state, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        assert!(matches!(
            read_persisted_state(&paths),
            Err(DaemonPersistError::TooLarge)
        ));
        fs::remove_file(&paths.state).unwrap();

        let mut persisted = DaemonPersistedState::capture(
            &snapshot(),
            99,
            "boot".into(),
            Vec::new(),
            PersistedPreferences::default(),
        );
        persisted.terminal_sessions.push(PersistedTerminalSession {
            id: 1,
            name: "unsafe".into(),
            kind: PtyKind::Utility,
            cwd: "/tmp".into(),
            previous_lifecycle: LifecycleState::Running,
            dimensions: TerminalDimensions {
                columns: 80,
                rows: 24,
            },
            exit_code: None,
            restartable: true,
            live_process_persisted: true,
        });
        assert!(matches!(
            write_persisted_state(&paths, &persisted),
            Err(DaemonPersistError::Unsafe { .. })
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn daemon_recovery_marks_unrecoverable_work_lost_and_clears_live_attachments() {
        let mut prior = snapshot();
        prior.jobs.push(crate::daemon::JobSummary {
            id: crate::daemon::JobId(1),
            kind: crate::daemon::JobKind::BitBakeBuild,
            label: "core-image-minimal".into(),
            lifecycle: LifecycleState::Running,
            progress_current: Some(2),
            progress_total: Some(10),
            exit_code: None,
        });
        prior.pty_sessions.push(crate::daemon::PtySessionSummary {
            id: crate::daemon::PtySessionId(7),
            name: "devshell".into(),
            kind: PtyKind::Devshell,
            cwd: "/work/build".into(),
            lifecycle: LifecycleState::Running,
            dimensions: TerminalDimensions {
                columns: 120,
                rows: 40,
            },
            writer: Some(crate::daemon::ClientId([9; 16])),
            writer_epoch: 4,
            viewers: 2,
            exit_code: None,
            restartable: true,
        });
        let persisted = DaemonPersistedState::capture(
            &prior,
            99,
            "old-boot".into(),
            Vec::new(),
            PersistedPreferences::default(),
        );
        let mut current = snapshot();
        current.daemon_instance_id = DaemonInstanceId([8; 16]);
        current.sequence = 0;
        current.generation = 0;
        let (recovered, report) = recover_persisted_snapshot(current, &persisted, "new-boot");

        assert_eq!(recovered.daemon_instance_id, DaemonInstanceId([8; 16]));
        assert_eq!(recovered.jobs[0].lifecycle, LifecycleState::Lost);
        assert_eq!(recovered.pty_sessions[0].lifecycle, LifecycleState::Lost);
        assert_eq!(recovered.pty_sessions[0].writer, None);
        assert_eq!(recovered.pty_sessions[0].viewers, 0);
        assert_eq!(recovered.bitbake.lifecycle, LifecycleState::Disconnected);
        assert!(recovered.bitbake.diagnostic.is_some());
        assert!(recovered.clients.is_empty());
        assert_eq!(report.lost_jobs, 1);
        assert_eq!(report.lost_terminal_sessions, 1);
        assert!(report.previous_boot_changed);
        assert!(report.bitbake_reconnect_recommended);
    }
}
