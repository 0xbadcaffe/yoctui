use std::{collections::BTreeSet, time::Duration};

use thiserror::Error;
use yoctui_protocol::{
    daemon::{
        Capability, ClientHello, ClientId, ClientMessage, CommandRequest, DaemonHello,
        DaemonInstanceId, DaemonSnapshot, MAX_FRAME_BYTES, ProtocolFailure, ProtocolVersion,
        ResumeCursor, SequencedEvent, ServerMessage, Subscription, WorkspaceIdentity,
        negotiate_capabilities, negotiate_version,
    },
    daemon_ipc::{DaemonConnection, IpcError, RuntimePaths, runtime_paths},
};

const MAX_CLIENT_NAME_BYTES: usize = 128;

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
    Snapshot(DaemonSnapshot),
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
        let mut connection = DaemonConnection::connect(paths, timeout)?;
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

    pub fn attach(
        &mut self,
        workspace: Option<WorkspaceIdentity>,
        subscription: Subscription,
        resume: Option<ResumeCursor>,
    ) -> Result<ClientAttachResult, ClientTransportError> {
        if self.state != ClientTransportState::Negotiated {
            return Err(ClientTransportError::InvalidState {
                expected: ClientTransportState::Negotiated,
                actual: self.state,
            });
        }
        self.send(&ClientMessage::Attach {
            workspace,
            subscription,
            resume,
        })?;
        let mut replayed_events = Vec::new();
        let mut replacement_reason = None;
        loop {
            match self.receive_message()? {
                ServerMessage::Event(event) => replayed_events.push(event),
                ServerMessage::ResyncRequired { reason, .. } => {
                    replacement_reason = Some(reason);
                }
                ServerMessage::Ping { nonce, .. } => {
                    self.send(&ClientMessage::Pong { nonce })?;
                }
                ServerMessage::Attached {
                    snapshot,
                    replayed_through,
                } => {
                    validate_attached(&self.hello, &snapshot, replayed_through)?;
                    self.state = ClientTransportState::Attached;
                    return Ok(ClientAttachResult {
                        snapshot,
                        replayed_events,
                        replacement_reason,
                    });
                }
                ServerMessage::Error(failure) => {
                    return Err(ClientTransportError::ProtocolFailure(failure));
                }
                message => return Err(ClientTransportError::Unexpected(Box::new(message))),
            }
        }
    }

    pub fn subscribe(&mut self, subscription: Subscription) -> Result<(), ClientTransportError> {
        self.require_attached()?;
        self.send(&ClientMessage::Subscribe { subscription })
    }

    pub fn unsubscribe(&mut self, subscription: Subscription) -> Result<(), ClientTransportError> {
        self.require_attached()?;
        self.send(&ClientMessage::Unsubscribe { subscription })
    }

    pub fn command(&mut self, request: CommandRequest) -> Result<(), ClientTransportError> {
        self.require_attached()?;
        self.send(&ClientMessage::Command(request))
    }

    pub fn receive(&mut self) -> Result<ClientServerEvent, ClientTransportError> {
        self.require_attached()?;
        loop {
            return match self.receive_message()? {
                ServerMessage::Snapshot(snapshot) => {
                    validate_snapshot_instance(&self.hello, &snapshot)?;
                    Ok(ClientServerEvent::Snapshot(snapshot))
                }
                ServerMessage::Event(event) => Ok(ClientServerEvent::Event(event)),
                ServerMessage::CommandResult(result) => {
                    Ok(ClientServerEvent::CommandResult(result))
                }
                ServerMessage::ResyncRequired {
                    reason,
                    current_sequence,
                } => Ok(ClientServerEvent::ResyncRequired {
                    reason,
                    current_sequence,
                }),
                ServerMessage::Ping { nonce, .. } => {
                    self.send(&ClientMessage::Pong { nonce })?;
                    continue;
                }
                ServerMessage::ShuttingDown => Ok(ClientServerEvent::ShuttingDown),
                ServerMessage::Error(failure) => {
                    Err(ClientTransportError::ProtocolFailure(failure))
                }
                message => Err(ClientTransportError::Unexpected(Box::new(message))),
            };
        }
    }

    pub fn try_receive(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<ClientServerEvent>, ClientTransportError> {
        self.require_attached()?;
        self.connection
            .as_ref()
            .ok_or(ClientTransportError::Disconnected)?
            .set_timeout(Some(timeout))?;
        match self.receive() {
            Ok(event) => Ok(Some(event)),
            Err(ClientTransportError::Ipc(IpcError::Timeout(_))) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn detach(&mut self) -> Result<(), ClientTransportError> {
        self.require_attached()?;
        self.send(&ClientMessage::Detach)?;
        loop {
            match self.receive_message()? {
                ServerMessage::Ping { nonce, .. } => self.send(&ClientMessage::Pong { nonce })?,
                ServerMessage::Detaching => {
                    self.connection.take();
                    self.state = ClientTransportState::Disconnected;
                    return Ok(());
                }
                ServerMessage::Error(failure) => {
                    return Err(ClientTransportError::ProtocolFailure(failure));
                }
                message => return Err(ClientTransportError::Unexpected(Box::new(message))),
            }
        }
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

    fn require_attached(&self) -> Result<(), ClientTransportError> {
        if self.state != ClientTransportState::Attached {
            return Err(ClientTransportError::InvalidState {
                expected: ClientTransportState::Attached,
                actual: self.state,
            });
        }
        Ok(())
    }

    fn send(&mut self, message: &ClientMessage) -> Result<(), ClientTransportError> {
        self.connection
            .as_mut()
            .ok_or(ClientTransportError::Disconnected)?
            .send(message)
            .map_err(ClientTransportError::from)
    }

    fn receive_message(&mut self) -> Result<ServerMessage, ClientTransportError> {
        self.connection
            .as_mut()
            .ok_or(ClientTransportError::Disconnected)?
            .receive()
            .map_err(ClientTransportError::from)
    }
}

fn requested_capabilities() -> Vec<Capability> {
    vec![
        Capability::StateSnapshots,
        Capability::IncrementalEvents,
        Capability::EventReplay,
        Capability::BackgroundJobs,
        Capability::BitBakeLifecycle,
        Capability::PtySessions,
        Capability::PtyWriterLease,
        Capability::GracefulShutdown,
    ]
}

fn validate_client(client_id: ClientId, name: &str) -> Result<(), ClientTransportError> {
    if client_id.0 == [0; 16]
        || name.is_empty()
        || name.len() > MAX_CLIENT_NAME_BYTES
        || name.chars().any(char::is_control)
    {
        return Err(ClientTransportError::InvalidClientIdentity);
    }
    Ok(())
}

fn validate_hello(
    hello: &DaemonHello,
    requested: &[Capability],
) -> Result<(), ClientTransportError> {
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

fn validate_attached(
    hello: &DaemonHello,
    snapshot: &DaemonSnapshot,
    replayed_through: u64,
) -> Result<(), ClientTransportError> {
    validate_snapshot_instance(hello, snapshot)?;
    if snapshot.sequence != replayed_through {
        return Err(ClientTransportError::InvalidAttachWatermark {
            snapshot: snapshot.sequence,
            replayed_through,
        });
    }
    Ok(())
}

fn validate_snapshot_instance(
    hello: &DaemonHello,
    snapshot: &DaemonSnapshot,
) -> Result<(), ClientTransportError> {
    if snapshot.daemon_instance_id != hello.daemon_instance_id {
        return Err(ClientTransportError::DaemonInstanceChanged);
    }
    Ok(())
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
mod tests {
    use std::{fs, os::unix::fs::PermissionsExt, thread};

    use super::*;
    use yoctui_protocol::{
        daemon::{
            BitBakeState, CommandOutcome, CommandResult, DaemonCommand, LifecycleState,
            ProjectProfileSummary, ProtocolLimits, RequestId,
        },
        daemon_ipc::{DaemonListener, runtime_paths_for},
    };

    fn snapshot(instance: DaemonInstanceId) -> DaemonSnapshot {
        DaemonSnapshot {
            daemon_instance_id: instance,
            sequence: 0,
            generation: 0,
            workspace: None,
            project_profile: ProjectProfileSummary::NotLoaded,
            bitbake: BitBakeState {
                lifecycle: LifecycleState::Disconnected,
                version: None,
                capabilities: Vec::new(),
                diagnostic: None,
            },
            compatibility: None,
            jobs: Vec::new(),
            pty_sessions: Vec::new(),
            clients: Vec::new(),
            recent_logs: Vec::new(),
            build_events: Vec::new(),
            recovery_warnings: Vec::new(),
        }
    }

    fn hello(instance: DaemonInstanceId) -> DaemonHello {
        DaemonHello {
            selected_version: ProtocolVersion::CURRENT,
            daemon_instance_id: instance,
            boot_id: "boot-test".into(),
            capabilities: vec![
                Capability::StateSnapshots,
                Capability::IncrementalEvents,
                Capability::GracefulShutdown,
            ],
            limits: ProtocolLimits {
                maximum_frame_bytes: MAX_FRAME_BYTES as u32,
                maximum_snapshot_bytes: MAX_FRAME_BYTES as u32,
                maximum_pending_requests: 8,
                maximum_queue_depth: 16,
                maximum_terminal_rows: 512,
                maximum_terminal_columns: 512,
                maximum_clients: 32,
                maximum_pty_sessions: 64,
                maximum_scrollback_lines: 100_000,
                maximum_utility_output_bytes: 4 * 1024 * 1024,
            },
        }
    }

    #[test]
    fn client_transport_negotiates_attaches_correlates_and_detaches() {
        let root = std::env::temp_dir().join(format!(
            "yoctui-client-transport-{}-{}",
            std::process::id(),
            std::time::SystemTime::UNIX_EPOCH
                .elapsed()
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let paths = runtime_paths_for(root.clone(), unsafe { libc::geteuid() }).unwrap();
        let server_paths = paths.clone();
        let instance = DaemonInstanceId([9; 16]);
        let server = thread::spawn(move || {
            let listener = DaemonListener::bind(&server_paths).unwrap();
            let mut connection = listener.accept(Duration::from_secs(2)).unwrap();
            assert!(matches!(
                connection.receive::<ClientMessage>().unwrap(),
                ClientMessage::Hello(_)
            ));
            connection
                .send(&ServerMessage::Hello(hello(instance)))
                .unwrap();
            assert!(matches!(
                connection.receive::<ClientMessage>().unwrap(),
                ClientMessage::Attach { .. }
            ));
            connection
                .send(&ServerMessage::Attached {
                    snapshot: snapshot(instance),
                    replayed_through: 0,
                })
                .unwrap();
            assert!(matches!(
                connection.receive::<ClientMessage>().unwrap(),
                ClientMessage::Subscribe { .. }
            ));
            assert!(matches!(
                connection.receive::<ClientMessage>().unwrap(),
                ClientMessage::Unsubscribe { .. }
            ));
            let ClientMessage::Command(request) = connection.receive::<ClientMessage>().unwrap()
            else {
                panic!("expected command");
            };
            connection
                .send(&ServerMessage::Ping {
                    nonce: 44,
                    deadline_unix_ms: 0,
                })
                .unwrap();
            assert_eq!(
                connection.receive::<ClientMessage>().unwrap(),
                ClientMessage::Pong { nonce: 44 }
            );
            connection
                .send(&ServerMessage::CommandResult(CommandResult {
                    request_id: request.request_id,
                    outcome: CommandOutcome::Completed,
                }))
                .unwrap();
            assert_eq!(
                connection.receive::<ClientMessage>().unwrap(),
                ClientMessage::Detach
            );
            connection.send(&ServerMessage::Detaching).unwrap();

            let mut connection = listener.accept(Duration::from_secs(2)).unwrap();
            assert!(matches!(
                connection.receive::<ClientMessage>().unwrap(),
                ClientMessage::Hello(_)
            ));
            connection
                .send(&ServerMessage::Hello(hello(instance)))
                .unwrap();
            assert!(matches!(
                connection.receive::<ClientMessage>().unwrap(),
                ClientMessage::Attach {
                    resume: Some(ResumeCursor {
                        last_sequence: 0,
                        ..
                    }),
                    ..
                }
            ));
            connection
                .send(&ServerMessage::Attached {
                    snapshot: snapshot(instance),
                    replayed_through: 0,
                })
                .unwrap();
            assert_eq!(
                connection.receive::<ClientMessage>().unwrap(),
                ClientMessage::Detach
            );
            connection.send(&ServerMessage::Detaching).unwrap();
        });
        while !paths.socket.exists() {
            thread::yield_now();
        }
        let mut client = DaemonClientTransport::connect_at(
            &paths,
            ClientId([7; 16]),
            "terminal-one".into(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(client.hello().daemon_instance_id, instance);
        let attached = client
            .attach(
                None,
                Subscription {
                    state: true,
                    jobs: true,
                    logs: true,
                    pty_sessions: Vec::new(),
                },
                None,
            )
            .unwrap();
        assert_eq!(attached.snapshot.daemon_instance_id, instance);
        let subscription = Subscription {
            state: false,
            jobs: false,
            logs: true,
            pty_sessions: Vec::new(),
        };
        client.subscribe(subscription.clone()).unwrap();
        client.unsubscribe(subscription).unwrap();
        client
            .command(CommandRequest {
                request_id: RequestId(12),
                expected_generation: Some(0),
                command: DaemonCommand::PrepareShutdown,
            })
            .unwrap();
        assert!(matches!(
            client.receive().unwrap(),
            ClientServerEvent::CommandResult(CommandResult {
                request_id: RequestId(12),
                ..
            })
        ));
        client.detach().unwrap();
        assert_eq!(client.state(), ClientTransportState::Disconnected);
        let mut client = client.reconnect_at(&paths, Duration::from_secs(2)).unwrap();
        client
            .attach(
                None,
                Subscription {
                    state: true,
                    jobs: true,
                    logs: false,
                    pty_sessions: Vec::new(),
                },
                Some(ResumeCursor {
                    daemon_instance_id: instance,
                    last_sequence: 0,
                }),
            )
            .unwrap();
        client.detach().unwrap();
        let _default_connect = DaemonClientTransport::connect;
        let _default_reconnect = DaemonClientTransport::reconnect;
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn client_transport_rejects_incompatible_or_unbounded_hello() {
        assert!(matches!(
            validate_client(ClientId([0; 16]), "client"),
            Err(ClientTransportError::InvalidClientIdentity)
        ));
        let mut invalid = hello(DaemonInstanceId([1; 16]));
        invalid.limits.maximum_frame_bytes = (MAX_FRAME_BYTES as u32).saturating_add(1);
        assert!(matches!(
            validate_hello(&invalid, &requested_capabilities()),
            Err(ClientTransportError::InvalidLimits)
        ));
        invalid = hello(DaemonInstanceId([1; 16]));
        invalid
            .capabilities
            .retain(|item| *item != Capability::IncrementalEvents);
        assert!(matches!(
            validate_hello(&invalid, &requested_capabilities()),
            Err(ClientTransportError::MissingCapability(
                Capability::IncrementalEvents
            ))
        ));
    }
}
