use std::{fs::File, io::Read, path::PathBuf, time::Duration};

use yoctui_app::DaemonClientSnapshot;
use yoctui_model::App;
use yoctui_protocol::daemon::{ClientId, Subscription};

use crate::client_transport::DaemonClientTransport;

use super::{ClientRuntimeError, InteractiveDaemonRuntime};

impl InteractiveDaemonRuntime {
    pub fn connect(app: &mut App, timeout: Duration) -> Result<Self, ClientRuntimeError> {
        let local_build_dir = app.workspace.build_dir.clone();
        let client_id = random_client_id()?;
        let mut transport =
            DaemonClientTransport::connect(client_id, "yoctui-ratatui".into(), timeout)?;
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
        restore_local_build_dir(app, local_build_dir.as_ref());
        app.terminal.client_id = Some(client_id.0);
        Ok(Self {
            transport,
            replica,
            local_build_dir,
            next_request: 1,
            last_pty_resize: None,
        })
    }
    pub fn detach(mut self, app: &mut App) -> Result<(), ClientRuntimeError> {
        self.transport.detach()?;
        self.replica.disconnect_app(app);
        Ok(())
    }
}

pub(super) fn restore_local_build_dir(app: &mut App, local_build_dir: Option<&PathBuf>) {
    if app.workspace.build_dir.is_none()
        && let Some(local_build_dir) = local_build_dir
    {
        app.workspace.build_dir = Some(local_build_dir.clone());
    }
}

pub(super) fn random_client_id() -> Result<ClientId, ClientRuntimeError> {
    let mut identity = [0_u8; 16];
    File::open("/dev/urandom")?.read_exact(&mut identity)?;
    if identity == [0; 16] {
        return Err(ClientRuntimeError::InvalidRandomIdentity);
    }
    Ok(ClientId(identity))
}
