use super::*;

pub(super) fn handle(
    request: CommandRequest,
    services: &mut DaemonServices,
    client: &mut DaemonClient,
) -> Result<Option<CommandOutcome>> {
    Ok(Some(match request.command {
        DaemonCommand::StartRaw { request } => match services.raw_supervisor.start(request) {
            Ok(start) => {
                let command = start.state.request.command.to_string();
                let raw_event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(Box::new(
                        yoctui_app::raw_execution_snapshot_to_protocol(&start.state)
                            .map_err(anyhow::Error::msg)?,
                    )),
                )?;
                client.connection.send(&ServerMessage::Event(raw_event))?;
                let job_event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::JobChanged(
                        yoctui_protocol::daemon::JobSummary {
                            id: start.job_id,
                            kind: yoctui_protocol::daemon::JobKind::Raw,
                            label: format!("Raw {command}"),
                            lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting,
                            progress_current: None,
                            progress_total: None,
                            exit_code: None,
                        },
                    ),
                )?;
                client.connection.send(&ServerMessage::Event(job_event))?;
                CommandOutcome::Accepted
            }
            Err(error) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                message: error.to_string(),
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        DaemonCommand::StartRawPty {
            request,
            dimensions,
        } => match services.raw_supervisor.prepare_pty(request) {
            Ok(start) => {
                match services
                    .pty_supervisor
                    .start_raw(start.pty_id, &start.command, dimensions)
                {
                    Ok(()) => {
                        services.raw_supervisor.activate_pty(&start)?;
                        services
                            .pty_supervisor
                            .attach(start.pty_id, yoctui_model::PtyClientId(client.client_id.0))
                            .map_err(anyhow::Error::msg)?;
                        services.daemon_journal.publish(
                            yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(Box::new(
                                yoctui_app::raw_execution_snapshot_to_protocol(&start.state)
                                    .map_err(anyhow::Error::msg)?,
                            )),
                        )?;
                        services.daemon_journal.publish(
                            yoctui_protocol::daemon::DaemonEvent::PtyChanged(
                                yoctui_protocol::daemon::PtySessionSummary {
                                    id: yoctui_protocol::daemon::PtySessionId(start.pty_id.0),
                                    name: format!("Raw {}", start.state.request.command),
                                    kind: yoctui_protocol::daemon::PtyKind::Utility,
                                    cwd: start.command.current_directory().display().to_string(),
                                    lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting,
                                    dimensions,
                                    writer: None,
                                    writer_epoch: 0,
                                    viewers: 1,
                                    exit_code: None,
                                    restartable: false,
                                },
                            ),
                        )?;
                        CommandOutcome::Accepted
                    }
                    Err(message) => CommandOutcome::Rejected {
                        code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                        message,
                        current_generation: services.daemon_journal.snapshot().generation,
                    },
                }
            }
            Err(error) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                message: error.to_string(),
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        DaemonCommand::CancelRaw { request_id } => {
            match services.raw_supervisor.cancel(&request_id) {
                Ok(daemon_raw::DaemonRawCancel::Job) => CommandOutcome::Accepted,
                Ok(daemon_raw::DaemonRawCancel::Pty { pty_id, state }) => {
                    services.daemon_journal.publish(
                        yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(Box::new(
                            yoctui_app::raw_execution_snapshot_to_protocol(&state)
                                .map_err(anyhow::Error::msg)?,
                        )),
                    )?;
                    services
                        .pty_supervisor
                        .terminate(pty_id)
                        .map_err(anyhow::Error::msg)?;
                    CommandOutcome::Accepted
                }
                Err(error) => CommandOutcome::Rejected {
                    code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                    message: error.to_string(),
                    current_generation: services.daemon_journal.snapshot().generation,
                },
            }
        }
        DaemonCommand::SetRawAttachment {
            request_id,
            attached,
        } => match services.raw_supervisor.set_attachment(
            &request_id,
            if attached {
                yoctui_model::RawAttachmentState::Attached
            } else {
                yoctui_model::RawAttachmentState::Detached
            },
        ) {
            Ok(daemon_raw::DaemonRawAttachment::Job) => CommandOutcome::Accepted,
            Ok(daemon_raw::DaemonRawAttachment::Pty { pty_id }) => {
                let result = if attached {
                    services
                        .pty_supervisor
                        .attach(pty_id, yoctui_model::PtyClientId(client.client_id.0))
                } else {
                    services
                        .pty_supervisor
                        .detach(pty_id, yoctui_model::PtyClientId(client.client_id.0))
                };
                match result {
                    Ok(()) => CommandOutcome::Accepted,
                    Err(message) => CommandOutcome::Rejected {
                        code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                        message,
                        current_generation: services.daemon_journal.snapshot().generation,
                    },
                }
            }
            Err(error) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                message: error.to_string(),
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        _ => unreachable!("command family was checked by dispatch"),
    }))
}
