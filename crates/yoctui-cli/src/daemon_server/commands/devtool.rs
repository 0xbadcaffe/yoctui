use super::*;

pub(super) fn handle(
    request: CommandRequest,
    services: &mut DaemonServices,
    client: &mut DaemonClient,
) -> Result<Option<CommandOutcome>> {
    Ok(Some(match request.command {
        DaemonCommand::StartDevtool {
            operation,
            build_directory,
        } => match services
            .devtool_supervisor
            .start(operation, build_directory.into())
        {
            Ok(job_id) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::JobChanged(
                        yoctui_protocol::daemon::JobSummary {
                            id: job_id,
                            kind: yoctui_protocol::daemon::JobKind::Devtool,
                            label: "Devtool starting".into(),
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
        },
        DaemonCommand::InspectDevtoolStatus {
            recipe,
            recipe_file,
            build_directory,
        } => match services.devtool_supervisor.inspect_status(
            yoctui_model::RecipeIdentity {
                name: recipe,
                file: recipe_file.into(),
            },
            build_directory.into(),
        ) {
            Ok(()) => CommandOutcome::Accepted,
            Err(error) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                message: error.to_string(),
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        _ => unreachable!("command family was checked by dispatch"),
    }))
}
