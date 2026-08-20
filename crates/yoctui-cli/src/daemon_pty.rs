use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender},
    time::{Duration, Instant},
};

use yoctui_model::{
    PtyClientId, PtyCommandIdentity, PtyDimensions, PtySessionId, PtySessionKind, PtySessionSpec,
    PtyWorkspaceContext,
};
use yoctui_protocol::daemon::{
    MAX_DAEMON_PTY_SESSIONS, MAX_PTY_OUTPUT_EVENT_BYTES, PtyCommand, PtyKind, TerminalDimensions,
};

use crate::pty_attach::{DaemonPtySession, PtyAttachEvent};

const PTY_SCREEN_MIN_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Debug)]
enum Control {
    Attach(PtyClientId),
    Detach(PtyClientId),
    Take(PtyClientId, u64),
    Release(PtyClientId, u64),
    Input(PtyClientId, u64, Vec<u8>),
    Resize(PtyClientId, u64, PtyDimensions),
    Terminate,
}

#[derive(Debug)]
enum Response {
    Epoch(u64),
    Unit,
}

type ControlReply = SyncSender<Result<Response, String>>;
type ControlMessage = (Control, ControlReply);

#[derive(Debug)]
pub enum DaemonPtyEvent {
    Started {
        session_id: PtySessionId,
        snapshot: yoctui_protocol::daemon::PtySessionSummary,
    },
    Output {
        session_id: PtySessionId,
        bytes: Vec<u8>,
        screen: Option<yoctui_protocol::daemon::PtyScreenSnapshot>,
    },
    Exited {
        session_id: PtySessionId,
        exit_code: Option<i32>,
        screen: Option<yoctui_protocol::daemon::PtyScreenSnapshot>,
    },
    Lost {
        session_id: PtySessionId,
        message: String,
    },
    Changed {
        session_id: PtySessionId,
        snapshot: yoctui_protocol::daemon::PtySessionSummary,
    },
}

struct SessionHandle {
    control: tokio::sync::mpsc::UnboundedSender<ControlMessage>,
}

pub struct DaemonPtySupervisor {
    sessions: HashMap<PtySessionId, SessionHandle>,
    tx: tokio::sync::mpsc::UnboundedSender<DaemonPtyEvent>,
    rx: tokio::sync::mpsc::UnboundedReceiver<DaemonPtyEvent>,
}

impl Default for DaemonPtySupervisor {
    fn default() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            sessions: HashMap::new(),
            tx,
            rx,
        }
    }
}

impl DaemonPtySupervisor {
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
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| "PTY session ID space exhausted".to_string())?,
        );
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
        if id.0 == 0 || self.sessions.contains_key(&id) {
            return Err(format!("PTY session {} already exists or is invalid", id.0));
        }
        let spec = wire_spec(id, name, kind, cwd, command, dimensions)?;
        let (control_tx, mut control_rx): (
            tokio::sync::mpsc::UnboundedSender<ControlMessage>,
            tokio::sync::mpsc::UnboundedReceiver<ControlMessage>,
        ) = tokio::sync::mpsc::unbounded_channel();
        let event_tx = self.tx.clone();
        let session_id = spec.id;
        tokio::spawn(async move {
            let mut session = match DaemonPtySession::start(
                spec,
                inherited_environment(),
                2_000,
                Duration::from_secs(2),
            )
            .await
            {
                Ok(session) => session,
                Err(error) => {
                    let _ = event_tx.send(DaemonPtyEvent::Lost {
                        session_id,
                        message: error.to_string(),
                    });
                    return;
                }
            };
            if let Ok(snapshot) = session.snapshot(0) {
                let _ = event_tx.send(DaemonPtyEvent::Started {
                    session_id,
                    snapshot: snapshot_to_wire(&snapshot.listing),
                });
            }
            let mut last_screen_publish = Instant::now()
                .checked_sub(PTY_SCREEN_MIN_INTERVAL)
                .unwrap_or_else(Instant::now);
            loop {
                tokio::select! {
                    control = control_rx.recv() => {
                        let Some((control, response)) = control else { return; };
                        let result = match control {
                            Control::Attach(client) => session.attach(client).map(|_| Response::Unit),
                            Control::Detach(client) => session.detach(client).map(|_| Response::Unit),
                            Control::Take(client, epoch) => session.take_control(client, epoch).map(Response::Epoch),
                            Control::Release(client, epoch) => session.release_control(client, epoch).map(|_| Response::Unit),
                            Control::Input(client, epoch, bytes) => session.input(client, epoch, &bytes).await.map(|_| Response::Unit),
                            Control::Resize(client, epoch, dimensions) => session.resize(client, epoch, dimensions).map(|_| Response::Unit),
                            Control::Terminate => session.terminate().await.map(|_| Response::Unit),
                        };
                        let _ = response.send(result.map_err(|error| error.to_string()));
                        if let Ok(snapshot) = session.snapshot(0) {
                            let _ = event_tx.send(DaemonPtyEvent::Changed { session_id, snapshot: snapshot_to_wire(&snapshot.listing) });
                        }
                    }
                    event = session.next_event() => {
                        match event {
                            Ok(PtyAttachEvent::Started) => {}
                            Ok(PtyAttachEvent::Output { mut bytes, .. }) => {
                                bytes.truncate(MAX_PTY_OUTPUT_EVENT_BYTES);
                                let screen = (last_screen_publish.elapsed() >= PTY_SCREEN_MIN_INTERVAL)
                                    .then(|| session.snapshot(0).ok())
                                    .flatten()
                                    .map(|snapshot| terminal_to_wire(session_id, &snapshot.terminal));
                                if screen.is_some() {
                                    last_screen_publish = Instant::now();
                                }
                                let _ = event_tx.send(DaemonPtyEvent::Output { session_id, bytes, screen });
                            }
                            Ok(PtyAttachEvent::Exited(status)) => {
                                let code = match status { yoctui_model::PtyExitStatus::Code(code) => Some(code), yoctui_model::PtyExitStatus::Signal(_) => None };
                                let screen = session
                                    .snapshot(0)
                                    .ok()
                                    .map(|snapshot| terminal_to_wire(session_id, &snapshot.terminal));
                                let _ = event_tx.send(DaemonPtyEvent::Exited { session_id, exit_code: code, screen });
                                return;
                            }
                            Ok(PtyAttachEvent::Lost { message }) => {
                                let _ = event_tx.send(DaemonPtyEvent::Lost { session_id, message });
                                return;
                            }
                            Err(message) => {
                                let _ = event_tx.send(DaemonPtyEvent::Lost { session_id, message: message.to_string() });
                                return;
                            }
                        }
                    }
                }
            }
        });
        self.sessions.insert(
            id,
            SessionHandle {
                control: control_tx,
            },
        );
        Ok(())
    }

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
        rx.recv_timeout(Duration::from_secs(1))
            .map_err(|_| "PTY session command timed out".to_string())?
    }
}

fn wire_spec(
    id: PtySessionId,
    name: String,
    kind: PtyKind,
    cwd: String,
    command: PtyCommand,
    dimensions: TerminalDimensions,
) -> Result<PtySessionSpec, String> {
    let cwd = PathBuf::from(cwd);
    if !cwd.is_absolute() {
        return Err("PTY cwd must be absolute".into());
    }
    if dimensions.columns == 0
        || dimensions.rows == 0
        || dimensions.columns > 512
        || dimensions.rows > 512
    {
        return Err("PTY dimensions must be within 1..=512".into());
    }
    Ok(PtySessionSpec {
        id,
        name,
        kind: match kind {
            PtyKind::BuildShell => PtySessionKind::BuildShell,
            PtyKind::SourceShell => PtySessionKind::SourceShell,
            PtyKind::LayerShell => PtySessionKind::LayerShell,
            PtyKind::RecipeShell => PtySessionKind::RecipeShell,
            PtyKind::DevtoolShell => PtySessionKind::DevtoolShell,
            PtyKind::Devshell => PtySessionKind::Devshell,
            PtyKind::Menuconfig => PtySessionKind::Menuconfig,
            PtyKind::SdkShell => PtySessionKind::SdkShell,
            PtyKind::NativeShell => PtySessionKind::NativeShell,
            PtyKind::Utility => PtySessionKind::InteractiveTool,
        },
        cwd: cwd.clone(),
        command: PtyCommandIdentity {
            executable: command.program.into(),
            arguments: command.arguments,
        },
        dimensions: PtyDimensions {
            columns: dimensions.columns,
            rows: dimensions.rows,
        },
        restartable: true,
        workspace: PtyWorkspaceContext {
            source_dir: cwd.clone(),
            build_dir: cwd.clone(),
            authorized_context_roots: vec![cwd.clone()],
            owner_identity: "daemon".into(),
        },
    })
}

fn inherited_environment() -> BTreeMap<String, String> {
    std::env::vars().collect()
}

fn snapshot_to_wire(
    listing: &crate::pty_attach::PtyAttachListing,
) -> yoctui_protocol::daemon::PtySessionSummary {
    yoctui_protocol::daemon::PtySessionSummary {
        id: yoctui_protocol::daemon::PtySessionId(listing.id.0),
        name: listing.name.clone(),
        kind: match listing.kind {
            PtySessionKind::BuildShell => PtyKind::BuildShell,
            PtySessionKind::SourceShell => PtyKind::SourceShell,
            PtySessionKind::LayerShell => PtyKind::LayerShell,
            PtySessionKind::RecipeShell => PtyKind::RecipeShell,
            PtySessionKind::DevtoolShell => PtyKind::DevtoolShell,
            PtySessionKind::Devshell => PtyKind::Devshell,
            PtySessionKind::Menuconfig => PtyKind::Menuconfig,
            PtySessionKind::SdkShell => PtyKind::SdkShell,
            PtySessionKind::NativeShell => PtyKind::NativeShell,
            PtySessionKind::InteractiveTool | PtySessionKind::DeployShell => PtyKind::Utility,
        },
        cwd: String::new(),
        lifecycle: match listing.lifecycle {
            crate::pty_attach::PtyAttachLifecycle::Running => {
                yoctui_protocol::daemon::LifecycleState::Running
            }
            crate::pty_attach::PtyAttachLifecycle::Exited => {
                yoctui_protocol::daemon::LifecycleState::Exited
            }
            crate::pty_attach::PtyAttachLifecycle::Lost => {
                yoctui_protocol::daemon::LifecycleState::Lost
            }
        },
        dimensions: yoctui_protocol::daemon::TerminalDimensions {
            columns: listing.dimensions.columns,
            rows: listing.dimensions.rows,
        },
        writer: listing
            .writer
            .map(|client| yoctui_protocol::daemon::ClientId(client.0)),
        writer_epoch: listing.writer_epoch,
        viewers: listing.viewers as u16,
        exit_code: listing.exit_status.and_then(|status| match status {
            yoctui_model::PtyExitStatus::Code(code) => Some(code),
            yoctui_model::PtyExitStatus::Signal(_) => None,
        }),
        restartable: true,
    }
}

fn terminal_to_wire(
    session_id: PtySessionId,
    terminal: &yoctui_model::TerminalSnapshot,
) -> yoctui_protocol::daemon::PtyScreenSnapshot {
    let columns = usize::from(terminal.dimensions.columns);
    let rows = terminal
        .cells
        .chunks(columns)
        .map(|cells| {
            cells
                .iter()
                .filter(|cell| !cell.wide_continuation)
                .map(|cell| cell.contents.as_str())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect();
    yoctui_protocol::daemon::PtyScreenSnapshot {
        session_id: yoctui_protocol::daemon::PtySessionId(session_id.0),
        dimensions: yoctui_protocol::daemon::TerminalDimensions {
            columns: terminal.dimensions.columns,
            rows: terminal.dimensions.rows,
        },
        cursor_column: terminal.cursor.1,
        cursor_row: terminal.cursor.0,
        rows,
        scrollback_lines: terminal.max_scrollback_offset.min(u32::MAX as usize) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_limits_reject_oversized_pty_dimensions() {
        let command = PtyCommand {
            program: "/bin/sh".into(),
            arguments: Vec::new(),
            environment_profile_id: None,
        };
        let dimensions = TerminalDimensions {
            columns: 513,
            rows: 24,
        };
        assert!(
            wire_spec(
                PtySessionId(1),
                "bounded".into(),
                PtyKind::BuildShell,
                "/tmp".into(),
                command,
                dimensions,
            )
            .is_err()
        );
    }

    #[test]
    fn next_generation_pty_converts_typed_emulator_cells_without_ansi_leakage() {
        let mut emulator = yoctui_model::TerminalEmulator::new(
            PtyDimensions {
                columns: 12,
                rows: 3,
            },
            8,
        )
        .unwrap();
        emulator
            .process(b"\x1b[2J\x1b[1;1Hready\r\nprompt")
            .unwrap();
        let terminal = emulator.snapshot(0).unwrap();
        let screen = terminal_to_wire(PtySessionId(3), &terminal);
        assert_eq!(screen.session_id.0, 3);
        assert_eq!(screen.rows[0], "ready");
        assert_eq!(screen.rows[1], "prompt");
        assert!(screen.rows.iter().all(|row| !row.contains('\x1b')));
    }
}
