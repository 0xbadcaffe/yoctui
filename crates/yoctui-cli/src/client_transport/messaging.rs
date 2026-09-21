use std::time::Duration;

use yoctui_protocol::{
    daemon::{
        ClientLayoutEvent, ClientMessage, CommandRequest, PtyInput, PtyResize, PtyViewport,
        ServerMessage, Subscription,
    },
    daemon_ipc::IpcError,
};

use super::attachment::validate_snapshot_instance;
use super::{ClientServerEvent, ClientTransportError, ClientTransportState, DaemonClientTransport};

impl DaemonClientTransport {
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

    pub fn pty_input(&mut self, input: PtyInput) -> Result<(), ClientTransportError> {
        self.require_attached()?;
        self.send(&ClientMessage::PtyInput(input))
    }

    pub fn pty_resize(&mut self, resize: PtyResize) -> Result<(), ClientTransportError> {
        self.require_attached()?;
        self.send(&ClientMessage::PtyResize(resize))
    }

    pub fn pty_viewport(&mut self, viewport: PtyViewport) -> Result<(), ClientTransportError> {
        self.require_attached()?;
        self.send(&ClientMessage::PtyViewport(viewport))
    }

    pub fn pty_layout(&mut self, event: ClientLayoutEvent) -> Result<(), ClientTransportError> {
        self.require_attached()?;
        self.send(&ClientMessage::Layout { event })
    }

    pub fn receive(&mut self) -> Result<ClientServerEvent, ClientTransportError> {
        self.require_attached()?;
        loop {
            return match self.receive_message()? {
                ServerMessage::Snapshot(snapshot) => {
                    validate_snapshot_instance(&self.hello, &snapshot)?;
                    Ok(ClientServerEvent::Snapshot(Box::new(snapshot)))
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
    pub(super) fn require_attached(&self) -> Result<(), ClientTransportError> {
        if self.state != ClientTransportState::Attached {
            return Err(ClientTransportError::InvalidState {
                expected: ClientTransportState::Attached,
                actual: self.state,
            });
        }
        Ok(())
    }

    pub(super) fn send(&mut self, message: &ClientMessage) -> Result<(), ClientTransportError> {
        self.connection
            .as_mut()
            .ok_or(ClientTransportError::Disconnected)?
            .send(message)
            .map_err(ClientTransportError::from)
    }

    pub(super) fn receive_message(&mut self) -> Result<ServerMessage, ClientTransportError> {
        self.connection
            .as_mut()
            .ok_or(ClientTransportError::Disconnected)?
            .receive()
            .map_err(ClientTransportError::from)
    }
}
