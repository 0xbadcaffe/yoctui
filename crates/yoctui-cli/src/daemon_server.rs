//! Daemon startup, readiness, fan-out and orderly shutdown.
use super::*;
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

mod background;
mod client_requests;
mod commands;
mod state;
mod telemetry;
use state::{ClientConnection, DaemonClient, DaemonServices};

const MAX_SUPERVISOR_EVENTS_PER_TICK: usize = 32;

pub(crate) async fn run_daemon_foreground(
    termination: &mut tokio::sync::mpsc::Receiver<()>,
) -> Result<()> {
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
    let startup_compatibility = daemon_compatibility::spawn_startup(startup_environment.clone());
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
    let sdk_supervisor = daemon_sdk::DaemonSdkSupervisor::new(job_ids.clone());
    let qemu_supervisor = daemon_qemu::DaemonQemuSupervisor::new(job_ids.clone());
    let wic_supervisor = daemon_wic::DaemonWicSupervisor::new(job_ids.clone());
    let test_supervisor = daemon_test::DaemonTestSupervisor::new(job_ids.clone());
    let qa_supervisor = daemon_qa::DaemonQaSupervisor::new(job_ids.clone());
    let qa_report_supervisor = daemon_qa::DaemonQaReportSupervisor::new(job_ids.clone());
    let security_supervisor = daemon_security::DaemonSecuritySupervisor::new(job_ids.clone());
    let security_mapper_supervisor =
        daemon_security::DaemonSecurityMapperSupervisor::new(job_ids.clone());
    let maintenance_supervisor =
        daemon_maintenance::DaemonMaintenanceSupervisor::new(job_ids.clone());
    let pty_supervisor = daemon_pty::DaemonPtySupervisor::default();
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
    let startup_metadata: Option<daemon_metadata::StartupMetadata> = None;
    if startup_configured {
        publish_startup_metadata_log(
            &mut daemon_journal,
            "Loading initial compatibility authority",
            false,
        )?;
    }

    let mut services = DaemonServices {
        daemon_state,
        daemon_journal,
        startup_environment,
        startup_compatibility,
        startup_metadata,
        startup_configured,
        rootfs_environment,
        rootfs_query_permit,
        instance,
        shutting_down: false,
        devtool_supervisor,
        raw_supervisor,
        bitbake_supervisor,
        sdk_supervisor,
        qemu_supervisor,
        wic_supervisor,
        test_supervisor,
        qa_supervisor,
        qa_report_supervisor,
        security_supervisor,
        security_mapper_supervisor,
        maintenance_supervisor,
        pty_supervisor,
    };
    // Connections are serviced in short, bounded slices.  Keeping the
    // negotiated state and replay cursor alongside each socket lets one idle
    // client yield to other clients while the daemon continues polling jobs.
    let mut clients: Vec<ClientConnection> = Vec::new();
    let mut archive_recorder = build_archive::Recorder::new(daemon_state_root()?);
    let mut telemetry = telemetry::TelemetryState {
        recovered: persisted.is_some(),
        last_telemetry_ms: record.started_unix_ms,
        ..Default::default()
    };
    // Keep socket production comfortably below the interactive client's
    // bounded 64-event/8-ms poll. A busy reducer may not consume all 64 before
    // its time bound, so matching the 32-event supervisor ingress can still
    // fill the socket. Cursor expiry is recovered by a replacement Snapshot.
    while !services.shutting_down {
        retired_rootfs_queries.retain(|query| !query.is_finished());
        let client_connections = clients
            .iter()
            .map(|(connection, _, _, _, _, _)| connection)
            .collect::<Vec<_>>();
        listener.wait_for_activity_with_additional_fd(
            &client_connections,
            services.bitbake_supervisor.notification_fd(),
            daemon_service_wait(
                daemon_has_active_work(services.daemon_journal.snapshot())
                    || services.rootfs_query_permit.available_permits() == 0,
            ),
        )?;
        drop(client_connections);
        services.bitbake_supervisor.consume_notification();

        background::poll(&mut services)?;
        telemetry::publish(
            &mut services,
            &mut telemetry,
            &clients,
            &record,
            &mut archive_recorder,
        )
        .await?;
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
                    services.daemon_journal.snapshot().sequence,
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
            negotiated,
            attached,
            mut last_sequence,
            client_id,
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
                    telemetry.slow_client_disconnects =
                        telemetry.slow_client_disconnects.saturating_add(1);
                    services
                        .pty_supervisor
                        .disconnect_client(yoctui_model::PtyClientId(client_id.0));
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
                    let compatibility = services
                        .daemon_state
                        .compatibility
                        .as_ref()
                        .context("rootfs compatibility authority was lost")?;
                    daemon_rootfs::validate_authority(
                        &query.query,
                        services.instance,
                        compatibility,
                    )?;
                    Ok(sources)
                });
                let outcome = match result {
                    Ok(sources) => CommandOutcome::RootfsSources {
                        sources: Box::new(sources),
                    },
                    Err(error) => CommandOutcome::Rejected {
                        code: yoctui_protocol::daemon::ProtocolErrorCode::Conflict,
                        message: format!("rootfs metadata unavailable: {error:#}"),
                        current_generation: services.daemon_journal.snapshot().generation,
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
                match services.daemon_journal.synchronize_bounded(
                    yoctui_protocol::daemon::ResumeCursor {
                        daemon_instance_id: services.instance,
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
                                    telemetry.slow_client_disconnects =
                                        telemetry.slow_client_disconnects.saturating_add(1);
                                    keep_client = false;
                                    break;
                                }
                            }
                        }
                    }
                    yoctui_protocol::daemon::DaemonSnapshotSync::Replace { snapshot, reason } => {
                        telemetry.forced_client_resynchronizations =
                            telemetry.forced_client_resynchronizations.saturating_add(1);
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
                services
                    .pty_supervisor
                    .disconnect_client(yoctui_model::PtyClientId(client_id.0));
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
            let mut client = DaemonClient {
                connection,
                negotiated,
                attached,
                last_sequence,
                client_id,
                rootfs_query,
            };
            let keep_client = client_requests::service(&mut services, &mut client, &record)?;
            let DaemonClient {
                connection,
                negotiated,
                attached,
                last_sequence,
                client_id,
                rootfs_query,
            } = client;
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
            services.daemon_journal.snapshot(),
            unix_ms(),
            record.boot_id.clone(),
            Vec::new(),
            PersistedPreferences::default(),
        ),
    )?;
    archive_recorder.finish().await;
    archive_recorder
        .poll(services.daemon_journal.snapshot(), unix_ms())
        .await;
    archive_recorder.finish().await;
    services.startup_compatibility.shutdown().await;
    if let Some(scan) = &mut services.startup_metadata {
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
    remove_runtime_record(&paths, services.instance)?;
    std::mem::forget(record_guard);
    drop(listener);
    Ok(())
}
