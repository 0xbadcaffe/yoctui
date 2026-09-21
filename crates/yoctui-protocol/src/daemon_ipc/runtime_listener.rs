#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePaths {
    pub directory: PathBuf,
    pub socket: PathBuf,
}

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("daemon runtime path must be absolute: {0}")]
    RelativeRuntimePath(PathBuf),
    #[error("unsafe daemon runtime path {path}: {reason}")]
    UnsafeRuntimePath { path: PathBuf, reason: String },
    #[error("daemon is already running at {0}")]
    AlreadyRunning(PathBuf),
    #[error("daemon unavailable at {path}: {source}")]
    Unavailable { path: PathBuf, source: io::Error },
    #[error("daemon IPC timed out during {0}")]
    Timeout(&'static str),
    #[error("daemon peer UID {actual} does not match expected UID {expected}")]
    PeerUid { expected: u32, actual: u32 },
    #[error("daemon IPC disconnected")]
    Disconnected,
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Protocol(#[from] DaemonProtocolError),
}

pub fn runtime_paths() -> Result<RuntimePaths, IpcError> {
    let uid = effective_uid();
    let root = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("/run/user/{uid}")));
    runtime_paths_for(root, uid)
}

pub fn runtime_paths_for(root: PathBuf, uid: u32) -> Result<RuntimePaths, IpcError> {
    if !root.is_absolute() {
        return Err(IpcError::RelativeRuntimePath(root));
    }
    validate_owned_directory(&root, uid, true)?;
    let canonical = root
        .canonicalize()
        .map_err(|source| IpcError::Unavailable {
            path: root.clone(),
            source,
        })?;
    if canonical != root {
        return Err(unsafe_path(
            &root,
            "runtime directory contains a symlink or non-canonical component",
        ));
    }
    let directory = root.join("yoctui");
    Ok(RuntimePaths {
        socket: directory.join("daemon.sock"),
        directory,
    })
}

#[derive(Debug)]
pub struct DaemonListener {
    listener: UnixListener,
    socket: PathBuf,
    socket_device: u64,
    socket_inode: u64,
    expected_uid: u32,
}

impl DaemonListener {
    pub fn bind(paths: &RuntimePaths) -> Result<Self, IpcError> {
        let uid = effective_uid();
        prepare_runtime_directory(paths, uid)?;
        clean_stale_socket(&paths.socket, uid)?;
        let listener = UnixListener::bind(&paths.socket)?;
        listener.set_nonblocking(true)?;
        if let Err(error) =
            fs::set_permissions(&paths.socket, fs::Permissions::from_mode(SOCKET_MODE))
        {
            let _ = fs::remove_file(&paths.socket);
            return Err(error.into());
        }
        let metadata = fs::symlink_metadata(&paths.socket)?;
        if !metadata.file_type().is_socket()
            || metadata.uid() != uid
            || metadata.permissions().mode() & 0o777 != SOCKET_MODE
        {
            let _ = fs::remove_file(&paths.socket);
            return Err(unsafe_path(
                &paths.socket,
                "bound socket identity or mode changed",
            ));
        }
        Ok(Self {
            listener,
            socket: paths.socket.clone(),
            socket_device: metadata.dev(),
            socket_inode: metadata.ino(),
            expected_uid: uid,
        })
    }

    pub fn accept(&self, timeout: Duration) -> Result<DaemonConnection, IpcError> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    let actual = peer_uid(&stream)?;
                    if actual != self.expected_uid {
                        return Err(IpcError::PeerUid {
                            expected: self.expected_uid,
                            actual,
                        });
                    }
                    return Ok(DaemonConnection {
                        stream,
                        server_mode: true,
                        pending: Vec::new(),
                        expected_frame_len: None,
                        write_poisoned: false,
                        outgoing: None,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    let now = Instant::now();
                    if now >= deadline {
                        return Err(IpcError::Timeout("accept"));
                    }
                    wait_until_readable(self.listener.as_raw_fd(), deadline - now)?;
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    /// Block until the listener or one of the attached client streams can
    /// make progress, or until `timeout` expires.
    ///
    /// A daemon can use this single readiness wait instead of repeatedly
    /// polling every client. Buffered complete frames also count as ready so
    /// callers never sleep while protocol work is already available.
    pub fn wait_for_activity(
        &self,
        connections: &[&DaemonConnection],
        timeout: Duration,
    ) -> Result<bool, IpcError> {
        self.wait_for_activity_with_additional_fd(connections, None, timeout)
    }

    /// Also wake for one process-local readiness descriptor, such as a
    /// coalesced supervisor notification pipe.
    pub fn wait_for_activity_with_additional_fd(
        &self,
        connections: &[&DaemonConnection],
        additional_fd: Option<RawFd>,
        timeout: Duration,
    ) -> Result<bool, IpcError> {
        if connections.iter().any(|connection| {
            !connection.event_write_pending()
                && (connection
                    .expected_frame_len
                    .is_some_and(|frame_len| connection.pending.len() >= frame_len)
                    || (connection.expected_frame_len.is_none() && connection.pending.len() >= 4))
        }) {
            return Ok(true);
        }

        let mut descriptors = Vec::with_capacity(connections.len() + 2);
        descriptors.push(libc::pollfd {
            fd: self.listener.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        });
        descriptors.extend(connections.iter().map(|connection| libc::pollfd {
            fd: connection.stream.as_raw_fd(),
            events: if connection.event_write_pending() {
                libc::POLLOUT
            } else {
                libc::POLLIN
            },
            revents: 0,
        }));
        if let Some(fd) = additional_fd {
            descriptors.push(libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            });
        }
        let timeout_ms = if timeout.is_zero() {
            0
        } else {
            yoctui_utils::poll_timeout_ms(timeout)
        };
        // SAFETY: `descriptors` owns initialized pollfd values and every
        // listener/connection descriptor remains borrowed for this call.
        let result = unsafe {
            libc::poll(
                descriptors.as_mut_ptr(),
                descriptors.len() as libc::nfds_t,
                timeout_ms,
            )
        };
        if result >= 0 {
            return Ok(result > 0);
        }
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            return Ok(false);
        }
        Err(error.into())
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket
    }
}

fn wait_until_readable(fd: std::os::fd::RawFd, timeout: Duration) -> Result<(), IpcError> {
    let timeout_ms = yoctui_utils::poll_timeout_ms(timeout);
    let mut descriptor = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: `descriptor` points to one initialized pollfd and the listener
    // owns `fd` for the complete duration of this blocking call.
    let result = unsafe { libc::poll(&mut descriptor, 1, timeout_ms) };
    if result >= 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::Interrupted {
        return Ok(());
    }
    Err(error.into())
}

impl Drop for DaemonListener {
    fn drop(&mut self) {
        let Ok(metadata) = fs::symlink_metadata(&self.socket) else {
            return;
        };
        if metadata.file_type().is_socket()
            && metadata.dev() == self.socket_device
            && metadata.ino() == self.socket_inode
        {
            let _ = fs::remove_file(&self.socket);
        }
    }
}
