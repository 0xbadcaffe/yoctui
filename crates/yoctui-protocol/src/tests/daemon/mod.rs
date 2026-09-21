use super::*;

fn client_id(byte: u8) -> ClientId {
    ClientId([byte; 16])
}

fn daemon_snapshot_fixture() -> DaemonSnapshot {
    DaemonSnapshot {
        daemon_instance_id: DaemonInstanceId([7; 16]),
        sequence: 0,
        generation: 0,
        workspace: None,
        project_profile: ProjectProfileSummary::Absent,
        bitbake: BitBakeState {
            lifecycle: LifecycleState::Disconnected,
            version: None,
            capabilities: Vec::new(),
            diagnostic: None,
        },
        compatibility: None,
        jobs: Vec::new(),
        raw_executions: Vec::new(),
        raw_history: Vec::new(),
        pty_sessions: Vec::new(),
        pty_screens: Vec::new(),
        clients: Vec::new(),
        recent_logs: Vec::new(),
        build_events: Vec::new(),
        build_progress: None,
        recovery_warnings: Vec::new(),
    }
}

fn compatibility_environment_fixture() -> CompatibilityEnvironmentIdentity {
    CompatibilityEnvironmentIdentity {
        build_directory: CompatibilityDetected::Detected {
            value: "/work/poky/build".into(),
            authority: CompatibilityIdentityAuthority::InitializedEnvironment,
        },
        source_roots: CompatibilityDetected::Unknown,
        bitbake_version: CompatibilityDetected::Detected {
            value: "2.18.0".into(),
            authority: CompatibilityIdentityAuthority::BitBakeVersionProbe,
        },
        oe_core: CompatibilityDetected::Unknown,
        poky: CompatibilityDetected::Unknown,
        distro: CompatibilityDetected::Unknown,
        machine: CompatibilityDetected::Unknown,
        layer_series: CompatibilityDetected::Unknown,
        available_tools: CompatibilityDetected::Unknown,
        backend: CompatibilityDetected::Unknown,
        protocol: CompatibilityDetected::Detected {
            value: CompatibilityProtocolIdentity {
                name: "yoctui-daemon".into(),
                version: "1.0".into(),
            },
            authority: CompatibilityIdentityAuthority::ProtocolNegotiation,
        },
    }
}

fn compatibility_snapshot_fixture(generation: u64) -> CompatibilitySnapshotData {
    CompatibilitySnapshotData {
        schema_version: COMPATIBILITY_SCHEMA_VERSION,
        generation,
        environment: compatibility_environment_fixture(),
        capabilities: vec![CompatibilityCapabilityData {
            id: "bitbake.getvar".into(),
            state: CompatibilityStateData::Available,
            evidence: vec![CompatibilityEvidenceData {
                kind: CompatibilityEvidenceKind::DirectProbe,
                outcome: CompatibilityEvidenceOutcome::Positive,
                subject: "bitbake --help".into(),
                detail: "getvar option present".into(),
                argv: vec!["bitbake".into(), "--help".into()],
            }],
            implementation: Some(CompatibilityImplementationData {
                id: "bitbake.getvar.native".into(),
                kind: "native".into(),
            }),
        }],
    }
}

mod compatibility_snapshot_round_trips_bounded_identity_state_and_evidence;

mod compatibility_snapshot_accepts_case_preserving_distro_identity;

mod nested_build_event_round_trips_without_duplicate_type_fields;

mod compatibility_validation_rejects_duplicate_oversized_and_unsupported_evidence;

mod compatibility_unknown_wire_values_fail_closed;

mod compatibility_events_replace_newer_snapshots_and_reject_stale_generations;

mod daemon_protocol_negotiates_versions_and_capabilities_explicitly;

mod daemon_protocol_round_trips_snapshot_event_and_correlated_command;

mod daemon_protocol_frames_partial_messages_and_rejects_oversize;

mod daemon_protocol_covers_reconnect_stale_writer_layout_and_mouse;

mod multi_client_state_keeps_identity_global_and_layout_local;

mod multi_client_fanout_replays_global_events_from_independent_cursors;

mod next_generation_pty_screen_is_bounded_and_retained_for_reattach;

mod daemon_snapshot_is_gap_free_bounded_and_replays_only_retained_events;

mod daemon_snapshot_evicts_oldest_logs_to_preserve_byte_bound;

mod daemon_snapshot_rejects_invalid_limits_and_oversized_snapshots;

mod snapshot_progress_keeps_counts_after_eviction_and_duplicate_completion;

mod snapshot_progress_preserves_unknown_totals_and_legacy_absence;

mod snapshot_timing_compacts_start_into_completion_and_freezes_duplicates;

mod task_identity_unresolved_job_statistics_preserve_aggregate_without_rows;

mod snapshot_timing_retained_rows_stay_bounded_after_eviction;

mod snapshot_timing_keeps_legacy_wire_shapes_and_rejects_malformed_times;

mod daemon_build_snapshot_retains_typed_attach_progress;

mod daemon_journal_uses_conservative_headroom_between_snapshot_serializations;

mod daemon_journal_updates_high_rate_job_progress_without_full_snapshot_serialization;

fn journal_telemetry(uptime_seconds: u64) -> DaemonEvent {
    DaemonEvent::Telemetry(DaemonTelemetry {
        uptime_seconds,
        bitbake: LifecycleState::Running,
        connected_clients: 2,
        active_jobs: 1,
        pty_sessions: 0,
        queue_depth: 1,
        pressure: DaemonPressureCounters::default(),
        memory_bytes: Some(24 * 1024 * 1024),
        recovery: DaemonRecoveryState::CleanStart,
    })
}

mod daemon_journal_telemetry_replays_without_serializing_unchanged_snapshot;

mod daemon_journal_telemetry_remeasures_at_the_same_hard_byte_limit;

mod daemon_journal_telemetry_size_rejection_preserves_snapshot_and_replay;

mod daemon_journal_telemetry_counter_exhaustion_is_transactional;

mod daemon_workspace_event_updates_persistent_workspace_identity;

fn raw_execution_request_fixture() -> RawExecutionRequestData {
    RawExecutionRequestData {
        schema_version: RAW_EXECUTION_SCHEMA_VERSION,
        request_id: "raw-request:protocol-1".into(),
        catalog_version: 1,
        command_id: "build.target".into(),
        parameters: vec![RawExecutionParameterData {
            id: "target".into(),
            value: RawParameterValueData::Target("core-image-minimal".into()),
        }],
        additional_arguments: vec!["--dry-run".into()],
        interaction: RawInteractionData::NoninteractiveJob,
        safety: RawSafetyData::Build,
        capability_generation: 7,
        build_directory: "/work/build".into(),
        preview_digest: "ab".repeat(32),
    }
}

fn raw_execution_chunk_fixture() -> RawOutputChunkData {
    RawOutputChunkData {
        schema_version: RAW_EXECUTION_SCHEMA_VERSION,
        stream_id: "raw-stream:stdout-1".into(),
        stream: RawOutputStreamData::Stdout,
        sequence: 1,
        text: "héllo\n".into(),
        truncated_bytes: 2,
        dropped_lines: 1,
    }
}

fn raw_execution_snapshot_fixture(sequence: u64) -> RawExecutionSnapshotData {
    let chunk = raw_execution_chunk_fixture();
    RawExecutionSnapshotData {
        schema_version: RAW_EXECUTION_SCHEMA_VERSION,
        request: raw_execution_request_fixture(),
        phase: RawExecutionPhaseData::Running,
        attachment: RawAttachmentData::Detached,
        owner: Some(RawExecutionOwnerData::Job("raw-job:protocol-1".into())),
        cancellation_requested: false,
        queued_unix_ms: 10,
        started_unix_ms: Some(20),
        elapsed_ms: 30,
        result: None,
        stdout: RawRetainedOutputData {
            stream_id: chunk.stream_id.clone(),
            stream: RawOutputStreamData::Stdout,
            retained_bytes: chunk.text.len() as u64,
            retained_lines: raw_protocol_line_count(&chunk.text) as u64,
            chunks: vec![chunk],
            next_sequence: 2,
            dropped_bytes: 2,
            dropped_lines: 1,
            truncated_chunks: 1,
        },
        stderr: RawRetainedOutputData {
            stream_id: "raw-stream:stderr-1".into(),
            stream: RawOutputStreamData::Stderr,
            chunks: Vec::new(),
            next_sequence: 1,
            retained_bytes: 0,
            retained_lines: 0,
            dropped_bytes: 0,
            dropped_lines: 0,
            truncated_chunks: 0,
        },
        sequence,
        generation: sequence,
    }
}

mod raw_execution_protocol_round_trips_request_event_chunk_snapshot_and_result;

mod raw_job_start_and_cancel_preserve_request_correlation;

mod raw_pty_start_preserves_only_confirmed_intent_and_dimensions;

mod raw_output_attachment_command_preserves_request_correlation;

mod raw_execution_protocol_rejects_unknown_cross_kind_and_unicode_byte_overflow;

mod raw_execution_protocol_snapshot_journal_replaces_newer_and_rejects_stale;

fn terminal_raw_snapshot(
    sequence: u64,
    outcome: RawExecutionOutcomeData,
) -> RawExecutionSnapshotData {
    let mut snapshot = raw_execution_snapshot_fixture(sequence);
    snapshot.phase = RawExecutionPhaseData::Terminal(outcome);
    snapshot.cancellation_requested = outcome == RawExecutionOutcomeData::Cancelled;
    snapshot.elapsed_ms = 50 + sequence;
    snapshot.result = Some(RawExecutionResultData {
        schema_version: RAW_EXECUTION_SCHEMA_VERSION,
        outcome,
        exit_code: match outcome {
            RawExecutionOutcomeData::Succeeded => Some(0),
            RawExecutionOutcomeData::Failed => Some(2),
            RawExecutionOutcomeData::Cancelled | RawExecutionOutcomeData::Lost => None,
            RawExecutionOutcomeData::Unknown => unreachable!(),
        },
        message: Some("must not be retained".into()),
        elapsed_ms: snapshot.elapsed_ms,
        durable_reference: Some("raw-durable:protocol-history".into()),
    });
    snapshot
}

mod raw_history_records_all_terminal_outcomes_without_live_authority;

mod raw_history_journal_updates_duplicate_request_idempotently_newest_first;

mod raw_history_rejects_unknown_schema_count_aggregate_and_order;

mod resource_limits_are_explicit_and_bounded;

mod image_console_pty_kinds_round_trip_without_collapsing_identity;
