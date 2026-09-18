use super::*;

pub(super) fn handle(
    request: CommandRequest,
    services: &mut DaemonServices,
    client: &mut DaemonClient,
) -> Result<Option<CommandOutcome>> {
    Ok(Some(match request.command {
        DaemonCommand::StartSdk {
            session_id,
            operation,
            context,
        } => {
            match services
                .sdk_supervisor
                .start(session_id, operation, context)
            {
                Ok(job_id) => {
                    let event = services.daemon_journal.publish(
                        yoctui_protocol::daemon::DaemonEvent::JobChanged(
                            yoctui_protocol::daemon::JobSummary {
                                id: job_id,
                                kind: yoctui_protocol::daemon::JobKind::Sdk,
                                label: format!("SDK session {session_id}"),
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
                    message: error.to_string(),
                    current_generation: services.daemon_journal.snapshot().generation,
                },
            }
        }
        DaemonCommand::CancelSdk { session_id } => match services.sdk_supervisor.cancel(session_id)
        {
            Ok(()) => CommandOutcome::Accepted,
            Err(error) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                message: error.to_string(),
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        _ => unreachable!("command family was checked by dispatch"),
    }))
}
