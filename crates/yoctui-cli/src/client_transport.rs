use thiserror::Error;
use yoctui_protocol::{
    daemon::{
        Capability, ClientId, DaemonHello, DaemonSnapshot, ProtocolFailure, SequencedEvent,
        ServerMessage,
    },
    daemon_ipc::{DaemonConnection, IpcError},
};

mod attachment;
mod handshake;
mod messaging;

#[cfg(test)]
use handshake::{requested_capabilities, validate_client, validate_hello};

const MAX_CLIENT_NAME_BYTES: usize = 128;
const MAX_SOCKET_DISCOVERY_WAIT: std::time::Duration = std::time::Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientTransportState {
    Negotiated,
    Attached,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientAttachResult {
    pub snapshot: DaemonSnapshot,
    pub replayed_events: Vec<SequencedEvent>,
    pub replacement_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientServerEvent {
    Snapshot(Box<DaemonSnapshot>),
    Event(SequencedEvent),
    CommandResult(yoctui_protocol::daemon::CommandResult),
    ResyncRequired {
        reason: String,
        current_sequence: u64,
    },
    ShuttingDown,
}

#[derive(Debug)]
pub struct DaemonClientTransport {
    connection: Option<DaemonConnection>,
    client_id: ClientId,
    client_name: String,
    hello: DaemonHello,
    state: ClientTransportState,
}

#[derive(Debug, Error)]
pub enum ClientTransportError {
    #[error("invalid client identity")]
    InvalidClientIdentity,
    #[error("daemon returned an invalid identity")]
    InvalidDaemonIdentity,
    #[error("daemon is missing required capability {0:?}")]
    MissingCapability(Capability),
    #[error("daemon returned duplicate capabilities")]
    DuplicateCapabilities,
    #[error("daemon returned invalid negotiated limits")]
    InvalidLimits,
    #[error("expected daemon hello, received {0:?}")]
    ExpectedHello(Box<ServerMessage>),
    #[error("unexpected daemon message: {0:?}")]
    Unexpected(Box<ServerMessage>),
    #[error("daemon protocol failure: {0:?}")]
    ProtocolFailure(ProtocolFailure),
    #[error("client transport state mismatch: expected {expected:?}, got {actual:?}")]
    InvalidState {
        expected: ClientTransportState,
        actual: ClientTransportState,
    },
    #[error("daemon attach watermark mismatch: snapshot {snapshot}, replay {replayed_through}")]
    InvalidAttachWatermark {
        snapshot: u64,
        replayed_through: u64,
    },
    #[error("daemon instance changed during synchronization")]
    DaemonInstanceChanged,
    #[error("daemon client is disconnected")]
    Disconnected,
    #[error(transparent)]
    Ipc(#[from] IpcError),
    #[error(transparent)]
    Protocol(#[from] yoctui_protocol::daemon::DaemonProtocolError),
}

#[cfg(test)]
#[path = "tests/client_transport/mod.rs"]
mod tests;
