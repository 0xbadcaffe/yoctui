use super::*;

pub(super) fn handle(
    request: CommandRequest,
    services: &mut DaemonServices,
    client: &mut DaemonClient,
) -> Result<Option<CommandOutcome>> {
    Ok(Some(match request.command {
        DaemonCommand::StartSecurityReportScan { generation, paths } => {
            match services.security_supervisor.start(generation, paths) {
                Ok(job_id) => {
                    let event = services.daemon_journal.publish(
                        yoctui_protocol::daemon::DaemonEvent::JobChanged(
                            yoctui_protocol::daemon::JobSummary {
                                id: job_id,
                                kind: yoctui_protocol::daemon::JobKind::Security,
                                label: format!("Security report scan {generation}"),
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
            }
        }
        DaemonCommand::CancelSecurityReportScan { generation } => {
            match services.security_supervisor.cancel(generation) {
                Ok(()) => CommandOutcome::Accepted,
                Err(error) => CommandOutcome::Rejected {
                    code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                    message: error,
                    current_generation: services.daemon_journal.snapshot().generation,
                },
            }
        }
        DaemonCommand::StartSecurityPackageMap {
            session_id,
            executable,
            arguments,
            report_roots,
        } => match services.security_mapper_supervisor.start(
            session_id,
            executable,
            arguments,
            report_roots,
        ) {
            Ok(job_id) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::JobChanged(
                        yoctui_protocol::daemon::JobSummary {
                            id: job_id,
                            kind: yoctui_protocol::daemon::JobKind::Security,
                            label: format!("Security package map {session_id}"),
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
        DaemonCommand::CancelSecurityPackageMap { session_id } => {
            match services.security_mapper_supervisor.cancel(session_id) {
                Ok(()) => CommandOutcome::Accepted,
                Err(error) => CommandOutcome::Rejected {
                    code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                    message: error,
                    current_generation: services.daemon_journal.snapshot().generation,
                },
            }
        }
        _ => unreachable!("command family was checked by dispatch"),
    }))
}
