use std::{collections::BTreeSet, time::Duration};

use yoctui_protocol::{
    daemon::{
        Capability, ClientHello, ClientId, ClientMessage, DaemonHello, DaemonInstanceId,
        MAX_FRAME_BYTES, ProtocolVersion, ServerMessage, negotiate_capabilities, negotiate_version,
    },
    daemon_ipc::{DaemonConnection, RuntimePaths, runtime_paths},
};

use super::{
    ClientTransportError, ClientTransportState, DaemonClientTransport, MAX_CLIENT_NAME_BYTES,
    MAX_SOCKET_DISCOVERY_WAIT,
};

impl DaemonClientTransport {
    pub fn connect(
        client_id: ClientId,
        client_name: String,
        timeout: Duration,
    ) -> Result<Self, ClientTransportError> {
        Self::connect_at(&runtime_paths()?, client_id, client_name, timeout)
    }

    pub fn connect_at(
        paths: &RuntimePaths,
        client_id: ClientId,
        client_name: String,
        timeout: Duration,
    ) -> Result<Self, ClientTransportError> {
        validate_client(client_id, &client_name)?;
        // A generous snapshot read budget must not make an absent daemon delay
        // disconnected startup for the same duration. Retain the short discovery wait.
        let mut connection =
            DaemonConnection::connect(paths, timeout.min(MAX_SOCKET_DISCOVERY_WAIT))?;
        connection.set_timeout(Some(timeout))?;
        let requested = requested_capabilities();
        connection.send(&ClientMessage::Hello(ClientHello {
            minimum_version: ProtocolVersion::CURRENT,
            maximum_version: ProtocolVersion::CURRENT,
            client_id,
            client_name: client_name.clone(),
            capabilities: requested.clone(),
        }))?;
        let message: ServerMessage = connection.receive()?;
        let ServerMessage::Hello(hello) = message else {
            return Err(ClientTransportError::ExpectedHello(Box::new(message)));
        };
        validate_hello(&hello, &requested)?;
        Ok(Self {
            connection: Some(connection),
            client_id,
            client_name,
            hello,
            state: ClientTransportState::Negotiated,
        })
    }

    pub fn hello(&self) -> &DaemonHello {
        &self.hello
    }

    pub fn state(&self) -> ClientTransportState {
        self.state
    }
    pub fn reconnect(
        self,
        timeout: Duration,
    ) -> Result<DaemonClientTransport, ClientTransportError> {
        Self::connect(self.client_id, self.client_name, timeout)
    }

    pub fn reconnect_at(
        self,
        paths: &RuntimePaths,
        timeout: Duration,
    ) -> Result<DaemonClientTransport, ClientTransportError> {
        Self::connect_at(paths, self.client_id, self.client_name, timeout)
    }
}

pub(super) fn requested_capabilities() -> Vec<Capability> {
    vec![
        Capability::StateSnapshots,
        Capability::IncrementalEvents,
        Capability::EventReplay,
        Capability::BackgroundJobs,
        Capability::BitBakeLifecycle,
        Capability::PtySessions,
        Capability::PtyWriterLease,
        Capability::RawExecution,
        Capability::GracefulShutdown,
    ]
}

pub(super) fn validate_client(client_id: ClientId, name: &str) -> Result<(), ClientTransportError> {
    if client_id.0 == [0; 16]
        || name.is_empty()
        || name.len() > MAX_CLIENT_NAME_BYTES
        || name.chars().any(char::is_control)
    {
        return Err(ClientTransportError::InvalidClientIdentity);
    }
    Ok(())
}

pub(super) fn validate_hello(
    hello: &DaemonHello,
    requested: &[Capability],
) -> Result<(), ClientTransportError> {
    if hello.selected_version != ProtocolVersion::CURRENT {
        return Err(yoctui_protocol::daemon::DaemonProtocolError::IncompatibleVersion.into());
    }
    negotiate_version(
        ProtocolVersion::CURRENT,
        ProtocolVersion::CURRENT,
        hello.selected_version,
    )?;
    if hello.daemon_instance_id == DaemonInstanceId([0; 16]) || hello.boot_id.is_empty() {
        return Err(ClientTransportError::InvalidDaemonIdentity);
    }
    let negotiated = negotiate_capabilities(requested, &hello.capabilities)?;
    for required in [Capability::StateSnapshots, Capability::IncrementalEvents] {
        if !negotiated.contains(&required) {
            return Err(ClientTransportError::MissingCapability(required));
        }
    }
    let limits = hello.limits;
    if limits.maximum_frame_bytes == 0
        || limits.maximum_frame_bytes as usize > MAX_FRAME_BYTES
        || limits.maximum_snapshot_bytes == 0
        || limits.maximum_snapshot_bytes > limits.maximum_frame_bytes
        || limits.maximum_pending_requests == 0
        || limits.maximum_queue_depth == 0
        || limits.maximum_terminal_rows == 0
        || limits.maximum_terminal_columns == 0
    {
        return Err(ClientTransportError::InvalidLimits);
    }
    let unique = hello.capabilities.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != hello.capabilities.len() {
        return Err(ClientTransportError::DuplicateCapabilities);
    }
    Ok(())
}
