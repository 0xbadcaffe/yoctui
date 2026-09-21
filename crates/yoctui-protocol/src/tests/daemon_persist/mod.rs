use super::*;
use crate::daemon::{
    BitBakeState, LifecycleState, RAW_EXECUTION_SCHEMA_VERSION, RawExecutionOwnerData,
    RawInteractionData, RawOutputChunkData, RawOutputStreamData, RawParameterValueData,
    RawRetainedOutputData, RawSafetyData,
};
use std::os::unix::fs::DirBuilderExt;

fn root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "yoctui-daemon-persist-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::DirBuilder::new()
        .mode(PRIVATE_DIRECTORY_MODE)
        .create(&path)
        .unwrap();
    path
}

fn raw_job_snapshot() -> RawExecutionSnapshotData {
    let chunk = RawOutputChunkData {
        schema_version: RAW_EXECUTION_SCHEMA_VERSION,
        stream_id: "raw-stream:persist-stdout".into(),
        stream: RawOutputStreamData::Stdout,
        sequence: 1,
        text: "retained before restart\n".into(),
        truncated_bytes: 0,
        dropped_lines: 0,
    };
    RawExecutionSnapshotData {
        schema_version: RAW_EXECUTION_SCHEMA_VERSION,
        request: crate::daemon::RawExecutionRequestData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            request_id: "raw-request:persist-1".into(),
            catalog_version: 1,
            command_id: "build.target".into(),
            parameters: vec![crate::daemon::RawExecutionParameterData {
                id: "target".into(),
                value: RawParameterValueData::Target("core-image-minimal".into()),
            }],
            additional_arguments: Vec::new(),
            interaction: RawInteractionData::NoninteractiveJob,
            safety: RawSafetyData::Build,
            capability_generation: 7,
            build_directory: "/work/build".into(),
            preview_digest: "ab".repeat(32),
        },
        phase: RawExecutionPhaseData::Running,
        attachment: RawAttachmentData::Attached,
        owner: Some(RawExecutionOwnerData::Job("raw-job:persist-1".into())),
        cancellation_requested: false,
        queued_unix_ms: 10,
        started_unix_ms: Some(20),
        elapsed_ms: 30,
        result: None,
        stdout: RawRetainedOutputData {
            stream_id: chunk.stream_id.clone(),
            stream: RawOutputStreamData::Stdout,
            retained_bytes: chunk.text.len() as u64,
            retained_lines: 1,
            chunks: vec![chunk],
            next_sequence: 2,
            dropped_bytes: 0,
            dropped_lines: 0,
            truncated_chunks: 0,
        },
        stderr: RawRetainedOutputData {
            stream_id: "raw-stream:persist-stderr".into(),
            stream: RawOutputStreamData::Stderr,
            chunks: Vec::new(),
            next_sequence: 1,
            retained_bytes: 0,
            retained_lines: 0,
            dropped_bytes: 0,
            dropped_lines: 0,
            truncated_chunks: 0,
        },
        sequence: 2,
        generation: 2,
    }
}

fn snapshot() -> DaemonSnapshot {
    DaemonSnapshot {
        daemon_instance_id: DaemonInstanceId([3; 16]),
        sequence: 8,
        generation: 5,
        workspace: None,
        project_profile: ProjectProfileSummary::Absent,
        bitbake: BitBakeState {
            lifecycle: LifecycleState::Running,
            version: Some("2.8.1".into()),
            capabilities: vec![BitBakeCapability::WorkspaceInspection],
            diagnostic: None,
        },
        compatibility: None,
        jobs: Vec::new(),
        raw_executions: Vec::new(),
        raw_history: Vec::new(),
        pty_sessions: Vec::new(),
        pty_screens: Vec::new(),
        clients: Vec::new(),
        recent_logs: vec![LogRecord {
            source: "test".into(),
            severity: crate::daemon::LogSeverity::Info,
            message: "retained".into(),
            unix_ms: 1,
            recipe: None,
            task: None,
            path: None,
            build: None,
        }],
        build_events: Vec::new(),
        build_progress: None,
        recovery_warnings: Vec::new(),
    }
}

mod daemon_persist_round_trips_private_bounded_metadata_without_processes;

mod daemon_persist_rejects_symlinks_oversize_and_live_process_claims;

mod daemon_recovery_marks_unrecoverable_work_lost_and_clears_live_attachments;

mod daemon_recovery_keeps_current_workspace_over_stale_persisted_identity;

mod raw_job_recovery_sanitizes_output_and_maps_running_work_to_lost_once;
