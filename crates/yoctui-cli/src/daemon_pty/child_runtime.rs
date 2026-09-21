use std::time::Instant;
use yoctui_model::PtySessionSpec;
use yoctui_protocol::daemon::{MAX_DAEMON_PTY_SESSIONS, MAX_PTY_OUTPUT_EVENT_BYTES};

use super::{
    Control, ControlMessage, DaemonPtyEvent, DaemonPtySupervisor, PTY_SCREEN_MIN_INTERVAL,
    PTY_TERMINATION_TIMEOUT, Response, SessionHandle,
    request_validation::inherited_environment,
    terminal_mapping::{snapshot_to_wire, terminal_to_wire},
};
use crate::pty_attach::{DaemonPtySession, PtyAttachEvent};

impl DaemonPtySupervisor {
    pub(super) fn start_spec(&mut self, spec: PtySessionSpec) -> Result<(), String> {
        let id = spec.id;
        if id.0 == 0 || self.sessions.contains_key(&id) {
            return Err(format!("PTY session {} already exists or is invalid", id.0));
        }
        if self.sessions.len() >= MAX_DAEMON_PTY_SESSIONS {
            return Err(format!(
                "PTY session limit reached ({MAX_DAEMON_PTY_SESSIONS})"
            ));
        }
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
                PTY_TERMINATION_TIMEOUT,
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
                        if matches!(&control, Control::Terminate) {
                            let result = session.terminate().await.map(|_| Response::Unit);
                            let snapshot = session.snapshot(0).ok();
                            let succeeded = result.is_ok();
                            let _ = response.send(result.map_err(|error| error.to_string()));
                            if succeeded {
                                let exit_code = snapshot.as_ref().and_then(|snapshot| {
                                    snapshot.listing.exit_status.and_then(|status| match status {
                                        yoctui_model::PtyExitStatus::Code(code) => Some(code),
                                        yoctui_model::PtyExitStatus::Signal(_) => None,
                                    })
                                });
                                let screen = snapshot.as_ref().map(|snapshot| {
                                    terminal_to_wire(session_id, &snapshot.terminal)
                                });
                                let _ = event_tx.send(DaemonPtyEvent::Exited {
                                    session_id,
                                    exit_code,
                                    screen,
                                });
                                return;
                            }
                            continue;
                        }
                        let result = match control {
                            Control::Attach(client) => session.attach(client).map(|_| Response::Unit),
                            Control::Detach(client) => session.detach(client).map(|_| Response::Unit),
                            Control::Take(client, epoch) => session.take_control(client, epoch).map(Response::Epoch),
                            Control::Release(client, epoch) => session.release_control(client, epoch).map(|_| Response::Unit),
                            Control::Input(client, epoch, bytes) => session.input(client, epoch, &bytes).await.map(|_| Response::Unit),
                            Control::Resize(client, epoch, dimensions) => session.resize(client, epoch, dimensions).map(|_| Response::Unit),
                            Control::Snapshot(offset) => session.snapshot(offset).map(|snapshot| Response::Screen(Box::new(terminal_to_wire(session_id, &snapshot.terminal)))),
                            Control::Rename(name) => session.rename(name).map(|_| Response::Unit),
                            Control::Terminate => unreachable!("handled above"),
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
}
