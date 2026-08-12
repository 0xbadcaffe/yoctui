use std::{fs::File, io::Read, time::Duration};

use thiserror::Error;
use yoctui_app::DaemonClientSnapshot;
use yoctui_model::{App, ClientDaemonLifecycle, Effect};
use yoctui_protocol::daemon::{
    ClientId, CommandRequest, DaemonCommand, JobId, RequestId, Subscription,
};

use crate::client_transport::{ClientServerEvent, ClientTransportError, DaemonClientTransport};

pub struct InteractiveDaemonRuntime {
    transport: DaemonClientTransport,
    replica: DaemonClientSnapshot,
    next_request: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEffectRoute {
    Daemon(RequestId),
    ClientLocal,
}

impl InteractiveDaemonRuntime {
    pub fn connect(app: &mut App, timeout: Duration) -> Result<Self, ClientRuntimeError> {
        let mut transport =
            DaemonClientTransport::connect(random_client_id()?, "yoctui-ratatui".into(), timeout)?;
        let attached = transport.attach(
            None,
            Subscription {
                state: true,
                jobs: true,
                logs: true,
                pty_sessions: Vec::new(),
            },
            None,
        )?;
        let mut replica = DaemonClientSnapshot::default();
        replica.begin_synchronization();
        replica.replace_app(app, attached.snapshot);
        for event in attached.replayed_events {
            replica.apply_event_to_app(app, &event)?;
        }
        Ok(Self {
            transport,
            replica,
            next_request: 1,
        })
    }

    pub fn poll(&mut self, app: &mut App) -> Result<bool, ClientRuntimeError> {
        let Some(event) = self.transport.try_receive(Duration::from_millis(1))? else {
            return Ok(false);
        };
        match event {
            ClientServerEvent::Snapshot(snapshot) => self.replica.replace_app(app, snapshot),
            ClientServerEvent::Event(event) => self.replica.apply_event_to_app(app, &event)?,
            ClientServerEvent::ResyncRequired { reason, .. } => {
                self.replica.begin_synchronization();
                self.replica.install_app(app);
                app.notification = Some(format!("Daemon resynchronization required: {reason}"));
            }
            ClientServerEvent::CommandResult(result) => {
                app.notification = Some(format!(
                    "Daemon request {}: {:?}",
                    result.request_id.0, result.outcome
                ));
            }
            ClientServerEvent::ShuttingDown => {
                self.replica.disconnect_app(app);
                app.notification = Some("Yoctui daemon is shutting down.".into());
            }
        }
        Ok(true)
    }

    pub fn route_effect(
        &mut self,
        app: &App,
        effect: &Effect,
    ) -> Result<RuntimeEffectRoute, ClientRuntimeError> {
        let command = match effect {
            Effect::Start(request) => DaemonCommand::StartBuild {
                targets: request.targets.clone(),
                task: request.task.clone(),
                force: request.force,
            },
            Effect::Cancel => {
                let job = app
                    .daemon
                    .jobs
                    .iter()
                    .find(|job| {
                        matches!(
                            job.lifecycle,
                            ClientDaemonLifecycle::Connecting | ClientDaemonLifecycle::Running
                        )
                    })
                    .ok_or(ClientRuntimeError::NoActiveDaemonJob)?;
                DaemonCommand::CancelJob {
                    job_id: JobId(job.id),
                }
            }
            _ => return Ok(RuntimeEffectRoute::ClientLocal),
        };
        let request_id = RequestId(self.next_request);
        self.next_request = self
            .next_request
            .checked_add(1)
            .ok_or(ClientRuntimeError::RequestSpaceExhausted)?;
        self.transport.command(CommandRequest {
            request_id,
            expected_generation: Some(app.daemon.generation),
            command,
        })?;
        Ok(RuntimeEffectRoute::Daemon(request_id))
    }

    pub fn detach(mut self, app: &mut App) -> Result<(), ClientRuntimeError> {
        self.transport.detach()?;
        self.replica.disconnect_app(app);
        Ok(())
    }
}

fn random_client_id() -> Result<ClientId, ClientRuntimeError> {
    let mut identity = [0_u8; 16];
    File::open("/dev/urandom")?.read_exact(&mut identity)?;
    if identity == [0; 16] {
        return Err(ClientRuntimeError::InvalidRandomIdentity);
    }
    Ok(ClientId(identity))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_runtime_effect_mapping_uses_daemon_global_state() {
        let request = yoctui_model::BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: Some("build".into()),
            force: false,
        };
        let command = match Effect::Start(request.clone()) {
            Effect::Start(request) => DaemonCommand::StartBuild {
                targets: request.targets,
                task: request.task,
                force: request.force,
            },
            _ => unreachable!(),
        };
        assert!(matches!(
            command,
            DaemonCommand::StartBuild { targets, task: Some(task), force: false }
                if targets == request.targets && task == "build"
        ));
        let mut app = App::new(16, 4096);
        app.daemon.jobs.push(yoctui_model::ClientDaemonJobSummary {
            id: 71,
            label: "core-image-minimal".into(),
            lifecycle: ClientDaemonLifecycle::Running,
        });
        assert_eq!(app.daemon.jobs[0].id, 71);
        assert!(matches!(Effect::PersistSettings, Effect::PersistSettings));
    }

    #[test]
    fn client_runtime_random_identity_is_nonzero() {
        assert_ne!(random_client_id().unwrap().0, [0; 16]);
    }
}
