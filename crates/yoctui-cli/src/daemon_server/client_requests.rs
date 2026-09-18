use super::*;

pub(super) fn service(
    services: &mut DaemonServices,
    client: &mut DaemonClient,
    record: &DaemonRuntimeRecord,
) -> Result<bool> {
    let mut keep_client = true;
    loop {
        if !client.connection.is_readable()? {
            break;
        }
        match client.connection.receive::<ClientMessage>() {
                Ok(ClientMessage::Hello(hello)) => {
                    if !yoctui_protocol::daemon::negotiate_version(
                        hello.minimum_version, hello.maximum_version, ProtocolVersion::CURRENT,
                    ).is_ok_and(|version| version == ProtocolVersion::CURRENT) {
                        let _ = client.connection.send(&ServerMessage::Error(yoctui_protocol::daemon::ProtocolFailure {
                            request_id: None,
                            code: yoctui_protocol::daemon::ProtocolErrorCode::IncompatibleVersion,
                            message: "Client and daemon protocol versions differ; upgrade both together".into(),
                            retryable: false,
                        }));
                        keep_client = false;
                        break;
                    }
                    client.connection.send(&ServerMessage::Hello(DaemonHello {
                        selected_version: ProtocolVersion::CURRENT,
                        daemon_instance_id: services.instance,
                        boot_id: record.boot_id.clone(),
                        capabilities: vec![
                            Capability::StateSnapshots,
                            Capability::IncrementalEvents,
                            Capability::EventReplay,
                            Capability::BackgroundJobs,
                            Capability::PtySessions,
                            Capability::PtyWriterLease,
                            Capability::PaneAttachments,
                            Capability::TerminalMouse,
                            Capability::EnvironmentCompatibility,
                            Capability::RawExecution,
                            Capability::RootfsSources,
                            Capability::GracefulShutdown,
                        ],
                        limits: ProtocolLimits {
                            maximum_frame_bytes: MAX_FRAME_BYTES as u32,
                            maximum_snapshot_bytes: MAX_FRAME_BYTES as u32,
                            maximum_pending_requests: 64,
                            maximum_queue_depth: 256,
                            maximum_terminal_rows: 512,
                            maximum_terminal_columns: 512,
                            maximum_clients: MAX_DAEMON_CLIENTS as u16,
                            maximum_pty_sessions: MAX_DAEMON_PTY_SESSIONS as u16,
                            maximum_scrollback_lines: MAX_TERMINAL_SCROLLBACK_LINES as u32,
                            maximum_utility_output_bytes: MAX_UTILITY_OUTPUT_BYTES as u32,
                        },
                    }))?;
                    client.client_id = hello.client_id;
                    client.negotiated = true;
                }
                Ok(ClientMessage::Attach { resume, subscription, .. }) if client.negotiated => {
                    for session_id in &subscription.pty_sessions {
                        let _ = services.pty_supervisor.attach(
                            yoctui_model::PtySessionId(session_id.0),
                            yoctui_model::PtyClientId(client.client_id.0),
                        );
                    }
                    match services.daemon_journal.synchronize(resume) {
                        DaemonSnapshotSync::Replace { snapshot, reason } => {
                            if reason != SnapshotReplacementReason::InitialAttach {
                                client.connection.send(&ServerMessage::ResyncRequired {
                                    reason: format!(
                                        "client replica must be replaced: {reason:?}"
                                    ),
                                    current_sequence: snapshot.sequence,
                                })?;
                            }
                            let replayed_through = snapshot.sequence;
                            client.last_sequence = replayed_through;
                            client.attached = true;
                            client.connection.send(&ServerMessage::Attached {
                                snapshot: *snapshot,
                                replayed_through,
                            })?;
                        }
                        DaemonSnapshotSync::Replay {
                            events,
                            replayed_through,
                        } => {
                            if daemon_replay_is_bounded(events.len()) {
                                for event in events {
                                    client.connection.send(&ServerMessage::Event(event))?;
                                }
                                client.connection.send(&ServerMessage::Attached {
                                    snapshot: services.daemon_journal.snapshot().clone(),
                                    replayed_through,
                                })?;
                                client.last_sequence = replayed_through;
                            } else {
                                // An attaching client cannot yield while its
                                // handshake is in progress. Replaying a large
                                // retained history here monopolizes the daemon
                                // and can starve the build supervisor. Replace
                                // the stale replica with one current snapshot;
                                // bounded incremental fan-out resumes after the
                                // attach has completed.
                                let snapshot = services.daemon_journal.snapshot().clone();
                                let current_sequence = snapshot.sequence;
                                client.connection.send(&ServerMessage::ResyncRequired {
                                    reason: format!(
                                        "client resume requires {event_count} events; replacing it with the current snapshot",
                                        event_count = events.len()
                                    ),
                                    current_sequence,
                                })?;
                                client.connection.send(&ServerMessage::Attached {
                                    snapshot,
                                    replayed_through: current_sequence,
                                })?;
                                client.last_sequence = current_sequence;
                            }
                            client.attached = true;
                        }
                    }
                }
                Ok(ClientMessage::Detach) if client.negotiated => {
                    client.connection.send(&ServerMessage::Detaching)?;
                    services.pty_supervisor.disconnect_client(yoctui_model::PtyClientId(client.client_id.0));
                    keep_client = false;
                    break;
                }
                Ok(ClientMessage::PtyInput(input)) if client.negotiated && client.attached => {
                    let outcome = services.pty_supervisor
                        .input(
                            yoctui_model::PtySessionId(input.session_id.0),
                            yoctui_model::PtyClientId(client.client_id.0),
                            input.writer_epoch,
                            input.bytes,
                        )
                        .map(|_| CommandOutcome::Accepted)
                        .unwrap_or_else(|message| CommandOutcome::Rejected {
                            code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                            message,
                            current_generation: services.daemon_journal.snapshot().generation,
                        });
                    client.connection.send(&ServerMessage::CommandResult(CommandResult {
                        request_id: input.request_id,
                        outcome,
                    }))?;
                }
                Ok(ClientMessage::PtyResize(resize)) if client.negotiated && client.attached => {
                    let outcome = services.pty_supervisor
                        .resize(
                            yoctui_model::PtySessionId(resize.session_id.0),
                            yoctui_model::PtyClientId(client.client_id.0),
                            resize.writer_epoch,
                            yoctui_model::PtyDimensions {
                                columns: resize.dimensions.columns,
                                rows: resize.dimensions.rows,
                            },
                        )
                        .map(|_| CommandOutcome::Accepted)
                        .unwrap_or_else(|message| CommandOutcome::Rejected {
                            code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                            message,
                            current_generation: services.daemon_journal.snapshot().generation,
                        });
                    client.connection.send(&ServerMessage::CommandResult(CommandResult {
                        request_id: resize.request_id,
                        outcome,
                    }))?;
                }
                Ok(ClientMessage::PtyViewport(viewport)) if client.negotiated && client.attached => {
                    let outcome = if viewport.scrollback_offset as usize
                        > yoctui_protocol::daemon::MAX_TERMINAL_SCROLLBACK_LINES
                    {
                        CommandOutcome::Rejected {
                            code: yoctui_protocol::daemon::ProtocolErrorCode::LimitExceeded,
                            message: "PTY scrollback offset exceeds the protocol limit".into(),
                            current_generation: services.daemon_journal.snapshot().generation,
                        }
                    } else {
                        match services.pty_supervisor.snapshot(
                            yoctui_model::PtySessionId(viewport.session_id.0),
                            viewport.scrollback_offset as usize,
                        ) {
                            Ok(screen) => {
                                let event = services.daemon_journal
                                    .publish(yoctui_protocol::daemon::DaemonEvent::PtyScreen(
                                        screen,
                                    ))?;
                                client.connection.send(&ServerMessage::Event(event))?;
                                CommandOutcome::Accepted
                            }
                            Err(message) => CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                message,
                                current_generation: services.daemon_journal.snapshot().generation,
                            },
                        }
                    };
                    client.connection.send(&ServerMessage::CommandResult(CommandResult {
                        request_id: viewport.request_id,
                        outcome,
                    }))?;
                }
                Ok(ClientMessage::Layout { event }) if client.negotiated && client.attached => {
                    use yoctui_protocol::daemon::ClientLayoutEvent;
                    match event {
                        ClientLayoutEvent::AttachSession { session_id, .. } => {
                            let _ = services.pty_supervisor.attach(
                                yoctui_model::PtySessionId(session_id.0),
                                yoctui_model::PtyClientId(client.client_id.0),
                            );
                        }
                        ClientLayoutEvent::DetachSession { session_id, .. } => {
                            let _ = services.pty_supervisor.detach(
                                yoctui_model::PtySessionId(session_id.0),
                                yoctui_model::PtyClientId(client.client_id.0),
                            );
                        }
                        ClientLayoutEvent::FocusWriter { .. } => {
                            // Writer acquisition remains an epoch-checked command; layout focus
                            // can never bypass the single-writer lease.
                        }
                    }
                }
                Ok(ClientMessage::Command(request))
                    if matches!(request.command, DaemonCommand::PrepareShutdown) =>
                {
                    let active_jobs = services.daemon_journal
                        .snapshot()
                        .jobs
                        .iter()
                        .filter(|job| {
                            matches!(
                                job.lifecycle,
                                yoctui_protocol::daemon::LifecycleState::Connecting
                                    | yoctui_protocol::daemon::LifecycleState::Running
                                    | yoctui_protocol::daemon::LifecycleState::Stopping
                            )
                        })
                        .count();
                    if active_jobs > 0 {
                        client.connection.send(&ServerMessage::CommandResult(CommandResult {
                            request_id: request.request_id,
                            outcome: CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::LimitExceeded,
                                message: format!(
                                    "daemon has {active_jobs} active job(s); cancel them explicitly before shutdown"
                                ),
                                current_generation: services.daemon_journal.snapshot().generation,
                            },
                        }))?;
                        continue;
                    }
                    client.connection.send(&ServerMessage::CommandResult(CommandResult {
                        request_id: request.request_id,
                        outcome: CommandOutcome::Completed,
                    }))?;
                    services.shutting_down = true;
                    break;
                }
                Ok(ClientMessage::Command(request)) if client.negotiated => {
                    use yoctui_protocol::daemon::{CommandOutcome, CommandResult};
                    if request.expected_generation.is_some_and(|generation| {
                        generation != services.daemon_journal.snapshot().generation
                    }) {
                        client.connection.send(&ServerMessage::CommandResult(CommandResult {
                            request_id: request.request_id,
                            outcome: CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::StaleGeneration,
                                message: "daemon generation changed; refresh and retry".into(),
                                current_generation: services.daemon_journal.snapshot().generation,
                            },
                        }))?;
                        continue;
                    }
                    let request_id = request.request_id;
                    // A deferred query or an already-sent rejection has no immediate reply.
                    let Some(outcome) = commands::dispatch(request, services, client)? else { continue; };
                    client.connection.send(&ServerMessage::CommandResult(CommandResult {
                        request_id,
                        outcome,
                    }))?;
                    // Command-created events are sent directly above. Advance
                    // this client's replay cursor so the next fan-out pass
                    // does not duplicate them.
                    client.last_sequence = services.daemon_journal.snapshot().sequence;
                }
                Ok(_) => client.connection.send(&ServerMessage::Error(
                    yoctui_protocol::daemon::ProtocolFailure {
                        request_id: None,
                        code: yoctui_protocol::daemon::ProtocolErrorCode::UnsupportedCapability,
                        message: "complete the daemon handshake before attach; this runtime currently supports state attach, detach, and graceful shutdown".into(),
                        retryable: false,
                    },
                ))?,
                Err(IpcError::Timeout(_)) => break,
                Err(IpcError::Disconnected) => {
                    services.pty_supervisor.disconnect_client(yoctui_model::PtyClientId(client.client_id.0));
                    keep_client = false;
                    break;
                }
                Err(error) => {
                    tracing::warn!(%error, "daemon client disconnected during request");
                    keep_client = false;
                    break;
                }
            }
    }

    Ok(keep_client)
}
