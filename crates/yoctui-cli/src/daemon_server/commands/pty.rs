use super::*;

pub(super) fn handle(
    request: CommandRequest,
    services: &mut DaemonServices,
    client: &mut DaemonClient,
) -> Result<Option<CommandOutcome>> {
    Ok(Some(match request.command {
        DaemonCommand::CreatePty {
            name,
            kind,
            cwd,
            command,
            dimensions,
        } => match services.pty_supervisor.start_new(
            name.clone(),
            kind,
            cwd.clone(),
            command,
            dimensions,
        ) {
            Ok(session_id) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::PtyChanged(
                        yoctui_protocol::daemon::PtySessionSummary {
                            id: yoctui_protocol::daemon::PtySessionId(session_id.0),
                            name,
                            kind,
                            cwd,
                            lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting,
                            dimensions,
                            writer: None,
                            writer_epoch: 0,
                            viewers: 0,
                            exit_code: None,
                            restartable: true,
                        },
                    ),
                )?;
                client.connection.send(&ServerMessage::Event(event))?;
                CommandOutcome::Accepted
            }
            Err(message) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                message,
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        DaemonCommand::TakePtyControl {
            session_id,
            expected_epoch,
        } => match services.pty_supervisor.take(
            yoctui_model::PtySessionId(session_id.0),
            yoctui_model::PtyClientId(client.client_id.0),
            expected_epoch,
        ) {
            Ok(_) => CommandOutcome::Accepted,
            Err(message) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                message,
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        DaemonCommand::ReleasePtyControl {
            session_id,
            expected_epoch,
        } => match services.pty_supervisor.release(
            yoctui_model::PtySessionId(session_id.0),
            yoctui_model::PtyClientId(client.client_id.0),
            expected_epoch,
        ) {
            Ok(()) => CommandOutcome::Accepted,
            Err(message) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                message,
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        DaemonCommand::TerminatePty {
            session_id, force, ..
        } => {
            if !force {
                CommandOutcome::Rejected {
                    code: yoctui_protocol::daemon::ProtocolErrorCode::ConfirmationRequired,
                    message: "PTY termination requires an explicit confirmed force request".into(),
                    current_generation: services.daemon_journal.snapshot().generation,
                }
            } else {
                if let Some(daemon_raw::DaemonRawCancel::Pty { state, .. }) = services
                    .raw_supervisor
                    .cancel_pty(yoctui_model::PtySessionId(session_id.0))?
                {
                    let event = services.daemon_journal.publish(
                        yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(Box::new(
                            yoctui_app::raw_execution_snapshot_to_protocol(&state)
                                .map_err(anyhow::Error::msg)?,
                        )),
                    )?;
                    client.connection.send(&ServerMessage::Event(event))?;
                }
                match services
                    .pty_supervisor
                    .terminate(yoctui_model::PtySessionId(session_id.0))
                {
                    Ok(()) => CommandOutcome::Accepted,
                    Err(message) => CommandOutcome::Rejected {
                        code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                        message,
                        current_generation: services.daemon_journal.snapshot().generation,
                    },
                }
            }
        }
        DaemonCommand::RenamePty { session_id, name } => {
            match services
                .pty_supervisor
                .rename(yoctui_model::PtySessionId(session_id.0), name)
            {
                Ok(()) => CommandOutcome::Accepted,
                Err(message) => CommandOutcome::Rejected {
                    code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                    message,
                    current_generation: services.daemon_journal.snapshot().generation,
                },
            }
        }
        DaemonCommand::ClosePty { session_id } => {
            let terminal = services
                .daemon_journal
                .snapshot()
                .pty_sessions
                .iter()
                .find(|terminal| terminal.id == session_id);
            if terminal.is_none() {
                CommandOutcome::Rejected {
                    code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                    message: format!("unknown PTY session {}", session_id.0),
                    current_generation: services.daemon_journal.snapshot().generation,
                }
            } else if terminal.is_some_and(|terminal| {
                matches!(
                    terminal.lifecycle,
                    yoctui_protocol::daemon::LifecycleState::Connecting
                        | yoctui_protocol::daemon::LifecycleState::Running
                        | yoctui_protocol::daemon::LifecycleState::Stopping
                )
            }) {
                CommandOutcome::Rejected {
                    code: yoctui_protocol::daemon::ProtocolErrorCode::ConfirmationRequired,
                    message: "running PTY must be explicitly terminated before close".into(),
                    current_generation: services.daemon_journal.snapshot().generation,
                }
            } else {
                match services
                    .pty_supervisor
                    .close(yoctui_model::PtySessionId(session_id.0))
                {
                    Ok(()) => {
                        let event = services.daemon_journal.publish(
                            yoctui_protocol::daemon::DaemonEvent::PtyRemoved { session_id },
                        )?;
                        client.connection.send(&ServerMessage::Event(event))?;
                        CommandOutcome::Accepted
                    }
                    Err(message) => CommandOutcome::Rejected {
                        code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                        message,
                        current_generation: services.daemon_journal.snapshot().generation,
                    },
                }
            }
        }
        _ => unreachable!("command family was checked by dispatch"),
    }))
}
