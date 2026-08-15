//! Secure local Unix-domain transport for the daemon protocol.
use crate::daemon::{DaemonProtocolError, MAX_FRAME_BYTES, decode_frame, encode_frame};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    env, fs,
    io::{self, Read, Write},
    os::unix::{
        fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;

pub const RUNTIME_DIRECTORY_MODE: u32 = 0o700;
pub const SOCKET_MODE: u32 = 0o600;
const CONNECT_RETRY_INTERVAL: Duration = Duration::from_millis(10);

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
        self.listener.set_nonblocking(true)?;
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
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(IpcError::Timeout("accept"));
                    }
                    thread::sleep(CONNECT_RETRY_INTERVAL.min(timeout));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket
    }
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

#[derive(Debug)]
pub struct DaemonConnection {
    stream: UnixStream,
    server_mode: bool,
}

impl DaemonConnection {
    pub fn connect(paths: &RuntimePaths, timeout: Duration) -> Result<Self, IpcError> {
        let deadline = Instant::now() + timeout;
        loop {
            match UnixStream::connect(&paths.socket) {
                Ok(stream) => {
                    return Ok(Self {
                        stream,
                        server_mode: false,
                    });
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound
                            | io::ErrorKind::ConnectionRefused
                            | io::ErrorKind::WouldBlock
                    ) && Instant::now() < deadline =>
                {
                    thread::sleep(CONNECT_RETRY_INTERVAL.min(timeout));
                }
                Err(source) => {
                    return Err(IpcError::Unavailable {
                        path: paths.socket.clone(),
                        source,
                    });
                }
            }
        }
    }

    pub fn set_timeout(&self, timeout: Option<Duration>) -> Result<(), IpcError> {
        self.set_read_timeout(timeout)?;
        self.set_write_timeout(timeout)?;
        Ok(())
    }

    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<(), IpcError> {
        self.stream.set_read_timeout(timeout)?;
        Ok(())
    }

    pub fn set_write_timeout(&self, timeout: Option<Duration>) -> Result<(), IpcError> {
        self.stream.set_write_timeout(timeout)?;
        Ok(())
    }

    pub fn send<T: Serialize>(&mut self, message: &T) -> Result<(), IpcError> {
        let frame = encode_frame(message)?;
        match self.stream.write_all(&frame).map_err(map_timeout) {
            Ok(()) => {}
            Err(error) if self.server_mode && is_peer_disconnect(&error) => return Ok(()),
            Err(error) => return Err(error),
        }
        match self.stream.flush().map_err(map_timeout) {
            Ok(()) => {}
            Err(error) if self.server_mode && is_peer_disconnect(&error) => return Ok(()),
            Err(error) => return Err(error),
        }
        Ok(())
    }

    pub fn receive<T: DeserializeOwned>(&mut self) -> Result<T, IpcError> {
        let mut prefix = [0_u8; 4];
        read_exact(&mut self.stream, &mut prefix)?;
        let length = u32::from_be_bytes(prefix) as usize;
        if length > MAX_FRAME_BYTES {
            return Err(DaemonProtocolError::TooLarge.into());
        }
        let mut frame = Vec::with_capacity(length + 4);
        frame.extend_from_slice(&prefix);
        frame.resize(length + 4, 0);
        read_exact(&mut self.stream, &mut frame[4..])?;
        Ok(decode_frame(&frame)?)
    }

    pub fn peer_uid(&self) -> Result<u32, IpcError> {
        peer_uid(&self.stream)
    }
}

fn is_peer_disconnect(error: &IpcError) -> bool {
    match error {
        IpcError::Disconnected => true,
        IpcError::Timeout(_) => true,
        IpcError::Io(source) => matches!(
            source.kind(),
            io::ErrorKind::BrokenPipe
                | io::ErrorKind::ConnectionReset
                | io::ErrorKind::NotConnected
        ),
        _ => false,
    }
}

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

fn read_exact(stream: &mut UnixStream, bytes: &mut [u8]) -> Result<(), IpcError> {
    match stream.read_exact(bytes) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Err(IpcError::Disconnected),
        Err(error) => Err(map_timeout(error)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::ClientMessage;
    use std::sync::mpsc;

    fn test_paths(name: &str) -> RuntimePaths {
        let root = env::temp_dir().join(format!(
            "yoctui-daemon-ipc-{name}-{}-{}",
            std::process::id(),
            Instant::now().elapsed().as_nanos()
        ));
        fs::DirBuilder::new()
            .mode(RUNTIME_DIRECTORY_MODE)
            .create(&root)
            .unwrap();
        runtime_paths_for(root, effective_uid()).unwrap()
    }

    fn cleanup(paths: &RuntimePaths) {
        let _ = fs::remove_file(&paths.socket);
        let _ = fs::remove_dir(&paths.directory);
        if let Some(root) = paths.directory.parent() {
            let _ = fs::remove_dir(root);
        }
    }

    #[test]
    fn daemon_ipc_binds_private_socket_and_authenticates_peer() {
        let paths = test_paths("round-trip");
        let listener = DaemonListener::bind(&paths).unwrap();
        let metadata = fs::symlink_metadata(listener.socket_path()).unwrap();
        assert!(metadata.file_type().is_socket());
        assert_eq!(metadata.permissions().mode() & 0o777, SOCKET_MODE);
        assert_eq!(metadata.uid(), effective_uid());

        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            let mut client =
                DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
            client.set_timeout(Some(Duration::from_secs(1))).unwrap();
            client.send(&ClientMessage::Pong { nonce: 19 }).unwrap();
            client.peer_uid().unwrap()
        });
        let mut server = listener.accept(Duration::from_secs(1)).unwrap();
        server.set_timeout(Some(Duration::from_secs(1))).unwrap();
        assert_eq!(
            server.receive::<ClientMessage>().unwrap(),
            ClientMessage::Pong { nonce: 19 }
        );
        assert_eq!(client.join().unwrap(), effective_uid());
        drop(listener);
        assert!(!paths.socket.exists());
        cleanup(&paths);
    }

    #[test]
    fn server_send_ignores_peer_disconnect_without_poisoning_daemon() {
        let paths = test_paths("peer-disconnect");
        let listener = DaemonListener::bind(&paths).unwrap();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            let mut connection =
                DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
            connection.send(&ClientMessage::Pong { nonce: 1 }).unwrap();
            connection
        });
        let mut server = listener.accept(Duration::from_secs(1)).unwrap();
        server.receive::<ClientMessage>().unwrap();
        drop(client.join().unwrap());
        assert!(server.send(&ClientMessage::Pong { nonce: 7 }).is_ok());
        cleanup(&paths);
    }

    #[test]
    fn security_daemon_enforces_private_runtime_and_same_uid_peer() {
        let paths = test_paths("security");
        let listener = DaemonListener::bind(&paths).unwrap();
        let metadata = fs::symlink_metadata(listener.socket_path()).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, SOCKET_MODE);
        assert_eq!(metadata.uid(), effective_uid());
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            let client = DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
            client.peer_uid().unwrap()
        });
        let server = listener.accept(Duration::from_secs(1)).unwrap();
        assert_eq!(server.peer_uid().unwrap(), effective_uid());
        assert_eq!(client.join().unwrap(), effective_uid());
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn daemon_ipc_removes_only_owned_stale_socket_and_reconnects() {
        let paths = test_paths("stale");
        fs::create_dir(&paths.directory).unwrap();
        fs::set_permissions(
            &paths.directory,
            fs::Permissions::from_mode(RUNTIME_DIRECTORY_MODE),
        )
        .unwrap();
        let stale = UnixListener::bind(&paths.socket).unwrap();
        drop(stale);
        let listener = DaemonListener::bind(&paths).unwrap();

        let (started_tx, started_rx) = mpsc::channel();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            started_tx.send(()).unwrap();
            DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap()
        });
        started_rx.recv().unwrap();
        let _server = listener.accept(Duration::from_secs(1)).unwrap();
        let _client = client.join().unwrap();
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn daemon_ipc_rejects_unsafe_paths_and_reports_unavailable_timeout() {
        assert!(matches!(
            runtime_paths_for(PathBuf::from("relative"), effective_uid()),
            Err(IpcError::RelativeRuntimePath(_))
        ));
        let paths = test_paths("unsafe");
        fs::create_dir(&paths.directory).unwrap();
        fs::set_permissions(&paths.directory, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            DaemonListener::bind(&paths),
            Err(IpcError::UnsafeRuntimePath { .. })
        ));
        fs::set_permissions(
            &paths.directory,
            fs::Permissions::from_mode(RUNTIME_DIRECTORY_MODE),
        )
        .unwrap();
        fs::write(&paths.socket, b"not a socket").unwrap();
        assert!(matches!(
            DaemonListener::bind(&paths),
            Err(IpcError::UnsafeRuntimePath { .. })
        ));
        fs::remove_file(&paths.socket).unwrap();
        std::os::unix::fs::symlink("/tmp", &paths.socket).unwrap();
        assert!(matches!(
            DaemonListener::bind(&paths),
            Err(IpcError::UnsafeRuntimePath { .. })
        ));
        fs::remove_file(&paths.socket).unwrap();
        assert!(matches!(
            DaemonConnection::connect(&paths, Duration::from_millis(20)),
            Err(IpcError::Unavailable { .. })
        ));
        cleanup(&paths);
    }

    #[test]
    fn daemon_ipc_read_timeout_and_message_bound_are_typed() {
        let paths = test_paths("limits");
        let listener = DaemonListener::bind(&paths).unwrap();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            let mut connection =
                DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
            connection
                .stream
                .write_all(&((MAX_FRAME_BYTES as u32 + 1).to_be_bytes()))
                .unwrap();
        });
        let mut server = listener.accept(Duration::from_secs(1)).unwrap();
        server.set_timeout(Some(Duration::from_millis(50))).unwrap();
        assert!(matches!(
            server.receive::<ClientMessage>(),
            Err(IpcError::Protocol(DaemonProtocolError::TooLarge))
        ));
        client.join().unwrap();

        let waiting_paths = paths.clone();
        let waiting_client = thread::spawn(move || {
            DaemonConnection::connect(&waiting_paths, Duration::from_secs(1)).unwrap()
        });
        let mut waiting_server = listener.accept(Duration::from_secs(1)).unwrap();
        waiting_server
            .set_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        assert!(matches!(
            waiting_server.receive::<ClientMessage>(),
            Err(IpcError::Timeout(_))
        ));
        drop(waiting_server);
        let _ = waiting_client.join().unwrap();
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn daemon_ipc_configures_read_and_write_deadlines_independently() {
        let paths = test_paths("independent-timeouts");
        let listener = DaemonListener::bind(&paths).unwrap();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap()
        });
        let server = listener.accept(Duration::from_secs(1)).unwrap();
        let read_timeout = Duration::from_millis(20);
        let write_timeout = Duration::from_secs(2);

        server.set_read_timeout(Some(read_timeout)).unwrap();
        server.set_write_timeout(Some(write_timeout)).unwrap();

        assert_eq!(server.stream.read_timeout().unwrap(), Some(read_timeout));
        assert_eq!(server.stream.write_timeout().unwrap(), Some(write_timeout));
        drop(client.join().unwrap());
        drop(listener);
        cleanup(&paths);
    }
}
