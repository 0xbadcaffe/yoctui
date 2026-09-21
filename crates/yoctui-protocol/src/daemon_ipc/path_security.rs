fn prepare_runtime_directory(paths: &RuntimePaths, uid: u32) -> Result<(), IpcError> {
    if !paths.directory.is_absolute() || !paths.socket.is_absolute() {
        return Err(unsafe_path(
            &paths.directory,
            "daemon runtime and socket paths must be absolute",
        ));
    }
    let root = paths
        .directory
        .parent()
        .ok_or_else(|| unsafe_path(&paths.directory, "runtime directory has no parent"))?;
    validate_owned_directory(root, uid, true)?;
    if paths.directory.exists() {
        validate_owned_directory(&paths.directory, uid, true)?;
    } else {
        fs::DirBuilder::new()
            .mode(RUNTIME_DIRECTORY_MODE)
            .create(&paths.directory)?;
        validate_owned_directory(&paths.directory, uid, true)?;
    }
    if paths.socket.parent() != Some(paths.directory.as_path()) {
        return Err(unsafe_path(
            &paths.socket,
            "socket is outside the daemon runtime directory",
        ));
    }
    Ok(())
}

fn validate_owned_directory(path: &Path, uid: u32, require_private: bool) -> Result<(), IpcError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| IpcError::Unavailable {
        path: path.to_path_buf(),
        source: error,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(unsafe_path(path, "expected a non-symlink directory"));
    }
    if metadata.uid() != uid {
        return Err(unsafe_path(path, "directory is owned by another UID"));
    }
    if require_private && metadata.permissions().mode() & 0o077 != 0 {
        return Err(unsafe_path(path, "directory permissions are not private"));
    }
    Ok(())
}

fn clean_stale_socket(path: &Path, uid: u32) -> Result<(), IpcError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || !metadata.file_type().is_socket() {
        return Err(unsafe_path(path, "existing socket path is not a socket"));
    }
    if metadata.uid() != uid {
        return Err(unsafe_path(path, "existing socket is owned by another UID"));
    }
    match UnixStream::connect(path) {
        Ok(_) => Err(IpcError::AlreadyRunning(path.to_path_buf())),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
            ) =>
        {
            fs::remove_file(path)?;
            Ok(())
        }
        Err(error) => Err(IpcError::Unavailable {
            path: path.to_path_buf(),
            source: error,
        }),
    }
}

fn map_timeout(error: io::Error) -> IpcError {
    if matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
    ) {
        IpcError::Timeout("read/write")
    } else {
        IpcError::Io(error)
    }
}

fn unsafe_path(path: &Path, reason: impl Into<String>) -> IpcError {
    IpcError::UnsafeRuntimePath {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

fn effective_uid() -> u32 {
    // SAFETY: geteuid has no preconditions and does not modify memory.
    unsafe { libc::geteuid() }
}

#[cfg(target_os = "linux")]
fn peer_uid(stream: &UnixStream) -> Result<u32, IpcError> {
    use std::os::fd::AsRawFd;
    let mut credentials = std::mem::MaybeUninit::<libc::ucred>::uninit();
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: credentials points to writable storage of `length` bytes and the
    // supplied file descriptor is owned by a live Unix stream.
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            credentials.as_mut_ptr().cast(),
            &mut length,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error().into());
    }
    if length as usize != std::mem::size_of::<libc::ucred>() {
        return Err(unsafe_path(Path::new("<peer>"), "invalid peer credentials"));
    }
    // SAFETY: successful getsockopt initialized the complete ucred value.
    Ok(unsafe { credentials.assume_init() }.uid)
}

#[cfg(not(target_os = "linux"))]
fn peer_uid(_stream: &UnixStream) -> Result<u32, IpcError> {
    // Other Unix targets must add their native peer-credential API before IPC
    // is enabled there; permissions alone are not silently treated as auth.
    Err(unsafe_path(
        Path::new("<peer>"),
        "peer UID verification is unsupported on this Unix target",
    ))
}
