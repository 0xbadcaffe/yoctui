use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver},
};
use yoctui_bitbake::RawPtyCommandSpec;
use yoctui_model::{
    PtyClientId, PtyCommandIdentity, PtyDimensions, PtySessionId, PtySessionKind, PtySessionSpec,
    PtyWorkspaceContext,
};
use yoctui_protocol::daemon::{
    MAX_DAEMON_PTY_SESSIONS, MAX_PTY_INPUT_BYTES, PtyCommand, PtyKind, TerminalDimensions,
};

use super::{
    Control, ControlReply, DaemonPtyEvent, DaemonPtySupervisor, GENERIC_PTY_ID_LIMIT,
    PTY_CONTROL_RESPONSE_TIMEOUT, RAW_PTY_NAMESPACE, Response,
    request_validation::{validate_dimensions, wire_spec},
};

impl Default for DaemonPtySupervisor {
    fn default() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            sessions: HashMap::new(),
            next_generic_id: 1,
            tx,
            rx,
        }
    }
}

impl DaemonPtySupervisor {
    pub fn with_recovered_session_ids(ids: impl IntoIterator<Item = u64>) -> Self {
        let next_generic_id = ids
            .into_iter()
            .filter(|id| *id < GENERIC_PTY_ID_LIMIT)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
            .min(GENERIC_PTY_ID_LIMIT);
        Self {
            next_generic_id,
            ..Self::default()
        }
    }

    pub fn start_raw(
        &mut self,
        id: PtySessionId,
        command: &RawPtyCommandSpec,
        dimensions: TerminalDimensions,
    ) -> Result<(), String> {
        validate_dimensions(dimensions)?;
        let sequence = command
            .session_id()
            .as_str()
            .strip_prefix("raw-session:daemon-")
            .and_then(|sequence| sequence.parse::<u64>().ok())
            .filter(|sequence| *sequence > 0 && *sequence < GENERIC_PTY_ID_LIMIT)
            .ok_or_else(|| "Raw PTY session identity is invalid".to_string())?;
        if id.0 != RAW_PTY_NAMESPACE | sequence {
            return Err("Raw and daemon PTY session identities do not match".into());
        }
        let cwd = command.current_directory().to_path_buf();
        self.start_spec(PtySessionSpec {
            id,
            name: format!("Raw {}", command.request_id()),
            kind: PtySessionKind::InteractiveTool,
            cwd: cwd.clone(),
            command: PtyCommandIdentity {
                executable: command.executable().to_path_buf(),
                arguments: command.arguments().to_vec(),
            },
            dimensions: PtyDimensions {
                columns: dimensions.columns,
                rows: dimensions.rows,
            },
            restartable: false,
            workspace: PtyWorkspaceContext {
                source_dir: cwd.clone(),
                build_dir: cwd.clone(),
                authorized_context_roots: vec![cwd],
                owner_identity: command.session_id().as_str().into(),
            },
        })
    }

    pub fn start_new(
        &mut self,
        name: String,
        kind: PtyKind,
        cwd: String,
        command: PtyCommand,
        dimensions: TerminalDimensions,
    ) -> Result<PtySessionId, String> {
        if self.sessions.len() >= MAX_DAEMON_PTY_SESSIONS {
            return Err(format!(
                "PTY session limit reached ({MAX_DAEMON_PTY_SESSIONS})"
            ));
        }
        let id = PtySessionId(
            self.sessions
                .keys()
                .map(|id| id.0)
                .filter(|id| *id < GENERIC_PTY_ID_LIMIT)
                .max()
                .and_then(|id| id.checked_add(1))
                .unwrap_or(1)
                .max(self.next_generic_id),
        );
        if id.0 >= GENERIC_PTY_ID_LIMIT {
            return Err("generic PTY session ID space exhausted".into());
        }
        self.start(id, name, kind, cwd, command, dimensions)?;
        Ok(id)
    }

    pub fn start(
        &mut self,
        id: PtySessionId,
        name: String,
        kind: PtyKind,
        cwd: String,
        command: PtyCommand,
        dimensions: TerminalDimensions,
    ) -> Result<(), String> {
        let spec = wire_spec(id, name, kind, cwd, command, dimensions)?;
        self.start_spec(spec)?;
        if id.0 < GENERIC_PTY_ID_LIMIT {
            self.next_generic_id = id.0.saturating_add(1).min(GENERIC_PTY_ID_LIMIT);
        }
        Ok(())
    }
}

impl DaemonPtySupervisor {
    pub fn attach(&self, id: PtySessionId, client: PtyClientId) -> Result<(), String> {
        self.request(id, Control::Attach(client)).map(|_| ())
    }
    pub fn detach(&self, id: PtySessionId, client: PtyClientId) -> Result<(), String> {
        self.request(id, Control::Detach(client)).map(|_| ())
    }
    pub fn take(&self, id: PtySessionId, client: PtyClientId, epoch: u64) -> Result<u64, String> {
        match self.request(id, Control::Take(client, epoch))? {
            Response::Epoch(epoch) => Ok(epoch),
            Response::Unit => Err("PTY did not return a writer epoch".into()),
            Response::Screen(_) => Err("PTY returned a screen instead of a writer epoch".into()),
        }
    }
    pub fn release(&self, id: PtySessionId, client: PtyClientId, epoch: u64) -> Result<(), String> {
        self.request(id, Control::Release(client, epoch))
            .map(|_| ())
    }
    pub fn input(
        &self,
        id: PtySessionId,
        client: PtyClientId,
        epoch: u64,
        bytes: Vec<u8>,
    ) -> Result<(), String> {
        if bytes.is_empty() || bytes.len() > MAX_PTY_INPUT_BYTES {
            return Err(format!(
                "PTY input must contain 1..={MAX_PTY_INPUT_BYTES} bytes"
            ));
        }
        self.request(id, Control::Input(client, epoch, bytes))
            .map(|_| ())
    }
    pub fn resize(
        &self,
        id: PtySessionId,
        client: PtyClientId,
        epoch: u64,
        dimensions: PtyDimensions,
    ) -> Result<(), String> {
        self.request(id, Control::Resize(client, epoch, dimensions))
            .map(|_| ())
    }
    pub fn terminate(&self, id: PtySessionId) -> Result<(), String> {
        self.request(id, Control::Terminate).map(|_| ())
    }
    pub fn snapshot(
        &self,
        id: PtySessionId,
        scrollback_offset: usize,
    ) -> Result<yoctui_protocol::daemon::PtyScreenSnapshot, String> {
        match self.request(id, Control::Snapshot(scrollback_offset))? {
            Response::Screen(screen) => Ok(*screen),
            Response::Epoch(_) | Response::Unit => {
                Err("PTY did not return a terminal screen".into())
            }
        }
    }
    pub fn rename(&self, id: PtySessionId, name: String) -> Result<(), String> {
        self.request(id, Control::Rename(name)).map(|_| ())
    }
    pub fn close(&mut self, id: PtySessionId) -> Result<(), String> {
        self.sessions
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| format!("unknown PTY session {}", id.0))
    }
    pub fn disconnect_client(&self, client: PtyClientId) {
        for id in self.sessions.keys().copied().collect::<Vec<_>>() {
            let _ = self.detach(id, client);
        }
    }
    pub fn try_event(&mut self) -> Option<DaemonPtyEvent> {
        self.rx.try_recv().ok()
    }

    fn request(&self, id: PtySessionId, control: Control) -> Result<Response, String> {
        let handle = self
            .sessions
            .get(&id)
            .ok_or_else(|| format!("unknown PTY session {}", id.0))?;
        let (tx, rx): (ControlReply, Receiver<Result<Response, String>>) = mpsc::sync_channel(1);
        handle
            .control
            .send((control, tx))
            .map_err(|_| "PTY session is no longer active".to_string())?;
        rx.recv_timeout(PTY_CONTROL_RESPONSE_TIMEOUT)
            .map_err(|_| "PTY session command timed out".to_string())?
    }
}
