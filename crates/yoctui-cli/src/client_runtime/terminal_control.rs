use yoctui_app::PrefixCommand;
use yoctui_model::{App, ClientDaemonLifecycle, TerminalEffect};
use yoctui_protocol::daemon::{
    ClientLayoutEvent, CommandRequest, DaemonCommand, PaneId, PtyInput, PtyResize, PtySessionId,
    PtyViewport, RequestId, TerminalDimensions,
};

use super::{ClientRuntimeError, InteractiveDaemonRuntime, RuntimeEffectRoute};

impl InteractiveDaemonRuntime {
    pub fn resize_selected_terminal(
        &mut self,
        app: &App,
        dimensions: yoctui_model::PtyDimensions,
    ) -> Result<bool, ClientRuntimeError> {
        let Some(details) = app.selected_terminal_details() else {
            self.last_pty_resize = None;
            return Ok(false);
        };
        if !app.selected_terminal_is_writer() {
            self.last_pty_resize = None;
            return Ok(false);
        }
        let dimensions = TerminalDimensions {
            columns: dimensions.columns,
            rows: dimensions.rows,
        };
        if details.columns == dimensions.columns && details.rows == dimensions.rows {
            self.last_pty_resize = None;
            return Ok(false);
        }
        let resize_key = (details.id, details.writer_epoch, dimensions);
        if self.last_pty_resize == Some(resize_key) {
            return Ok(false);
        }
        let request_id = RequestId(self.next_request);
        self.next_request = self
            .next_request
            .checked_add(1)
            .ok_or(ClientRuntimeError::RequestSpaceExhausted)?;
        self.transport.pty_resize(PtyResize {
            request_id,
            session_id: PtySessionId(details.id),
            writer_epoch: details.writer_epoch,
            dimensions,
        })?;
        self.last_pty_resize = Some(resize_key);
        Ok(true)
    }
    pub(super) fn route_terminal_effect(
        &mut self,
        app: &App,
        effect: &TerminalEffect,
    ) -> Result<RuntimeEffectRoute, ClientRuntimeError> {
        let request_id = RequestId(self.next_request);
        self.next_request = self
            .next_request
            .checked_add(1)
            .ok_or(ClientRuntimeError::RequestSpaceExhausted)?;
        match effect {
            TerminalEffect::Create {
                name,
                kind,
                cwd,
                program,
                arguments,
            } => self.transport.command(CommandRequest {
                request_id,
                expected_generation: Some(app.daemon.generation),
                command: DaemonCommand::CreatePty {
                    name: name.clone(),
                    kind: wire_terminal_kind(*kind),
                    cwd: cwd.display().to_string(),
                    command: yoctui_protocol::daemon::PtyCommand {
                        program: program.display().to_string(),
                        arguments: arguments.clone(),
                        environment_profile_id: None,
                    },
                    dimensions: TerminalDimensions {
                        columns: 120,
                        rows: 40,
                    },
                },
            })?,
            TerminalEffect::TakeControl {
                session_id,
                expected_epoch,
            } => {
                self.transport
                    .pty_layout(ClientLayoutEvent::AttachSession {
                        pane_id: PaneId(app.pane_layout.focused.0),
                        session_id: PtySessionId(*session_id),
                    })?;
                self.transport.command(CommandRequest {
                    request_id,
                    expected_generation: Some(app.daemon.generation),
                    command: DaemonCommand::TakePtyControl {
                        session_id: PtySessionId(*session_id),
                        expected_epoch: *expected_epoch,
                    },
                })?;
            }
            TerminalEffect::ReleaseControl {
                session_id,
                writer_epoch,
            } => self.transport.command(CommandRequest {
                request_id,
                expected_generation: Some(app.daemon.generation),
                command: DaemonCommand::ReleasePtyControl {
                    session_id: PtySessionId(*session_id),
                    expected_epoch: *writer_epoch,
                },
            })?,
            TerminalEffect::Input {
                session_id,
                writer_epoch,
                bytes,
            } => self.transport.pty_input(PtyInput {
                request_id,
                session_id: PtySessionId(*session_id),
                writer_epoch: *writer_epoch,
                bytes: bytes.clone(),
            })?,
            TerminalEffect::Viewport {
                session_id,
                scrollback_offset,
            } => self.transport.pty_viewport(PtyViewport {
                request_id,
                session_id: PtySessionId(*session_id),
                scrollback_offset: u32::try_from(*scrollback_offset)
                    .map_err(|_| ClientRuntimeError::InvalidTerminalViewport)?,
            })?,
            TerminalEffect::Rename { session_id, name } => {
                self.transport.command(CommandRequest {
                    request_id,
                    expected_generation: Some(app.daemon.generation),
                    command: DaemonCommand::RenamePty {
                        session_id: PtySessionId(*session_id),
                        name: name.clone(),
                    },
                })?;
            }
            TerminalEffect::Terminate { session_id } => {
                self.transport.command(CommandRequest {
                    request_id,
                    expected_generation: Some(app.daemon.generation),
                    command: DaemonCommand::TerminatePty {
                        session_id: PtySessionId(*session_id),
                        force: true,
                        confirmation: None,
                    },
                })?;
            }
            TerminalEffect::Close { session_id } => {
                self.transport.command(CommandRequest {
                    request_id,
                    expected_generation: Some(app.daemon.generation),
                    command: DaemonCommand::ClosePty {
                        session_id: PtySessionId(*session_id),
                    },
                })?;
            }
        }
        Ok(RuntimeEffectRoute::Daemon(request_id))
    }
    pub fn detach_terminal(&mut self, app: &App) -> Result<(), ClientRuntimeError> {
        let session = app
            .selected_terminal_session()
            .ok_or(ClientRuntimeError::MissingPtySession)?;
        self.transport
            .pty_layout(ClientLayoutEvent::DetachSession {
                pane_id: PaneId(app.pane_layout.focused.0),
                session_id: PtySessionId(session.id),
            })?;
        Ok(())
    }
}

pub(super) fn wire_terminal_kind(
    kind: yoctui_model::TerminalCreationKind,
) -> yoctui_protocol::daemon::PtyKind {
    match kind {
        yoctui_model::TerminalCreationKind::BuildShell => {
            yoctui_protocol::daemon::PtyKind::BuildShell
        }
        yoctui_model::TerminalCreationKind::DevtoolShell => {
            yoctui_protocol::daemon::PtyKind::DevtoolShell
        }
        yoctui_model::TerminalCreationKind::Utility | yoctui_model::TerminalCreationKind::GitUi => {
            yoctui_protocol::daemon::PtyKind::Utility
        }
        yoctui_model::TerminalCreationKind::Devshell => yoctui_protocol::daemon::PtyKind::Devshell,
        yoctui_model::TerminalCreationKind::Menuconfig => {
            yoctui_protocol::daemon::PtyKind::Menuconfig
        }
        yoctui_model::TerminalCreationKind::QemuConsole => {
            yoctui_protocol::daemon::PtyKind::QemuConsole
        }
        yoctui_model::TerminalCreationKind::SshConsole => {
            yoctui_protocol::daemon::PtyKind::SshConsole
        }
    }
}

pub(super) fn prefix_daemon_command(
    app: &App,
    command: PrefixCommand,
) -> Result<Option<DaemonCommand>, ClientRuntimeError> {
    let daemon_command = match command {
        PrefixCommand::CreateSession => {
            let cwd = app
                .workspace
                .build_dir
                .as_ref()
                .ok_or(ClientRuntimeError::MissingBuildDirectory)?;
            DaemonCommand::CreatePty {
                name: "build shell".into(),
                kind: yoctui_protocol::daemon::PtyKind::BuildShell,
                cwd: cwd.display().to_string(),
                command: yoctui_protocol::daemon::PtyCommand {
                    program: "/bin/sh".into(),
                    arguments: Vec::new(),
                    environment_profile_id: None,
                },
                dimensions: yoctui_protocol::daemon::TerminalDimensions {
                    columns: 120,
                    rows: 40,
                },
            }
        }
        PrefixCommand::TakeControl => {
            let session = app
                .daemon
                .pty_sessions
                .get(app.pty_selection)
                .filter(|session| matches!(session.lifecycle, ClientDaemonLifecycle::Running))
                .ok_or(ClientRuntimeError::MissingPtySession)?;
            let details = app
                .selected_terminal_details()
                .ok_or(ClientRuntimeError::MissingPtySession)?;
            DaemonCommand::TakePtyControl {
                session_id: yoctui_protocol::daemon::PtySessionId(session.id),
                expected_epoch: details.writer_epoch,
            }
        }
        PrefixCommand::CommandPalette
        | PrefixCommand::Help
        | PrefixCommand::OpenTerminalSessions
        | PrefixCommand::CopyMode
        | PrefixCommand::Search
        | PrefixCommand::Rename
        | PrefixCommand::ReleaseControl
        | PrefixCommand::Kill
        | PrefixCommand::Zoom => return Ok(None),
        PrefixCommand::NextSession
        | PrefixCommand::PreviousSession
        | PrefixCommand::SplitHorizontal
        | PrefixCommand::SplitVertical
        | PrefixCommand::ClosePane
        | PrefixCommand::Detach => return Ok(None),
    };
    Ok(Some(daemon_command))
}
