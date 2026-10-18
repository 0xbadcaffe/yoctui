use super::*;

fn large_raw(id: usize, character: char) -> RawExecutionSnapshotData {
    let mut execution = raw_execution_snapshot_fixture(1);
    execution.request.request_id = format!("raw-request:pressure-{id}");
    for output in [&mut execution.stdout, &mut execution.stderr] {
        output.chunks = (1..=16)
            .map(|sequence| RawOutputChunkData {
                schema_version: RAW_EXECUTION_SCHEMA_VERSION,
                stream_id: output.stream_id.clone(),
                stream: output.stream,
                sequence,
                text: character
                    .to_string()
                    .repeat(MAX_RAW_EXECUTION_OUTPUT_CHUNK_BYTES),
                truncated_bytes: 0,
                dropped_lines: 0,
            })
            .collect();
        output.next_sequence = 17;
        output.retained_bytes = MAX_RAW_EXECUTION_RETAINED_BYTES as u64;
        output.retained_lines = 16;
        output.dropped_bytes = 0;
        output.dropped_lines = 0;
        output.truncated_chunks = 0;
    }
    execution.validate().unwrap();
    execution
}

fn assert_wire_snapshot(journal: &DaemonSnapshotJournal) {
    let snapshot = journal.snapshot().clone();
    let encoded = serde_json::to_vec(&snapshot).unwrap();
    assert!(encoded.len() <= MAX_SNAPSHOT_BYTES);
    assert!(journal.ipc_metrics().snapshot_bytes_upper_bound >= encoded.len());
    encode_frame(&ServerMessage::Attached {
        replayed_through: snapshot.sequence,
        snapshot: snapshot.clone(),
    })
    .unwrap();
    encode_frame(&ServerMessage::Snapshot(snapshot)).unwrap();
}

#[test]
fn raw_output_pressure_preserves_requests_and_gap_free_live_replay() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    let mut replay = daemon_snapshot_fixture();
    for id in 0..4 {
        let event = journal
            .publish(DaemonEvent::RawExecutionChanged(Box::new(large_raw(
                id, 'x',
            ))))
            .unwrap();
        encode_frame(&ServerMessage::Event(event.clone())).unwrap();
        apply_sequenced_event(&mut replay, &event).unwrap();
        assert_eq!(event.sequence, id as u64 + 1);
        assert_wire_snapshot(&journal);
    }
    assert_eq!(journal.snapshot().raw_executions.len(), 4);
    assert_eq!(replay.raw_executions.len(), 4);
    assert!(
        journal
            .snapshot()
            .raw_executions
            .iter()
            .any(|raw| raw.stdout.dropped_bytes > 0)
    );
    for raw in &journal.snapshot().raw_executions {
        raw.validate().unwrap();
        for output in [&raw.stdout, &raw.stderr] {
            assert_eq!(output.retained_bytes + output.dropped_bytes, 1024 * 1024);
            assert_eq!(output.retained_lines + output.dropped_lines, 16);
        }
    }
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Completed {
            success: true,
            exit_code: Some(0),
            finished_unix_ms: Some(123),
        }))
        .unwrap();
    assert_wire_snapshot(&journal);
    assert!(matches!(
        journal.snapshot().build_events.last(),
        Some(DaemonBuildEvent::Completed {
            success: true,
            exit_code: Some(0),
            ..
        })
    ));
}

#[test]
fn replay_is_byte_bounded_and_expired_cursors_receive_a_valid_checkpoint() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    for id in 0..10 {
        journal
            .publish(DaemonEvent::RawExecutionChanged(Box::new(large_raw(
                id, 'x',
            ))))
            .unwrap();
        assert!(journal.retained_event_bytes <= MAX_RETAINED_EVENT_BYTES);
    }
    assert!(journal.events.len() < 10);
    assert_eq!(journal.events.len(), journal.event_sizes.len());
    assert_eq!(
        journal.retained_event_bytes,
        journal.event_sizes.iter().sum::<usize>()
    );
    assert!(matches!(
        journal.synchronize(Some(ResumeCursor {
            daemon_instance_id: journal.snapshot().daemon_instance_id,
            last_sequence: 0,
        })),
        DaemonSnapshotSync::Replace {
            reason: SnapshotReplacementReason::HistoryExpired,
            ..
        }
    ));
    assert_wire_snapshot(&journal);
}

#[test]
fn escaped_raw_output_is_bounded_before_event_framing() {
    let mut journal =
        DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
            .unwrap();
    let event = journal
        .publish(DaemonEvent::RawExecutionChanged(Box::new(large_raw(
            0, '\0',
        ))))
        .unwrap();
    encode_frame(&ServerMessage::Event(event)).unwrap();
    assert_wire_snapshot(&journal);
    let raw = &journal.snapshot().raw_executions[0];
    raw.validate().unwrap();
    assert!(raw.stdout.dropped_bytes + raw.stderr.dropped_bytes > 0);
    assert_eq!(raw.sequence, 1);
    assert_eq!(raw.stdout.next_sequence, 17);
    assert_eq!(raw.stderr.next_sequence, 17);
}

#[test]
fn oversized_recovered_history_is_compacted_before_serving_attach() {
    let mut snapshot = daemon_snapshot_fixture();
    snapshot.raw_executions = (0..4).map(|id| large_raw(id, 'x')).collect();
    assert!(serde_json::to_vec(&snapshot).unwrap().len() > MAX_FRAME_BYTES);
    let journal = DaemonSnapshotJournal::new(snapshot, DaemonSnapshotLimits::default()).unwrap();
    assert_wire_snapshot(&journal);
    assert_eq!(journal.snapshot().raw_executions.len(), 4);
    for raw in &journal.snapshot().raw_executions {
        raw.validate().unwrap();
    }
}

#[test]
fn byte_eviction_preserves_build_outcome_workspace_and_aggregate_counts() {
    let mut journal = DaemonSnapshotJournal::new(
        daemon_snapshot_fixture(),
        DaemonSnapshotLimits {
            snapshot_bytes: 4000,
            ..DaemonSnapshotLimits::default()
        },
    )
    .unwrap();
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Workspace {
            data: WorkspaceData {
                build_dir: Some("/build".into()),
                source_dir: Some("/source".into()),
                variables: Default::default(),
                variable_provenance: Default::default(),
                variable_provenance_chain: Default::default(),
                bitbake_version: None,
                release: None,
                layers: Vec::new(),
                recipes: Vec::new(),
            },
        }))
        .unwrap();
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Started {
            started_unix_ms: Some(10),
        }))
        .unwrap();
    for id in 0..40 {
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::TaskCompleted {
                recipe: format!("recipe-{id}"),
                task: "do_build".into(),
                success: true,
                started_unix_ms: Some(10),
                finished_unix_ms: Some(20),
            }))
            .unwrap();
    }
    journal
        .publish(DaemonEvent::Build(DaemonBuildEvent::Completed {
            success: true,
            exit_code: Some(0),
            finished_unix_ms: Some(30),
        }))
        .unwrap();
    let snapshot = journal.snapshot();
    assert!(serde_json::to_vec(snapshot).unwrap().len() <= 4000);
    assert_eq!(snapshot.build_progress.unwrap().completed, 40);
    assert!(
        snapshot
            .build_events
            .iter()
            .any(|event| matches!(event, DaemonBuildEvent::Workspace { .. }))
    );
    assert!(
        snapshot
            .build_events
            .iter()
            .any(|event| matches!(event, DaemonBuildEvent::Started { .. }))
    );
    assert!(matches!(
        snapshot.build_events.last(),
        Some(DaemonBuildEvent::Completed { success: true, .. })
    ));
}
