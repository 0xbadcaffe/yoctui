use super::*;

pub(super) fn handle(
    request: CommandRequest,
    services: &mut DaemonServices,
    client: &mut DaemonClient,
) -> Result<Option<CommandOutcome>> {
    Ok(Some(match request.command {
        DaemonCommand::StartTestSession {
            session_id,
            request,
            build_directory,
            path_directories,
        } => match services.test_supervisor.start(
            session_id,
            request,
            build_directory,
            path_directories,
        ) {
            Ok(job_id) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::JobChanged(
                        yoctui_protocol::daemon::JobSummary {
                            id: job_id,
                            kind: yoctui_protocol::daemon::JobKind::Testing,
                            label: format!("Test session {session_id}"),
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
        DaemonCommand::CancelTestSession { session_id } => {
            match services.test_supervisor.cancel(session_id) {
                Ok(()) => CommandOutcome::Accepted,
                Err(error) => CommandOutcome::Rejected {
                    code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                    message: error,
                    current_generation: services.daemon_journal.snapshot().generation,
                },
            }
        }
        DaemonCommand::ImportTestResults { generation, roots } => {
            match services.test_supervisor.import_results(generation, roots) {
                Ok(job_id) => {
                    let event = services.daemon_journal.publish(
                        yoctui_protocol::daemon::DaemonEvent::JobChanged(
                            yoctui_protocol::daemon::JobSummary {
                                id: job_id,
                                kind: yoctui_protocol::daemon::JobKind::Testing,
                                label: "Test result import".into(),
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
        DaemonCommand::CompareTestResults {
            generation,
            baseline_identity,
            candidate_identity,
        } => match services.test_supervisor.compare_results(
            generation,
            baseline_identity,
            candidate_identity,
        ) {
            Ok(job_id) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::JobChanged(
                        yoctui_protocol::daemon::JobSummary {
                            id: job_id,
                            kind: yoctui_protocol::daemon::JobKind::Testing,
                            label: "Test comparison".into(),
                            lifecycle: yoctui_protocol::daemon::LifecycleState::Exited,
                            progress_current: None,
                            progress_total: None,
                            exit_code: Some(0),
                        },
                    ),
                )?;
                client.connection.send(&ServerMessage::Event(event))?;
                CommandOutcome::Accepted
            }
            Err(error) => CommandOutcome::Rejected {
                code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                message: error,
                current_generation: services.daemon_journal.snapshot().generation,
            },
        },
        DaemonCommand::ExportTestJunit {
            generation,
            result_identity,
            destination,
        } => match services
            .test_supervisor
            .start_junit(generation, result_identity, destination)
        {
            Ok(job_id) => {
                let event = services.daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::JobChanged(
                        yoctui_protocol::daemon::JobSummary {
                            id: job_id,
                            kind: yoctui_protocol::daemon::JobKind::Testing,
                            label: "JUnit export".into(),
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
        DaemonCommand::InspectTestResultTool { path_directories } => {
            let capability = yoctui_bitbake::TestResultAdapter::new(
                path_directories
                    .into_iter()
                    .map(std::path::PathBuf::from)
                    .collect(),
            )
            .capability();
            let wire = match capability {
                yoctui_model::ResultToolCapability::NotInspected => {
                    yoctui_protocol::daemon::DaemonTestResultToolCapability::NotInspected
                }
                yoctui_model::ResultToolCapability::Missing => {
                    yoctui_protocol::daemon::DaemonTestResultToolCapability::Missing
                }
                yoctui_model::ResultToolCapability::Available(path) => {
                    yoctui_protocol::daemon::DaemonTestResultToolCapability::Available {
                        executable: path.display().to_string(),
                    }
                }
                yoctui_model::ResultToolCapability::Failed(message) => {
                    yoctui_protocol::daemon::DaemonTestResultToolCapability::Failed { message }
                }
            };
            let event = services
                .daemon_journal
                .publish(yoctui_protocol::daemon::DaemonEvent::TestResultTool(wire))?;
            client.connection.send(&ServerMessage::Event(event))?;
            CommandOutcome::Accepted
        }
        _ => unreachable!("command family was checked by dispatch"),
    }))
}
