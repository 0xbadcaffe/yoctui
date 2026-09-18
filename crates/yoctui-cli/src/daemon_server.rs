//! Daemon server.
use super::*;

#[cfg(unix)]
pub(crate) async fn run_daemon_foreground(
    termination: &mut tokio::sync::mpsc::Receiver<()>,
) -> Result<()> {
    use yoctui_protocol::{
        daemon::{
            Capability, ClientId, ClientMessage, CommandOutcome, CommandResult, DaemonCommand,
            DaemonHello, DaemonPressureCounters, DaemonRecoveryState, DaemonSnapshotJournal,
            DaemonSnapshotLimits, DaemonSnapshotSync, DaemonTelemetry, MAX_DAEMON_CLIENTS,
            MAX_DAEMON_PTY_SESSIONS, MAX_FRAME_BYTES, MAX_TERMINAL_SCROLLBACK_LINES,
            MAX_UTILITY_OUTPUT_BYTES, ProtocolLimits, ProtocolVersion, ServerMessage,
            SnapshotReplacementReason, encode_frame,
        },
        daemon_ipc::{DaemonConnection, DaemonListener, IpcError, runtime_paths},
        daemon_lifecycle::{
            DaemonRuntimeRecord, RuntimeRecordState, classify_runtime_record, read_boot_id,
            read_runtime_record, remove_runtime_record, write_runtime_record,
        },
        daemon_persist::{
            DaemonPersistedState, PersistedPreferences, persist_paths_for, read_persisted_state,
            recover_persisted_snapshot, write_persisted_state,
        },
    };
    let paths = runtime_paths()?;
    let listener = DaemonListener::bind(&paths)?;
    let boot_id = read_boot_id()?;
    if let Some(previous) = read_runtime_record(&paths)? {
        match classify_runtime_record(&previous, &boot_id) {
            RuntimeRecordState::Stale => {
                remove_runtime_record(&paths, previous.daemon_instance_id)?;
            }
            RuntimeRecordState::Current | RuntimeRecordState::ForeignProcess => {
                anyhow::bail!(
                    "refusing to replace live daemon runtime record for pid {}",
                    previous.pid
                );
            }
        }
    }
    let instance = random_instance_id()?;
    let record = DaemonRuntimeRecord {
        pid: std::process::id(),
        daemon_instance_id: instance,
        started_unix_ms: unix_ms(),
        boot_id,
        executable: env::current_exe()?.canonicalize()?,
    };
    let mut daemon_state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId(instance.0),
        record.started_unix_ms,
        record.boot_id.clone(),
        yoctui_model::DaemonStateLimits::default(),
    )?;
    let daemon_log_limit = daemon_state.limits.logs;
    yoctui_app::reduce_daemon_state(
        &mut daemon_state,
        yoctui_model::DaemonStateAction::ReplaceJobs(Box::new(daemon_job_state_from_app(
            &App::new(daemon_log_limit, MAX_FRAME_BYTES),
        ))),
    )?;
    let persist_paths = persist_paths_for(&daemon_state_root()?)?;
    let persisted = read_persisted_state(&persist_paths)?;
    if let Some(persisted) = &persisted {
        recover_daemon_model_metadata(&mut daemon_state, persisted, &record.boot_id)?;
    }
    let startup_environment = env::vars().collect::<BTreeMap<_, _>>();
    let mut startup_compatibility =
        daemon_compatibility::spawn_startup(startup_environment.clone());
    let snapshot = daemon_protocol_snapshot(&daemon_state);
    let snapshot = persisted
        .as_ref()
        .map(|persisted| recover_persisted_snapshot(snapshot.clone(), persisted, &record.boot_id).0)
        .unwrap_or(snapshot);
    let daemon_journal = DaemonSnapshotJournal::new(snapshot, DaemonSnapshotLimits::default())?;
    let mut daemon_journal = daemon_journal;
    let job_ids = daemon_job_ids::DaemonJobIds::from_snapshot(daemon_journal.snapshot());
    let mut devtool_supervisor = daemon_devtool::DaemonDevtoolSupervisor::new(job_ids.clone());
    devtool_supervisor.replace_compatibility(daemon_state.compatibility.clone())?;
    let mut raw_supervisor = daemon_raw::DaemonRawSupervisor::default();
    raw_supervisor.replace_compatibility(daemon_state.compatibility.clone())?;
    raw_supervisor.restore_snapshot(daemon_journal.snapshot())?;
    let mut bitbake_supervisor = daemon_bitbake::DaemonBitBakeSupervisor::new(job_ids.clone());
    bitbake_supervisor
        .replace_compatibility(daemon_state.compatibility.clone())
        .map_err(anyhow::Error::msg)?;
    let mut sdk_supervisor = daemon_sdk::DaemonSdkSupervisor::new(job_ids.clone());
    let mut qemu_supervisor = daemon_qemu::DaemonQemuSupervisor::new(job_ids.clone());
    let mut wic_supervisor = daemon_wic::DaemonWicSupervisor::new(job_ids.clone());
    let mut test_supervisor = daemon_test::DaemonTestSupervisor::new(job_ids.clone());
    let mut qa_supervisor = daemon_qa::DaemonQaSupervisor::new(job_ids.clone());
    let mut qa_report_supervisor = daemon_qa::DaemonQaReportSupervisor::new(job_ids.clone());
    let mut security_supervisor = daemon_security::DaemonSecuritySupervisor::new(job_ids.clone());
    let mut security_mapper_supervisor =
        daemon_security::DaemonSecurityMapperSupervisor::new(job_ids.clone());
    let mut maintenance_supervisor =
        daemon_maintenance::DaemonMaintenanceSupervisor::new(job_ids.clone());
    let mut pty_supervisor = daemon_pty::DaemonPtySupervisor::default();
    write_persisted_state(
        &persist_paths,
        &DaemonPersistedState::capture(
            daemon_journal.snapshot(),
            unix_ms(),
            record.boot_id.clone(),
            Vec::new(),
            PersistedPreferences::default(),
        ),
    )?;
    write_runtime_record(&paths, &record)?;
    let record_guard = DaemonRuntimeGuard {
        paths: paths.clone(),
        instance,
    };
    let rootfs_environment = startup_environment.clone();
    let rootfs_query_permit = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let mut retired_rootfs_queries: Vec<daemon_rootfs::PendingQuery> = Vec::new();
    let startup_configured = startup_environment.contains_key("BUILDDIR");
    let mut startup_metadata: Option<daemon_metadata::StartupMetadata> = None;
    if startup_configured {
        publish_startup_metadata_log(
            &mut daemon_journal,
            "Loading initial compatibility authority",
            false,
        )?;
    }
    // Connections are serviced in short, bounded slices.  Keeping the
    // negotiated state and replay cursor alongside each socket lets one idle
    // client yield to other clients while the daemon continues polling jobs.
    let mut clients: Vec<(
        DaemonConnection,
        bool,
        bool,
        u64,
        ClientId,
        Option<daemon_rootfs::PendingQuery>,
    )> = Vec::new();
    let mut archive_recorder = build_archive::Recorder::new(daemon_state_root()?);
    let mut shutting_down = false;
    let mut last_telemetry_ms = record.started_unix_ms;
    let mut maximum_client_backlog = 0_usize;
    let mut forced_client_resynchronizations = 0_u64;
    let mut slow_client_disconnects = 0_u64;
    const MAX_SUPERVISOR_EVENTS_PER_TICK: usize = 32;
    // Keep socket production comfortably below the interactive client's
    // bounded 64-event/8-ms poll. A busy reducer may not consume all 64 before
    // its time bound, so matching the 32-event supervisor ingress can still
    // fill the socket. Cursor expiry is recovered by a replacement Snapshot.
    while !shutting_down {
        retired_rootfs_queries.retain(|query| !query.is_finished());
        let client_connections = clients
            .iter()
            .map(|(connection, _, _, _, _, _)| connection)
            .collect::<Vec<_>>();
        listener.wait_for_activity_with_additional_fd(
            &client_connections,
            bitbake_supervisor.notification_fd(),
            daemon_service_wait(
                daemon_has_active_work(daemon_journal.snapshot())
                    || rootfs_query_permit.available_permits() == 0,
            ),
        )?;
        drop(client_connections);
        bitbake_supervisor.consume_notification();

        if let Some(result) = startup_compatibility.try_result() {
            match result {
                Ok(Some(compatibility)) => {
                    devtool_supervisor.replace_compatibility(Some(compatibility.clone()))?;
                    raw_supervisor.replace_compatibility(Some(compatibility.clone()))?;
                    bitbake_supervisor
                        .replace_compatibility(Some(compatibility.clone()))
                        .map_err(anyhow::Error::msg)?;
                    yoctui_app::reduce_daemon_state(
                        &mut daemon_state,
                        yoctui_model::DaemonStateAction::ReplaceCompatibility(Box::new(
                            compatibility.clone(),
                        )),
                    )?;
                    let wire = daemon_protocol_snapshot(&daemon_state)
                        .compatibility
                        .expect("installed compatibility has wire authority");
                    daemon_journal.publish(
                        yoctui_protocol::daemon::DaemonEvent::CompatibilityChanged(Box::new(wire)),
                    )?;
                    publish_startup_metadata_log(
                        &mut daemon_journal,
                        "Loading initial workspace and recipe inventory",
                        false,
                    )?;
                    let environment = startup_environment.clone();
                    startup_metadata = Some(daemon_metadata::StartupMetadata::spawn(
                        |cancelled| async move {
                            inspect_daemon_startup_workspace(
                                &environment,
                                Some(compatibility),
                                cancelled,
                            )
                            .await
                        },
                    ));
                }
                Ok(None) => {}
                Err(error) => {
                    eprintln!("Compatibility authority is unavailable: {error:#}");
                    tracing::warn!(%error, "daemon compatibility startup probe failed");
                    publish_startup_metadata_log(
                        &mut daemon_journal,
                        &format!("Compatibility authority is unavailable: {error:#}"),
                        true,
                    )?;
                    yoctui_app::reduce_daemon_state(
                        &mut daemon_state,
                        yoctui_model::DaemonStateAction::RecordError(format!(
                            "Compatibility authority is unavailable: {error:#}"
                        )),
                    )?;
                }
            }
        }
        if let Some(result) = startup_metadata.as_mut().and_then(|scan| scan.try_result()) {
            match result {
                Ok(Some(workspace)) => {
                    if daemon_metadata::publish_workspace(&mut daemon_journal, workspace.clone())? {
                        yoctui_app::reduce_daemon_state(
                            &mut daemon_state,
                            yoctui_model::DaemonStateAction::ReplaceWorkspace(workspace),
                        )?;
                    }
                }
                Ok(None) if startup_configured => publish_startup_metadata_log(
                    &mut daemon_journal,
                    "Initial metadata unavailable: no configured build environment",
                    false,
                )?,
                Ok(None) => {}
                Err(error) => publish_startup_metadata_log(
                    &mut daemon_journal,
                    &format!("Initial metadata scan failed: {error:#}"),
                    true,
                )?,
            }
        }

        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = devtool_supervisor.try_event() else {
                break;
            };
            publish_daemon_devtool_event(&mut daemon_journal, event)?;
        }
        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = raw_supervisor.try_event() else {
                break;
            };
            publish_daemon_raw_event(&mut daemon_journal, event)?;
        }
        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = bitbake_supervisor.try_event() else {
                break;
            };
            publish_daemon_bitbake_event(&mut daemon_journal, event)?;
        }
        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = sdk_supervisor.try_event() else {
                break;
            };
            publish_daemon_sdk_event(&mut daemon_journal, event)?;
        }
        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = qemu_supervisor.try_event() else {
                break;
            };
            publish_daemon_qemu_event(&mut daemon_journal, event)?;
        }
        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = wic_supervisor.try_event() else {
                break;
            };
            publish_daemon_wic_event(&mut daemon_journal, event)?;
        }
        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = test_supervisor.try_event() else {
                break;
            };
            publish_daemon_test_event(&mut daemon_journal, event)?;
        }
        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = qa_supervisor.try_event() else {
                break;
            };
            publish_daemon_qa_event(&mut daemon_journal, event)?;
        }
        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = qa_report_supervisor.try_event() else {
                break;
            };
            publish_daemon_qa_report_event(&mut daemon_journal, event)?;
        }
        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = security_supervisor.try_event() else {
                break;
            };
            publish_daemon_security_event(&mut daemon_journal, event)?;
        }
        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = security_mapper_supervisor.try_event() else {
                break;
            };
            publish_daemon_security_mapper_event(&mut daemon_journal, event)?;
        }
        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = maintenance_supervisor.try_event() else {
                break;
            };
            publish_daemon_maintenance_event(&mut daemon_journal, event)?;
        }
        for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
            let Some(event) = pty_supervisor.try_event() else {
                break;
            };
            let raw_state = match &event {
                daemon_pty::DaemonPtyEvent::Started { session_id, .. } => {
                    raw_supervisor.pty_started(*session_id)?
                }
                daemon_pty::DaemonPtyEvent::Changed {
                    session_id,
                    snapshot,
                } => raw_supervisor.pty_attachment(*session_id, snapshot.viewers > 0)?,
                daemon_pty::DaemonPtyEvent::Exited {
                    session_id,
                    exit_code,
                    ..
                } => raw_supervisor.pty_finished(*session_id, *exit_code, None)?,
                daemon_pty::DaemonPtyEvent::Lost {
                    session_id,
                    message,
                } => raw_supervisor.pty_finished(*session_id, None, Some(message.clone()))?,
                daemon_pty::DaemonPtyEvent::Output { .. } => None,
            };
            if let Some(state) = raw_state {
                daemon_journal.publish(
                    yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(Box::new(
                        yoctui_app::raw_execution_snapshot_to_protocol(&state)
                            .map_err(anyhow::Error::msg)?,
                    )),
                )?;
            }
            publish_daemon_pty_event(&mut daemon_journal, event)?;
        }
        let now_ms = unix_ms();
        archive_recorder
            .poll(daemon_journal.snapshot(), unix_ms())
            .await;
        let active_work = daemon_has_active_work(daemon_journal.snapshot());
        let current_client_backlog = clients
            .iter()
            .filter(|(_, _, attached, _, _, _)| *attached)
            .map(|(_, _, _, sequence, _, _)| {
                usize::try_from(daemon_journal.snapshot().sequence.saturating_sub(*sequence))
                    .unwrap_or(usize::MAX)
                    .min(daemon_journal.retained_event_capacity())
            })
            .sum::<usize>();
        maximum_client_backlog = maximum_client_backlog.max(current_client_backlog);
        let attached_clients = clients
            .iter()
            .filter(|(_, _, attached, _, _, _)| *attached)
            .count();
        let telemetry_interval = daemon_telemetry_interval(attached_clients, active_work);
        if telemetry_interval.is_some_and(|interval| {
            now_ms.saturating_sub(last_telemetry_ms)
                >= u64::try_from(interval.as_millis()).unwrap_or(u64::MAX)
        }) {
            let snapshot = daemon_journal.snapshot();
            let bitbake_pressure = bitbake_supervisor.pressure();
            let active_jobs = snapshot
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
            let _ = daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::Telemetry(
                DaemonTelemetry {
                    uptime_seconds: now_ms.saturating_sub(record.started_unix_ms) / 1_000,
                    bitbake: snapshot.bitbake.lifecycle,
                    connected_clients: attached_clients.min(u16::MAX as usize) as u16,
                    active_jobs: active_jobs.min(u16::MAX as usize) as u16,
                    pty_sessions: snapshot.pty_sessions.len().min(u16::MAX as usize) as u16,
                    queue_depth: bitbake_pressure
                        .current_queue_depth
                        .saturating_add(current_client_backlog)
                        .min(u16::MAX as usize) as u16,
                    pressure: DaemonPressureCounters {
                        current_queue_depth: bitbake_pressure
                            .current_queue_depth
                            .saturating_add(current_client_backlog)
                            .min(u32::MAX as usize)
                            as u32,
                        maximum_queue_depth: bitbake_pressure
                            .maximum_queue_depth
                            .saturating_add(maximum_client_backlog)
                            .min(u32::MAX as usize)
                            as u32,
                        cosmetic_coalesced: 0,
                        cosmetic_dropped: bitbake_pressure.cosmetic_dropped,
                        reliable_waits: bitbake_pressure.reliable_waits,
                        forced_resynchronizations: forced_client_resynchronizations,
                        slow_client_disconnects,
                    },
                    memory_bytes: process_memory_bytes(),
                    recovery: if persisted.is_some() {
                        DaemonRecoveryState::Recovered
                    } else {
                        DaemonRecoveryState::CleanStart
                    },
                },
            ))?;
            last_telemetry_ms = now_ms;
        }
        if termination_requested(termination) {
            break;
        }
        match listener.accept(Duration::ZERO) {
            Ok(connection) if clients.len() < MAX_DAEMON_CLIENTS => {
                // Read readiness is checked before receiving; this short
                // deadline only bounds a peer that stalls midway through a
                // frame. Handshake and replacement snapshots retain a
                // saturation-tolerant bounded write deadline; only live event
                // fan-out uses the short slow-client isolation slice below.
                connection.set_read_timeout(Some(Duration::from_millis(2)))?;
                connection.set_write_timeout(Some(Duration::from_secs(1)))?;
                clients.push((
                    connection,
                    false,
                    false,
                    daemon_journal.snapshot().sequence,
                    ClientId([0; 16]),
                    None,
                ));
            }
            Ok(_) => {
                tracing::warn!(limit = MAX_DAEMON_CLIENTS, "daemon client limit reached");
            }
            Err(IpcError::Timeout(_)) => {}
            Err(error) => return Err(error.into()),
        }

        let mut remaining_clients = Vec::with_capacity(clients.len());
        let mut encoded_event_frames = HashMap::<u64, Vec<u8>>::new();
        for (
            mut connection,
            mut negotiated,
            mut attached,
            mut last_sequence,
            mut client_id,
            mut rootfs_query,
        ) in clients.drain(..)
        {
            match connection.flush_event_frame() {
                Ok(true) => {}
                Ok(false) => {
                    remaining_clients.push((
                        connection,
                        negotiated,
                        attached,
                        last_sequence,
                        client_id,
                        rootfs_query,
                    ));
                    continue;
                }
                Err(error) => {
                    tracing::debug!(%error, "dropping daemon client after event delivery failure");
                    slow_client_disconnects = slow_client_disconnects.saturating_add(1);
                    pty_supervisor.disconnect_client(yoctui_model::PtyClientId(client_id.0));
                    if let Some(mut query) = rootfs_query.take() {
                        query.cancel();
                        retired_rootfs_queries.push(query);
                    }
                    continue;
                }
            }
            let mut keep_client = true;
            if let Some(result) = rootfs_query.as_mut().and_then(|query| query.try_result()) {
                let query = rootfs_query.take().expect("completed rootfs query");
                let result = result.and_then(|sources| {
                    let compatibility = daemon_state
                        .compatibility
                        .as_ref()
                        .context("rootfs compatibility authority was lost")?;
                    daemon_rootfs::validate_authority(&query.query, instance, compatibility)?;
                    Ok(sources)
                });
                let outcome = match result {
                    Ok(sources) => CommandOutcome::RootfsSources {
                        sources: Box::new(sources),
                    },
                    Err(error) => CommandOutcome::Rejected {
                        code: yoctui_protocol::daemon::ProtocolErrorCode::Conflict,
                        message: format!("rootfs metadata unavailable: {error:#}"),
                        current_generation: daemon_journal.snapshot().generation,
                    },
                };
                if connection
                    .send(&ServerMessage::CommandResult(CommandResult {
                        request_id: query.request_id,
                        outcome,
                    }))
                    .is_err()
                {
                    keep_client = false;
                }
            }
            if attached {
                match daemon_journal.synchronize_bounded(
                    yoctui_protocol::daemon::ResumeCursor {
                        daemon_instance_id: instance,
                        last_sequence,
                    },
                    MAX_DAEMON_CLIENT_EVENTS_PER_TICK,
                ) {
                    yoctui_protocol::daemon::DaemonSnapshotSync::Replay { events, .. } => {
                        for event in events {
                            let sequence = event.sequence;
                            let frame = match encoded_event_frames.entry(event.sequence) {
                                std::collections::hash_map::Entry::Occupied(entry) => {
                                    entry.into_mut()
                                }
                                std::collections::hash_map::Entry::Vacant(entry) => {
                                    entry.insert(encode_frame(&ServerMessage::Event(event))?)
                                }
                            };
                            match connection
                                .queue_event_frame(frame)
                                .and_then(|()| connection.flush_event_frame())
                            {
                                Ok(complete) => {
                                    last_sequence = sequence;
                                    if !complete {
                                        break;
                                    }
                                }
                                Err(error) => {
                                    tracing::debug!(%error, "dropping daemon client during event fan-out");
                                    slow_client_disconnects =
                                        slow_client_disconnects.saturating_add(1);
                                    keep_client = false;
                                    break;
                                }
                            }
                        }
                    }
                    yoctui_protocol::daemon::DaemonSnapshotSync::Replace { snapshot, reason } => {
                        forced_client_resynchronizations =
                            forced_client_resynchronizations.saturating_add(1);
                        last_sequence = snapshot.sequence;
                        // This client is already attached. `Attached` is only
                        // valid during the handshake; after a cursor expires,
                        // replace the live replica through the Snapshot frame
                        // that the attached transport accepts.
                        let replacement = if reason != SnapshotReplacementReason::InitialAttach {
                            connection.send(&ServerMessage::ResyncRequired {
                                reason: format!("client replica must be replaced: {reason:?}"),
                                current_sequence: snapshot.sequence,
                            })
                        } else {
                            Ok(())
                        }
                        .and_then(|()| connection.send(&ServerMessage::Snapshot(*snapshot)));
                        if let Err(error) = replacement {
                            tracing::debug!(%error, "dropping daemon client during replica replacement");
                            keep_client = false;
                        }
                    }
                }
            }
            if !keep_client {
                pty_supervisor.disconnect_client(yoctui_model::PtyClientId(client_id.0));
                if let Some(mut query) = rootfs_query.take() {
                    query.cancel();
                    retired_rootfs_queries.push(query);
                }
                continue;
            }
            if connection.event_write_pending() {
                remaining_clients.push((
                    connection,
                    negotiated,
                    attached,
                    last_sequence,
                    client_id,
                    rootfs_query,
                ));
                continue;
            }
            loop {
                if !connection.is_readable()? {
                    break;
                }
                match connection.receive::<ClientMessage>() {
                Ok(ClientMessage::Hello(hello)) => {
                    if !yoctui_protocol::daemon::negotiate_version(
                        hello.minimum_version, hello.maximum_version, ProtocolVersion::CURRENT,
                    ).is_ok_and(|version| version == ProtocolVersion::CURRENT) {
                        let _ = connection.send(&ServerMessage::Error(yoctui_protocol::daemon::ProtocolFailure {
                            request_id: None,
                            code: yoctui_protocol::daemon::ProtocolErrorCode::IncompatibleVersion,
                            message: "Client and daemon protocol versions differ; upgrade both together".into(),
                            retryable: false,
                        }));
                        keep_client = false;
                        break;
                    }
                    connection.send(&ServerMessage::Hello(DaemonHello {
                        selected_version: ProtocolVersion::CURRENT,
                        daemon_instance_id: instance,
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
                    client_id = hello.client_id;
                    negotiated = true;
                }
                Ok(ClientMessage::Attach { resume, subscription, .. }) if negotiated => {
                    for session_id in &subscription.pty_sessions {
                        let _ = pty_supervisor.attach(
                            yoctui_model::PtySessionId(session_id.0),
                            yoctui_model::PtyClientId(client_id.0),
                        );
                    }
                    match daemon_journal.synchronize(resume) {
                        DaemonSnapshotSync::Replace { snapshot, reason } => {
                            if reason != SnapshotReplacementReason::InitialAttach {
                                connection.send(&ServerMessage::ResyncRequired {
                                    reason: format!(
                                        "client replica must be replaced: {reason:?}"
                                    ),
                                    current_sequence: snapshot.sequence,
                                })?;
                            }
                            let replayed_through = snapshot.sequence;
                            last_sequence = replayed_through;
                            attached = true;
                            connection.send(&ServerMessage::Attached {
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
                                    connection.send(&ServerMessage::Event(event))?;
                                }
                                connection.send(&ServerMessage::Attached {
                                    snapshot: daemon_journal.snapshot().clone(),
                                    replayed_through,
                                })?;
                                last_sequence = replayed_through;
                            } else {
                                // An attaching client cannot yield while its
                                // handshake is in progress. Replaying a large
                                // retained history here monopolizes the daemon
                                // and can starve the build supervisor. Replace
                                // the stale replica with one current snapshot;
                                // bounded incremental fan-out resumes after the
                                // attach has completed.
                                let snapshot = daemon_journal.snapshot().clone();
                                let current_sequence = snapshot.sequence;
                                connection.send(&ServerMessage::ResyncRequired {
                                    reason: format!(
                                        "client resume requires {event_count} events; replacing it with the current snapshot",
                                        event_count = events.len()
                                    ),
                                    current_sequence,
                                })?;
                                connection.send(&ServerMessage::Attached {
                                    snapshot,
                                    replayed_through: current_sequence,
                                })?;
                                last_sequence = current_sequence;
                            }
                            attached = true;
                        }
                    }
                }
                Ok(ClientMessage::Detach) if negotiated => {
                    connection.send(&ServerMessage::Detaching)?;
                    pty_supervisor.disconnect_client(yoctui_model::PtyClientId(client_id.0));
                    keep_client = false;
                    break;
                }
                Ok(ClientMessage::PtyInput(input)) if negotiated && attached => {
                    let outcome = pty_supervisor
                        .input(
                            yoctui_model::PtySessionId(input.session_id.0),
                            yoctui_model::PtyClientId(client_id.0),
                            input.writer_epoch,
                            input.bytes,
                        )
                        .map(|_| CommandOutcome::Accepted)
                        .unwrap_or_else(|message| CommandOutcome::Rejected {
                            code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                            message,
                            current_generation: daemon_journal.snapshot().generation,
                        });
                    connection.send(&ServerMessage::CommandResult(CommandResult {
                        request_id: input.request_id,
                        outcome,
                    }))?;
                }
                Ok(ClientMessage::PtyResize(resize)) if negotiated && attached => {
                    let outcome = pty_supervisor
                        .resize(
                            yoctui_model::PtySessionId(resize.session_id.0),
                            yoctui_model::PtyClientId(client_id.0),
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
                            current_generation: daemon_journal.snapshot().generation,
                        });
                    connection.send(&ServerMessage::CommandResult(CommandResult {
                        request_id: resize.request_id,
                        outcome,
                    }))?;
                }
                Ok(ClientMessage::PtyViewport(viewport)) if negotiated && attached => {
                    let outcome = if viewport.scrollback_offset as usize
                        > yoctui_protocol::daemon::MAX_TERMINAL_SCROLLBACK_LINES
                    {
                        CommandOutcome::Rejected {
                            code: yoctui_protocol::daemon::ProtocolErrorCode::LimitExceeded,
                            message: "PTY scrollback offset exceeds the protocol limit".into(),
                            current_generation: daemon_journal.snapshot().generation,
                        }
                    } else {
                        match pty_supervisor.snapshot(
                            yoctui_model::PtySessionId(viewport.session_id.0),
                            viewport.scrollback_offset as usize,
                        ) {
                            Ok(screen) => {
                                let event = daemon_journal
                                    .publish(yoctui_protocol::daemon::DaemonEvent::PtyScreen(
                                        screen,
                                    ))?;
                                connection.send(&ServerMessage::Event(event))?;
                                CommandOutcome::Accepted
                            }
                            Err(message) => CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                message,
                                current_generation: daemon_journal.snapshot().generation,
                            },
                        }
                    };
                    connection.send(&ServerMessage::CommandResult(CommandResult {
                        request_id: viewport.request_id,
                        outcome,
                    }))?;
                }
                Ok(ClientMessage::Layout { event }) if negotiated && attached => {
                    use yoctui_protocol::daemon::ClientLayoutEvent;
                    match event {
                        ClientLayoutEvent::AttachSession { session_id, .. } => {
                            let _ = pty_supervisor.attach(
                                yoctui_model::PtySessionId(session_id.0),
                                yoctui_model::PtyClientId(client_id.0),
                            );
                        }
                        ClientLayoutEvent::DetachSession { session_id, .. } => {
                            let _ = pty_supervisor.detach(
                                yoctui_model::PtySessionId(session_id.0),
                                yoctui_model::PtyClientId(client_id.0),
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
                    let active_jobs = daemon_journal
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
                        connection.send(&ServerMessage::CommandResult(CommandResult {
                            request_id: request.request_id,
                            outcome: CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::LimitExceeded,
                                message: format!(
                                    "daemon has {active_jobs} active job(s); cancel them explicitly before shutdown"
                                ),
                                current_generation: daemon_journal.snapshot().generation,
                            },
                        }))?;
                        continue;
                    }
                    connection.send(&ServerMessage::CommandResult(CommandResult {
                        request_id: request.request_id,
                        outcome: CommandOutcome::Completed,
                    }))?;
                    shutting_down = true;
                    break;
                }
                Ok(ClientMessage::Command(request)) if negotiated => {
                    use yoctui_protocol::daemon::{CommandOutcome, CommandResult};
                    if request.expected_generation.is_some_and(|generation| {
                        generation != daemon_journal.snapshot().generation
                    }) {
                        connection.send(&ServerMessage::CommandResult(CommandResult {
                            request_id: request.request_id,
                            outcome: CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::StaleGeneration,
                                message: "daemon generation changed; refresh and retry".into(),
                                current_generation: daemon_journal.snapshot().generation,
                            },
                        }))?;
                        continue;
                    }
                    let outcome = match request.command {
                        DaemonCommand::InspectRootfsSources { query } => {
                            let result = (|| -> Result<daemon_rootfs::PendingQuery> {
                                anyhow::ensure!(attached && request.expected_generation.is_some(), "rootfs queries require current attached authority");
                                anyhow::ensure!(rootfs_query.is_none(), "this client already has a rootfs query");
                                anyhow::ensure!(!startup_compatibility.pending() && !startup_metadata.as_ref().is_some_and(|scan| scan.pending()) && !daemon_journal.snapshot().jobs.iter().any(|job| job.kind == yoctui_protocol::daemon::JobKind::BitBakeBuild && matches!(job.lifecycle, yoctui_protocol::daemon::LifecycleState::Connecting | yoctui_protocol::daemon::LifecycleState::Running | yoctui_protocol::daemon::LifecycleState::Stopping)), "rootfs metadata is busy; retry after the active build finishes");
                                let compatibility = daemon_state.compatibility.clone().context("rootfs query requires compatibility authority")?;
                                daemon_rootfs::PendingQuery::start(request.request_id, query, instance, compatibility, rootfs_environment.clone(), rootfs_query_permit.clone())
                            })();
                            match result {
                                Ok(pending) => { rootfs_query = Some(pending); continue; }
                                Err(error) => CommandOutcome::Rejected {
                                    code: yoctui_protocol::daemon::ProtocolErrorCode::Conflict,
                                    message: format!("rootfs metadata unavailable: {error:#}"),
                                    current_generation: daemon_journal.snapshot().generation,
                                },
                            }
                        }
                        DaemonCommand::StartBuild { .. } if rootfs_query_permit.available_permits() == 0 => {
                            CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::Conflict,
                                message: "Rootfs recipe metadata is still loading; retry the build when metadata is ready".into(),
                                current_generation: daemon_journal.snapshot().generation,
                            }
                        }
                        DaemonCommand::StartBuild { .. } if startup_compatibility.pending() || startup_metadata.as_ref().is_some_and(|scan| scan.pending()) => {
                            CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::Conflict,
                                message: "Initial compatibility or recipe inventory is still loading; retry the build when metadata is ready".into(),
                                current_generation: daemon_journal.snapshot().generation,
                            }
                        }
                        DaemonCommand::StartBuild { targets, task, force } => {
                            let build_dir = daemon_journal
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
                                connection.send(&ServerMessage::CommandResult(CommandResult {
                                    request_id: request.request_id,
                                    outcome: CommandOutcome::Rejected {
                                        code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                                        message: "daemon BitBake build requires current compatibility build-directory authority".into(),
                                        current_generation: daemon_journal.snapshot().generation,
                                    },
                                }))?;
                                continue;
                            };
                            let build_targets = targets.clone();
                            match bitbake_supervisor.start(
                                build_dir,
                                BuildRequest { targets, task, force },
                            ) {
                                Ok(job_id) => {
                                    let reset_event = daemon_journal.publish(
                                        yoctui_protocol::daemon::DaemonEvent::Build(
                                            yoctui_protocol::daemon::DaemonBuildEvent::Reset {
                                                targets: build_targets.clone(),
                                            },
                                        ),
                                    )?;
                                    connection.send(&ServerMessage::Event(reset_event))?;
                                    let mut bitbake = daemon_journal.snapshot().bitbake.clone();
                                    bitbake.lifecycle = yoctui_protocol::daemon::LifecycleState::Connecting;
                                    bitbake.diagnostic = None;
                                    let bitbake_event = daemon_journal.publish(
                                        yoctui_protocol::daemon::DaemonEvent::BitBakeChanged(bitbake),
                                    )?;
                                    connection.send(&ServerMessage::Event(bitbake_event))?;
                                    let event = daemon_journal.publish(
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
                                    connection.send(&ServerMessage::Event(event))?;
                                    CommandOutcome::Accepted
                                }
                                Err(error) => CommandOutcome::Rejected {
                                    code: yoctui_protocol::daemon::ProtocolErrorCode::LimitExceeded,
                                    message: error,
                                    current_generation: daemon_journal.snapshot().generation,
                                },
                            }
                        }
                        DaemonCommand::StartDevtool {
                            operation,
                            build_directory,
                        } => match devtool_supervisor.start(operation, build_directory.into()) {
                            Ok(job_id) => {
                                let event = daemon_journal.publish(
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
                                connection.send(&ServerMessage::Event(event))?;
                                CommandOutcome::Accepted
                            }
                            Err(error) => CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                                message: error.to_string(),
                                current_generation: daemon_journal.snapshot().generation,
                            },
                        },
                        DaemonCommand::StartRaw { request } => match raw_supervisor.start(request) {
                            Ok(start) => {
                                let command = start.state.request.command.to_string();
                                let raw_event = daemon_journal.publish(
                                    yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(
                                        Box::new(
                                            yoctui_app::raw_execution_snapshot_to_protocol(
                                                &start.state,
                                            )
                                            .map_err(anyhow::Error::msg)?,
                                        ),
                                    ),
                                )?;
                                connection.send(&ServerMessage::Event(raw_event))?;
                                let job_event = daemon_journal.publish(
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
                                connection.send(&ServerMessage::Event(job_event))?;
                                CommandOutcome::Accepted
                            }
                            Err(error) => CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                                message: error.to_string(),
                                current_generation: daemon_journal.snapshot().generation,
                            },
                        },
                        DaemonCommand::StartRawPty {
                            request,
                            dimensions,
                        } => match raw_supervisor.prepare_pty(request) {
                            Ok(start) => match pty_supervisor.start_raw(
                                start.pty_id,
                                &start.command,
                                dimensions,
                            ) {
                                Ok(()) => {
                                    raw_supervisor.activate_pty(&start)?;
                                    pty_supervisor.attach(
                                        start.pty_id,
                                        yoctui_model::PtyClientId(client_id.0),
                                    ).map_err(anyhow::Error::msg)?;
                                    daemon_journal.publish(
                                        yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(
                                            Box::new(
                                                yoctui_app::raw_execution_snapshot_to_protocol(
                                                    &start.state,
                                                )
                                                .map_err(anyhow::Error::msg)?,
                                            ),
                                        ),
                                    )?;
                                    daemon_journal.publish(
                                        yoctui_protocol::daemon::DaemonEvent::PtyChanged(
                                            yoctui_protocol::daemon::PtySessionSummary {
                                                id: yoctui_protocol::daemon::PtySessionId(
                                                    start.pty_id.0,
                                                ),
                                                name: format!(
                                                    "Raw {}",
                                                    start.state.request.command
                                                ),
                                                kind: yoctui_protocol::daemon::PtyKind::Utility,
                                                cwd: start
                                                    .command
                                                    .current_directory()
                                                    .display()
                                                    .to_string(),
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
                                    current_generation: daemon_journal.snapshot().generation,
                                },
                            },
                            Err(error) => CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                                message: error.to_string(),
                                current_generation: daemon_journal.snapshot().generation,
                            },
                        },
                        DaemonCommand::CancelRaw { request_id } => {
                            match raw_supervisor.cancel(&request_id) {
                                Ok(daemon_raw::DaemonRawCancel::Job) => CommandOutcome::Accepted,
                                Ok(daemon_raw::DaemonRawCancel::Pty { pty_id, state }) => {
                                    daemon_journal.publish(
                                        yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(
                                            Box::new(
                                                yoctui_app::raw_execution_snapshot_to_protocol(
                                                    &state,
                                                )
                                                .map_err(anyhow::Error::msg)?,
                                            ),
                                        ),
                                    )?;
                                    pty_supervisor
                                        .terminate(pty_id)
                                        .map_err(anyhow::Error::msg)?;
                                    CommandOutcome::Accepted
                                }
                                Err(error) => CommandOutcome::Rejected {
                                    code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                    message: error.to_string(),
                                    current_generation: daemon_journal.snapshot().generation,
                                },
                            }
                        }
                        DaemonCommand::SetRawAttachment {
                            request_id,
                            attached,
                        } => match raw_supervisor.set_attachment(
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
                                    pty_supervisor.attach(
                                        pty_id,
                                        yoctui_model::PtyClientId(client_id.0),
                                    )
                                } else {
                                    pty_supervisor.detach(
                                        pty_id,
                                        yoctui_model::PtyClientId(client_id.0),
                                    )
                                };
                                match result {
                                    Ok(()) => CommandOutcome::Accepted,
                                    Err(message) => CommandOutcome::Rejected {
                                        code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                        message,
                                        current_generation: daemon_journal.snapshot().generation,
                                    },
                                }
                            }
                            Err(error) => CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                message: error.to_string(),
                                current_generation: daemon_journal.snapshot().generation,
                            },
                        },
                        DaemonCommand::CancelJob { job_id } => {
                            match devtool_supervisor.cancel(job_id).or_else(|_| bitbake_supervisor.cancel(job_id)) {
                                Ok(()) => CommandOutcome::Accepted,
                                Err(error) => CommandOutcome::Rejected {
                                    code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                    message: error.to_string(),
                                    current_generation: daemon_journal.snapshot().generation,
                                },
                            }
                        }
                        DaemonCommand::StartSdk { session_id, operation, context } => {
                            match sdk_supervisor.start(session_id, operation, context) {
                                Ok(job_id) => {
                                    let event = daemon_journal.publish(
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
                                    connection.send(&ServerMessage::Event(event))?;
                                    CommandOutcome::Accepted
                                }
                                Err(error) => CommandOutcome::Rejected {
                                    code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                                    message: error.to_string(),
                                    current_generation: daemon_journal.snapshot().generation,
                                },
                            }
                        }
                        DaemonCommand::CancelSdk { session_id } => match sdk_supervisor.cancel(session_id) {
                            Ok(()) => CommandOutcome::Accepted,
                            Err(error) => CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                message: error.to_string(),
                                current_generation: daemon_journal.snapshot().generation,
                            },
                        },
                        DaemonCommand::StartQemu { session_id, request, build_directory, executable } => {
                            match qemu_supervisor.start(session_id, request, build_directory, executable) {
                                Ok(job_id) => { let event = daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary { id: job_id, kind: yoctui_protocol::daemon::JobKind::Qemu, label: format!("QEMU session {session_id}"), lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting, progress_current: None, progress_total: None, exit_code: None }))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted }
                                Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage, message: error, current_generation: daemon_journal.snapshot().generation }
                            }
                        }
                        DaemonCommand::CancelQemu { session_id } => match qemu_supervisor.cancel(session_id) {
                            Ok(()) => CommandOutcome::Accepted,
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound, message: error, current_generation: daemon_journal.snapshot().generation }
                        },
                        DaemonCommand::StartWicCreate { session_id, request, build_directory, executable } => match wic_supervisor.start(session_id, request, build_directory, executable) {
                            Ok(job_id) => { let event = daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary { id: job_id, kind: yoctui_protocol::daemon::JobKind::Wic, label: format!("Wic session {session_id}"), lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting, progress_current: None, progress_total: None, exit_code: None }))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted },
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::StartWicWrite { session_id, executable, image_path, device_path, device_major_minor, device_size_bytes, device_model, device_serial, device_transport, build_directory } => match wic_supervisor.start_write(session_id, executable, image_path, yoctui_model::WicDeviceIdentity { path: device_path.into(), major_minor: device_major_minor, size_bytes: device_size_bytes, model: device_model, serial: device_serial, transport: device_transport }, build_directory) {
                            Ok(job_id) => { let event = daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary { id: job_id, kind: yoctui_protocol::daemon::JobKind::Wic, label: format!("Wic device session {session_id}"), lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting, progress_current: None, progress_total: None, exit_code: None }))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted },
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::CancelWic { session_id } => match wic_supervisor.cancel(session_id) { Ok(()) => CommandOutcome::Accepted, Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound, message: error, current_generation: daemon_journal.snapshot().generation } },
                        DaemonCommand::StartTestSession { session_id, request, build_directory, path_directories } => match test_supervisor.start(session_id, request, build_directory, path_directories) {
                            Ok(job_id) => { let event=daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary{id:job_id,kind:yoctui_protocol::daemon::JobKind::Testing,label:format!("Test session {session_id}"),lifecycle:yoctui_protocol::daemon::LifecycleState::Connecting,progress_current:None,progress_total:None,exit_code:None}))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted },
                            Err(error)=>CommandOutcome::Rejected{code:yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,message:error,current_generation:daemon_journal.snapshot().generation},
                        },
                        DaemonCommand::CancelTestSession { session_id } => match test_supervisor.cancel(session_id) { Ok(())=>CommandOutcome::Accepted, Err(error)=>CommandOutcome::Rejected{code:yoctui_protocol::daemon::ProtocolErrorCode::NotFound,message:error,current_generation:daemon_journal.snapshot().generation} },
                        DaemonCommand::ImportTestResults { generation, roots } => match test_supervisor.import_results(generation, roots) { Ok(job_id) => { let event=daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary{id:job_id,kind:yoctui_protocol::daemon::JobKind::Testing,label:"Test result import".into(),lifecycle:yoctui_protocol::daemon::LifecycleState::Connecting,progress_current:None,progress_total:None,exit_code:None}))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted }, Err(error)=>CommandOutcome::Rejected{code:yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,message:error,current_generation:daemon_journal.snapshot().generation} },
                        DaemonCommand::CompareTestResults { generation, baseline_identity, candidate_identity } => match test_supervisor.compare_results(generation, baseline_identity, candidate_identity) { Ok(job_id)=>{let event=daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary{id:job_id,kind:yoctui_protocol::daemon::JobKind::Testing,label:"Test comparison".into(),lifecycle:yoctui_protocol::daemon::LifecycleState::Exited,progress_current:None,progress_total:None,exit_code:Some(0)}))?;connection.send(&ServerMessage::Event(event))?;CommandOutcome::Accepted},Err(error)=>CommandOutcome::Rejected{code:yoctui_protocol::daemon::ProtocolErrorCode::NotFound,message:error,current_generation:daemon_journal.snapshot().generation}},
                        DaemonCommand::ExportTestJunit { generation, result_identity, destination } => match test_supervisor.start_junit(generation, result_identity, destination) { Ok(job_id)=>{let event=daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary{id:job_id,kind:yoctui_protocol::daemon::JobKind::Testing,label:"JUnit export".into(),lifecycle:yoctui_protocol::daemon::LifecycleState::Connecting,progress_current:None,progress_total:None,exit_code:None}))?;connection.send(&ServerMessage::Event(event))?;CommandOutcome::Accepted},Err(error)=>CommandOutcome::Rejected{code:yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,message:error,current_generation:daemon_journal.snapshot().generation}},
                        DaemonCommand::InspectTestResultTool { path_directories } => { let capability = yoctui_bitbake::TestResultAdapter::new(path_directories.into_iter().map(std::path::PathBuf::from).collect()).capability(); let wire = match capability { yoctui_model::ResultToolCapability::NotInspected => yoctui_protocol::daemon::DaemonTestResultToolCapability::NotInspected, yoctui_model::ResultToolCapability::Missing => yoctui_protocol::daemon::DaemonTestResultToolCapability::Missing, yoctui_model::ResultToolCapability::Available(path) => yoctui_protocol::daemon::DaemonTestResultToolCapability::Available { executable: path.display().to_string() }, yoctui_model::ResultToolCapability::Failed(message) => yoctui_protocol::daemon::DaemonTestResultToolCapability::Failed { message } }; let event=daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::TestResultTool(wire))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted },
                        DaemonCommand::InspectQaCapability { request } => match daemon_qa::inspect(request.input) { Ok(snapshot) => { let event=daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::QaCapability(snapshot))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted }, Err(error)=>CommandOutcome::Rejected{code:yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,message:error,current_generation:daemon_journal.snapshot().generation} },
                        DaemonCommand::StartQaLayerCheck { session_id, operation_id, check_id, layer_name, layer_root, executable, arguments, report_roots } => match qa_supervisor.start(session_id, operation_id, check_id, layer_name, layer_root, executable, arguments, report_roots) {
                            Ok(job_id) => { let event = daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary { id: job_id, kind: yoctui_protocol::daemon::JobKind::Qa, label: format!("QA layer session {session_id}"), lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting, progress_current: None, progress_total: None, exit_code: None }))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted },
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::CancelQaLayerCheck { session_id } => match qa_supervisor.cancel(session_id) {
                            Ok(()) => CommandOutcome::Accepted,
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::StartQaReportScan { generation, build_directory, paths } => match qa_report_supervisor.start(generation, build_directory, paths) {
                            Ok(job_id) => { let event = daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary { id: job_id, kind: yoctui_protocol::daemon::JobKind::Qa, label: format!("QA report scan {generation}"), lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting, progress_current: None, progress_total: None, exit_code: None }))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted },
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::CancelQaReportScan { generation } => match qa_report_supervisor.cancel(generation) {
                            Ok(()) => CommandOutcome::Accepted,
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::StartSecurityReportScan { generation, paths } => match security_supervisor.start(generation, paths) {
                            Ok(job_id) => { let event = daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary { id: job_id, kind: yoctui_protocol::daemon::JobKind::Security, label: format!("Security report scan {generation}"), lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting, progress_current: None, progress_total: None, exit_code: None }))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted },
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::CancelSecurityReportScan { generation } => match security_supervisor.cancel(generation) {
                            Ok(()) => CommandOutcome::Accepted,
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::StartSecurityPackageMap { session_id, executable, arguments, report_roots } => match security_mapper_supervisor.start(session_id, executable, arguments, report_roots) {
                            Ok(job_id) => { let event = daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary { id: job_id, kind: yoctui_protocol::daemon::JobKind::Security, label: format!("Security package map {session_id}"), lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting, progress_current: None, progress_total: None, exit_code: None }))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted },
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::CancelSecurityPackageMap { session_id } => match security_mapper_supervisor.cancel(session_id) {
                            Ok(()) => CommandOutcome::Accepted,
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::InspectMaintenanceCapability { request, build_directory, sstate_directory, tmp_directory, stamps_directories, executable_search_path } => match daemon_maintenance::inspect(request, build_directory, sstate_directory, tmp_directory, stamps_directories, executable_search_path) {
                            Ok(snapshot) => { let event = daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::MaintenanceSnapshot(snapshot))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted },
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::StartMaintenanceSstateReadiness { session_id, capability_request, operation_id, build_directory, sstate_directory, tmp_directory, stamps_directories, executable_search_path, targets, mode, output, log, timeout_seconds } => match maintenance_supervisor.start_readiness(session_id, capability_request, operation_id, build_directory, sstate_directory, tmp_directory, stamps_directories, executable_search_path, targets, mode, output, log, timeout_seconds) {
                            Ok(job_id) => { let event = daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary { id: job_id, kind: yoctui_protocol::daemon::JobKind::Maintenance, label: format!("Maintenance session {session_id}"), lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting, progress_current: None, progress_total: None, exit_code: None }))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted },
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::CancelMaintenance { session_id } => match maintenance_supervisor.cancel(session_id) {
                            Ok(()) => CommandOutcome::Accepted,
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::StartMaintenanceExternal { session_id, executable, expected_name, arguments, current_directory } => match maintenance_supervisor.start_external(session_id, executable, expected_name, arguments, current_directory) {
                            Ok(job_id) => { let event = daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::JobChanged(yoctui_protocol::daemon::JobSummary { id: job_id, kind: yoctui_protocol::daemon::JobKind::Maintenance, label: format!("Maintenance session {session_id}"), lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting, progress_current: None, progress_total: None, exit_code: None }))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted },
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::InspectMaintenanceServices { request, build_directory, prserv_host, hashserve, hashserve_upstream, signature_handler, executable_search_path, process_root } => match daemon_maintenance::inspect_services(request, build_directory, prserv_host, hashserve, hashserve_upstream, signature_handler, executable_search_path, process_root) {
                            Ok(snapshot) => { let event = daemon_journal.publish(yoctui_protocol::daemon::DaemonEvent::MaintenanceSnapshot(snapshot))?; connection.send(&ServerMessage::Event(event))?; CommandOutcome::Accepted },
                            Err(error) => CommandOutcome::Rejected { code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage, message: error, current_generation: daemon_journal.snapshot().generation },
                        },
                        DaemonCommand::CreatePty {
                            name,
                            kind,
                            cwd,
                            command,
                            dimensions,
                        } => match pty_supervisor.start_new(name.clone(), kind, cwd.clone(), command, dimensions) {
                            Ok(session_id) => {
                                let event = daemon_journal.publish(
                                    yoctui_protocol::daemon::DaemonEvent::PtyChanged(
                                        yoctui_protocol::daemon::PtySessionSummary {
                                            id: yoctui_protocol::daemon::PtySessionId(session_id.0),
                                            name,
                                            kind,
                                            cwd,
                                            lifecycle: yoctui_protocol::daemon::LifecycleState::Connecting,
                                            dimensions,
                                            writer: None,
                                            writer_epoch: 0,
                                            viewers: 0,
                                            exit_code: None,
                                            restartable: true,
                                        },
                                    ),
                                )?;
                                connection.send(&ServerMessage::Event(event))?;
                                CommandOutcome::Accepted
                            }
                            Err(message) => CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                                message,
                                current_generation: daemon_journal.snapshot().generation,
                            },
                        },
                        DaemonCommand::TakePtyControl { session_id, expected_epoch } => match pty_supervisor
                            .take(yoctui_model::PtySessionId(session_id.0), yoctui_model::PtyClientId(client_id.0), expected_epoch)
                        {
                            Ok(_) => CommandOutcome::Accepted,
                            Err(message) => CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                message,
                                current_generation: daemon_journal.snapshot().generation,
                            },
                        },
                        DaemonCommand::ReleasePtyControl { session_id, expected_epoch } => match pty_supervisor
                            .release(yoctui_model::PtySessionId(session_id.0), yoctui_model::PtyClientId(client_id.0), expected_epoch)
                        {
                            Ok(()) => CommandOutcome::Accepted,
                            Err(message) => CommandOutcome::Rejected {
                                code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                message,
                                current_generation: daemon_journal.snapshot().generation,
                            },
                        },
                        DaemonCommand::TerminatePty { session_id, force, .. } => {
                            if !force {
                                CommandOutcome::Rejected {
                                    code: yoctui_protocol::daemon::ProtocolErrorCode::ConfirmationRequired,
                                    message: "PTY termination requires an explicit confirmed force request".into(),
                                    current_generation: daemon_journal.snapshot().generation,
                                }
                            } else {
                            if let Some(daemon_raw::DaemonRawCancel::Pty { state, .. }) =
                                raw_supervisor.cancel_pty(yoctui_model::PtySessionId(session_id.0))?
                            {
                                let event = daemon_journal.publish(
                                    yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(
                                        Box::new(
                                            yoctui_app::raw_execution_snapshot_to_protocol(&state)
                                                .map_err(anyhow::Error::msg)?,
                                        ),
                                    ),
                                )?;
                                connection.send(&ServerMessage::Event(event))?;
                            }
                            match pty_supervisor.terminate(yoctui_model::PtySessionId(session_id.0)) {
                                Ok(()) => CommandOutcome::Accepted,
                                Err(message) => CommandOutcome::Rejected {
                                    code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                    message,
                                    current_generation: daemon_journal.snapshot().generation,
                                },
                            }
                            }
                        }
                        DaemonCommand::RenamePty { session_id, name } => {
                            match pty_supervisor.rename(
                                yoctui_model::PtySessionId(session_id.0),
                                name,
                            ) {
                                Ok(()) => CommandOutcome::Accepted,
                                Err(message) => CommandOutcome::Rejected {
                                    code: yoctui_protocol::daemon::ProtocolErrorCode::MalformedMessage,
                                    message,
                                    current_generation: daemon_journal.snapshot().generation,
                                },
                            }
                        }
                        DaemonCommand::ClosePty { session_id } => {
                            let terminal = daemon_journal
                                .snapshot()
                                .pty_sessions
                                .iter()
                                .find(|terminal| terminal.id == session_id);
                            if terminal.is_none() {
                                CommandOutcome::Rejected {
                                    code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                    message: format!("unknown PTY session {}", session_id.0),
                                    current_generation: daemon_journal.snapshot().generation,
                                }
                            } else if terminal.is_some_and(|terminal| {
                                matches!(
                                    terminal.lifecycle,
                                    yoctui_protocol::daemon::LifecycleState::Connecting
                                        | yoctui_protocol::daemon::LifecycleState::Running
                                        | yoctui_protocol::daemon::LifecycleState::Stopping
                                )
                            }) {
                                CommandOutcome::Rejected {
                                    code: yoctui_protocol::daemon::ProtocolErrorCode::ConfirmationRequired,
                                    message: "running PTY must be explicitly terminated before close".into(),
                                    current_generation: daemon_journal.snapshot().generation,
                                }
                            } else {
                                match pty_supervisor
                                    .close(yoctui_model::PtySessionId(session_id.0))
                                {
                                    Ok(()) => {
                                        let event = daemon_journal.publish(
                                            yoctui_protocol::daemon::DaemonEvent::PtyRemoved {
                                                session_id,
                                            },
                                        )?;
                                        connection.send(&ServerMessage::Event(event))?;
                                        CommandOutcome::Accepted
                                    }
                                    Err(message) => CommandOutcome::Rejected {
                                        code: yoctui_protocol::daemon::ProtocolErrorCode::NotFound,
                                        message,
                                        current_generation: daemon_journal.snapshot().generation,
                                    },
                                }
                            }
                        }
                        _ => CommandOutcome::Rejected {
                            code: yoctui_protocol::daemon::ProtocolErrorCode::UnsupportedCapability,
                            message: "daemon command is not implemented by this runtime".into(),
                            current_generation: daemon_journal.snapshot().generation,
                        },
                    };
                    connection.send(&ServerMessage::CommandResult(CommandResult {
                        request_id: request.request_id,
                        outcome,
                    }))?;
                    // Command-created events are sent directly above. Advance
                    // this client's replay cursor so the next fan-out pass
                    // does not duplicate them.
                    last_sequence = daemon_journal.snapshot().sequence;
                }
                Ok(_) => connection.send(&ServerMessage::Error(
                    yoctui_protocol::daemon::ProtocolFailure {
                        request_id: None,
                        code: yoctui_protocol::daemon::ProtocolErrorCode::UnsupportedCapability,
                        message: "complete the daemon handshake before attach; this runtime currently supports state attach, detach, and graceful shutdown".into(),
                        retryable: false,
                    },
                ))?,
                Err(IpcError::Timeout(_)) => break,
                Err(IpcError::Disconnected) => {
                    pty_supervisor.disconnect_client(yoctui_model::PtyClientId(client_id.0));
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
            if keep_client {
                remaining_clients.push((
                    connection,
                    negotiated,
                    attached,
                    last_sequence,
                    client_id,
                    rootfs_query,
                ));
            } else if let Some(mut query) = rootfs_query {
                query.cancel();
                retired_rootfs_queries.push(query);
            }
        }
        clients = remaining_clients;
    }
    write_persisted_state(
        &persist_paths,
        &DaemonPersistedState::capture(
            daemon_journal.snapshot(),
            unix_ms(),
            record.boot_id.clone(),
            Vec::new(),
            PersistedPreferences::default(),
        ),
    )?;
    archive_recorder.finish().await;
    archive_recorder
        .poll(daemon_journal.snapshot(), unix_ms())
        .await;
    archive_recorder.finish().await;
    startup_compatibility.shutdown().await;
    if let Some(scan) = &mut startup_metadata {
        scan.shutdown().await;
    }
    for (_, _, _, _, _, query) in &mut clients {
        if let Some(query) = query {
            query.shutdown().await;
        }
    }
    for query in &mut retired_rootfs_queries {
        query.shutdown().await;
    }
    remove_runtime_record(&paths, instance)?;
    std::mem::forget(record_guard);
    drop(listener);
    Ok(())
}
