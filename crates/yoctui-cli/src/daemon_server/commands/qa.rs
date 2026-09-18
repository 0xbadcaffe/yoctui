use super::*;

pub(super) fn handle(
    request: CommandRequest,
    services: &mut DaemonServices,
    client: &mut DaemonClient,
) -> Result<Option<CommandOutcome>> {
    Ok(Some(match request.command {
        DaemonCommand::InspectQaCapability { request } => match daemon_qa::inspect(request.input) {
            Ok(snapshot) => {
                let event = services
                    .daemon_journal
                    .publish(yoctui_protocol::daemon::DaemonEvent::QaCapability(snapshot))?;
                client.connection.send(&ServerMessage::Event(event))?;
                CommandOutcome::Accepted
            }
            Err(error) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                message: error,
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        DaemonCommand::StartQaLayerCheck {
            session_id,
            operation_id,
            check_id,
            layer_name,
            layer_root,
            executable,
            arguments,
            report_roots,
        } => match services.qa_supervisor.start(
            session_id,
            operation_id,
            check_id,
            layer_name,
            layer_root,
            executable,
            arguments,
            report_roots,
        ) {
            Ok(job_id) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::JobChanged(
                        yoctui_protocol::daemon::JobSummary {
                            id: job_id,
                            kind: yoctui_protocol::daemon::JobKind::Qa,
                            label: format!("QA layer session {session_id}"),
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
        DaemonCommand::CancelQaLayerCheck { session_id } => {
            match services.qa_supervisor.cancel(session_id) {
                Ok(()) => CommandOutcome::Accepted,
                Err(error) => CommandOutcome::Rejected {
                    code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                    message: error,
                    current_generation: services.daemon_journal.snapshot().generation,
                },
            }
        }
        DaemonCommand::StartQaReportScan {
            generation,
            build_directory,
            paths,
        } => match services
            .qa_report_supervisor
            .start(generation, build_directory, paths)
        {
            Ok(job_id) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::JobChanged(
                        yoctui_protocol::daemon::JobSummary {
                            id: job_id,
                            kind: yoctui_protocol::daemon::JobKind::Qa,
                            label: format!("QA report scan {generation}"),
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
        DaemonCommand::CancelQaReportScan { generation } => {
            match services.qa_report_supervisor.cancel(generation) {
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
