use super::*;

pub(super) fn handle(
    request: CommandRequest,
    services: &mut DaemonServices,
    client: &mut DaemonClient,
) -> Result<Option<CommandOutcome>> {
    Ok(Some(match request.command {
        DaemonCommand::InspectMaintenanceCapability {
            request,
            build_directory,
            sstate_directory,
            tmp_directory,
            stamps_directories,
            executable_search_path,
        } => match daemon_maintenance::inspect(
            request,
            build_directory,
            sstate_directory,
            tmp_directory,
            stamps_directories,
            executable_search_path,
        ) {
            Ok(snapshot) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::MaintenanceSnapshot(snapshot),
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
        DaemonCommand::StartMaintenanceSstateReadiness {
            session_id,
            capability_request,
            operation_id,
            build_directory,
            sstate_directory,
            tmp_directory,
            stamps_directories,
            executable_search_path,
            targets,
            mode,
            output,
            log,
            timeout_seconds,
        } => match services.maintenance_supervisor.start_readiness(
            session_id,
            capability_request,
            operation_id,
            build_directory,
            sstate_directory,
            tmp_directory,
            stamps_directories,
            executable_search_path,
            targets,
            mode,
            output,
            log,
            timeout_seconds,
        ) {
            Ok(job_id) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::JobChanged(
                        yoctui_protocol::daemon::JobSummary {
                            id: job_id,
                            kind: yoctui_protocol::daemon::JobKind::Maintenance,
                            label: format!("Maintenance session {session_id}"),
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
        DaemonCommand::CancelMaintenance { session_id } => {
            match services.maintenance_supervisor.cancel(session_id) {
                Ok(()) => CommandOutcome::Accepted,
                Err(error) => CommandOutcome::Rejected {
                    code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                    message: error,
                    current_generation: services.daemon_journal.snapshot().generation,
                },
            }
        }
        DaemonCommand::StartMaintenanceExternal {
            session_id,
            executable,
            expected_name,
            arguments,
            current_directory,
        } => match services.maintenance_supervisor.start_external(
            session_id,
            executable,
            expected_name,
            arguments,
            current_directory,
        ) {
            Ok(job_id) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::JobChanged(
                        yoctui_protocol::daemon::JobSummary {
                            id: job_id,
                            kind: yoctui_protocol::daemon::JobKind::Maintenance,
                            label: format!("Maintenance session {session_id}"),
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
        DaemonCommand::InspectMaintenanceServices {
            request,
            build_directory,
            prserv_host,
            hashserve,
            hashserve_upstream,
            signature_handler,
            executable_search_path,
            process_root,
        } => match daemon_maintenance::inspect_services(
            request,
            build_directory,
            prserv_host,
            hashserve,
            hashserve_upstream,
            signature_handler,
            executable_search_path,
            process_root,
        ) {
            Ok(snapshot) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::MaintenanceSnapshot(snapshot),
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
        _ => unreachable!("command family was checked by dispatch"),
    }))
}
