use std::{path::PathBuf, time::Duration};

use thiserror::Error;
use yoctui_app::DaemonClientSnapshot;
use yoctui_protocol::daemon::{RequestId, TerminalDimensions};

use crate::client_transport::{ClientTransportError, DaemonClientTransport};

mod attach;
mod effect_inputs;
mod effect_routing;
mod replica;
mod terminal_control;

#[cfg(test)]
use attach::random_client_id;
#[cfg(test)]
use effect_routing::daemon_command_for_effect;
#[cfg(test)]
use terminal_control::{prefix_daemon_command, wire_terminal_kind};

pub(crate) const MAX_EVENTS_PER_POLL: usize = 64;
pub(crate) const DAEMON_RECONNECT_INTERVAL: Duration = Duration::from_secs(1);
// Before terminal setup, allow a healthy large snapshot to finish before choosing
// local adapters. This must not be used by the blocking in-loop reconnect path.
pub(crate) const INITIAL_DAEMON_ATTACH_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_POLL_DURATION: Duration = Duration::from_millis(8);

pub struct InteractiveDaemonRuntime {
    transport: DaemonClientTransport,
    replica: DaemonClientSnapshot,
    local_build_dir: Option<PathBuf>,
    next_request: u64,
    last_pty_resize: Option<(u64, u64, TerminalDimensions)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEffectRoute {
    Daemon(RequestId),
    ClientLocal,
}

#[derive(Debug, Error)]
pub enum ClientRuntimeError {
    #[error(transparent)]
    Transport(#[from] ClientTransportError),
    #[error(transparent)]
    Replica(#[from] yoctui_app::DaemonClientSyncError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("random client identity was zero")]
    InvalidRandomIdentity,
    #[error("daemon has no active job to cancel")]
    NoActiveDaemonJob,
    #[error("daemon request ID space exhausted")]
    RequestSpaceExhausted,
    #[error("terminal viewport offset exceeds the wire range")]
    InvalidTerminalViewport,
    #[error("Raw execution request could not be encoded: {0}")]
    RawExecution(String),
    #[error("no running PTY session is available")]
    MissingPtySession,
    #[error("authoritative build directory is unavailable")]
    MissingBuildDirectory,
    #[error("authoritative SDK deploy root is unavailable")]
    MissingSdkDeployRoot,
    #[error("authoritative SDK tool root is unavailable")]
    MissingSdkWorkspaceRoot,
    #[error("runqemu capability is unavailable for the selected image")]
    MissingQemuCapability,
    #[error("Wic capability is unavailable")]
    MissingWicCapability,
    #[error("QA layer session is unavailable")]
    MissingQaLayerSession,
    #[error("security session is unavailable")]
    MissingSecuritySession,
    #[error("maintenance tool is unavailable")]
    MissingMaintenanceTool,
}

#[cfg(test)]
#[path = "tests/client_runtime/mod.rs"]
mod tests;
