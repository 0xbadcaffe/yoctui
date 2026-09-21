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
    validate_persisted_raw_executions(&state.raw_executions)?;
    validate_persisted_raw_history(&state.raw_history)?;
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
    validate_persisted_raw_executions(&state.raw_executions)?;
    validate_persisted_raw_history(&state.raw_history)?;
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

fn validate_persisted_raw_executions(
    executions: &[RawExecutionSnapshotData],
) -> Result<(), DaemonPersistError> {
    if executions.len() > MAX_RAW_EXECUTION_REQUESTS {
        return Err(DaemonPersistError::TooLarge);
    }
    for execution in executions {
        execution
            .validate()
            .map_err(|error| DaemonPersistError::Unsafe {
                path: PathBuf::from("raw-execution"),
                reason: error.to_string(),
            })?;
    }
    Ok(())
}

fn validate_persisted_raw_history(
    records: &[RawHistoryRecordData],
) -> Result<(), DaemonPersistError> {
    crate::daemon::validate_raw_history_records(records).map_err(|error| {
        DaemonPersistError::Unsafe {
            path: PathBuf::from("raw-history"),
            reason: error.to_string(),
        }
    })
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
