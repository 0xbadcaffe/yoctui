//! Secure local Unix-domain transport for the daemon protocol.
use crate::daemon::{DaemonProtocolError, MAX_FRAME_BYTES, decode_frame, encode_frame};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    env, fs,
    io::{self, Read, Write},
    os::unix::{
        fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt},
        io::{AsRawFd, RawFd},
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

include!("daemon_ipc/runtime_listener.rs");
include!("daemon_ipc/connection.rs");
include!("daemon_ipc/path_security.rs");

#[cfg(test)]
#[path = "tests/daemon_ipc/mod.rs"]
mod tests;
