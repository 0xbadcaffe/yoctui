use super::*;

pub(super) fn handle(
    request: CommandRequest,
    services: &mut DaemonServices,
    client: &mut DaemonClient,
) -> Result<Option<CommandOutcome>> {
    Ok(Some(match request.command {
        DaemonCommand::StartWicCreate {
            session_id,
            request,
            build_directory,
            executable,
        } => match services
            .wic_supervisor
            .start(session_id, request, build_directory, executable)
        {
            Ok(job_id) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::JobChanged(
                        yoctui_protocol::daemon::JobSummary {
                            id: job_id,
                            kind: yoctui_protocol::daemon::JobKind::Wic,
                            label: format!("Wic session {session_id}"),
                            lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting,
                            progress_current: None,
                            progress_total: None,
                            exit_code: None,
                        },
                    ),
                )?;
                client.connection.send(&ServerMessage::Event(event))?;
                CommandOutcome::Accepted
            }
            Err(error) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                message: error,
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        DaemonCommand::StartWicWrite {
            session_id,
            executable,
            image_path,
            device_path,
            device_major_minor,
            device_size_bytes,
            device_model,
            device_serial,
            device_transport,
            build_directory,
        } => match services.wic_supervisor.start_write(
            session_id,
            executable,
            image_path,
            yoctui_model::WicDeviceIdentity {
                path: device_path.into(),
                major_minor: device_major_minor,
                size_bytes: device_size_bytes,
                model: device_model,
                serial: device_serial,
                transport: device_transport,
            },
            build_directory,
        ) {
            Ok(job_id) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::JobChanged(
                        yoctui_protocol::daemon::JobSummary {
                            id: job_id,
                            kind: yoctui_protocol::daemon::JobKind::Wic,
                            label: format!("Wic device session {session_id}"),
                            lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting,
                            progress_current: None,
                            progress_total: None,
                            exit_code: None,
                        },
                    ),
                )?;
                client.connection.send(&ServerMessage::Event(event))?;
                CommandOutcome::Accepted
            }
            Err(error) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                message: error,
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        DaemonCommand::CancelWic { session_id } => match services.wic_supervisor.cancel(session_id)
        {
            Ok(()) => CommandOutcome::Accepted,
            Err(error) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                message: error,
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        _ => unreachable!("command family was checked by dispatch"),
    }))
}
