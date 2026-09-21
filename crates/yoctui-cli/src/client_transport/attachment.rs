use yoctui_protocol::daemon::{
    ClientMessage, DaemonHello, DaemonSnapshot, ResumeCursor, ServerMessage, Subscription,
    WorkspaceIdentity,
};

use super::{
    ClientAttachResult, ClientTransportError, ClientTransportState, DaemonClientTransport,
};

impl DaemonClientTransport {
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

pub(super) fn validate_snapshot_instance(
    hello: &DaemonHello,
    snapshot: &DaemonSnapshot,
) -> Result<(), ClientTransportError> {
    if snapshot.daemon_instance_id != hello.daemon_instance_id {
        return Err(ClientTransportError::DaemonInstanceChanged);
    }
    Ok(())
}
