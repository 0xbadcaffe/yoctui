use super::*;

pub(super) fn handle(
    request: CommandRequest,
    services: &mut DaemonServices,
    client: &mut DaemonClient,
) -> Result<Option<CommandOutcome>> {
    Ok(Some(match request.command {
DaemonCommand::StartBuild { .. } if services.rootfs_query_permit.available_permits() == 0 => {
                            CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::Conflict,
                                message: "Rootfs recipe metadata is still loading; retry the build when metadata is ready".into(),
                                current_generation: services.daemon_journal.snapshot().generation,
                            }
                        }
DaemonCommand::StartBuild { .. } if services.startup_compatibility.pending() || services.startup_metadata.as_ref().is_some_and(|scan| scan.pending()) => {
                            CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::Conflict,
                                message: "Initial compatibility or recipe inventory is still loading; retry the build when metadata is ready".into(),
                                current_generation: services.daemon_journal.snapshot().generation,
                            }
                        }
DaemonCommand::StartBuild { targets, task, force } => {
                            let build_dir = services.daemon_journal
                                .snapshot()
                                .compatibility
                                .as_ref()
                                .and_then(|compatibility| {
                                    match &compatibility.environment.build_directory {
                                        yoctui_protocol::daemon::CompatibilityDetected::Detected {
                                            value,
                                            ..
                                        } => Some(PathBuf::from(value)),
                                        yoctui_protocol::daemon::CompatibilityDetected::Unknown => {
                                            None
                                        }
                                    }
                                });
                            let Some(build_dir) = build_dir else {
                                client.connection.send(&ServerMessage::CommandResult(CommandResult {
                                    request_id: request.request_id,
                                    outcome: CommandOutcome::Rejected {
                                        code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                                        message: "daemon BitBake build requires current compatibility build-directory authority".into(),
                                        current_generation: services.daemon_journal.snapshot().generation,
                                    },
                                }))?;
                                return Ok(None);
                            };
                            let build_targets = targets.clone();
                            match services.bitbake_supervisor.start(
                                build_dir,
                                BuildRequest { targets, task, force },
                            ) {
                                Ok(job_id) => {
                                    let reset_event = services.daemon_journal.publish(
                                        yoctui_protocol::daemon::DaemonEvent::Build(
                                            yoctui_protocol::daemon::DaemonBuildEvent::Reset {
                                                targets: build_targets.clone(),
                                            },
                                        ),
                                    )?;
                                    client.connection.send(&ServerMessage::Event(reset_event))?;
                                    let mut bitbake = services.daemon_journal.snapshot().bitbake.clone();
                                    bitbake.lifecycle = yoctui_protocol::daemon::LifecycleState::Connecting;
                                    bitbake.diagnostic = None;
                                    let bitbake_event = services.daemon_journal.publish(
                                        yoctui_protocol::daemon::DaemonEvent::BitBakeChanged(bitbake),
                                    )?;
                                    client.connection.send(&ServerMessage::Event(bitbake_event))?;
                                    let event = services.daemon_journal.publish(
                                        yoctui_protocol::daemon::DaemonEvent::JobChanged(
                                            yoctui_protocol::daemon::JobSummary {
                                                id: job_id,
                                                kind: yoctui_protocol::daemon::JobKind::BitBakeBuild,
                                                label: format!("BitBake build {}", build_targets.join(" ")),
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
                                    code: yoctui_protocol::daemon::ProtocolErrorCode::LimitExceeded,
                                    message: error,
                                    current_generation: services.daemon_journal.snapshot().generation,
                                },
                            }
                        }
DaemonCommand::CancelJob { job_id } => {
                            match services.devtool_supervisor.cancel(job_id).or_else(|_| services.bitbake_supervisor.cancel(job_id)) {
                                Ok(()) => CommandOutcome::Accepted,
                                Err(error) => CommandOutcome::Rejected {
                                    code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                    message: error.to_string(),
                                    current_generation: services.daemon_journal.snapshot().generation,
                                },
                            }
                        }
_ => unreachable!("command family was checked by dispatch"),
}))
}
