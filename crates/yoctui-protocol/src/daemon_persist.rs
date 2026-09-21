//! Safe, bounded persistence for reconstructable daemon metadata.
use crate::daemon::{
    BitBakeCapability, BitBakeState, DaemonInstanceId, DaemonSnapshot, JobSummary, LifecycleState,
    LogRecord, MAX_RAW_EXECUTION_REQUESTS, ProjectProfileSummary, PtyKind, RawAttachmentData,
    RawExecutionOutcomeData, RawExecutionPhaseData, RawExecutionResultData,
    RawExecutionSnapshotData, RawHistoryRecordData, TerminalDimensions, WorkspaceIdentity,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use thiserror::Error;

pub const DAEMON_PERSIST_SCHEMA_VERSION: u32 = 1;
pub const MAX_DAEMON_PERSIST_BYTES: u64 = 4 * 1024 * 1024;
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(1);

include!("daemon_persist/recovery_state.rs");
include!("daemon_persist/storage.rs");

#[cfg(test)]
mod tests {
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

    #[test]
    fn daemon_persist_round_trips_private_bounded_metadata_without_processes() {
        let root = root("round-trip");
        let paths = persist_paths_for(&root).unwrap();
        let persisted = DaemonPersistedState::capture(
            &snapshot(),
            99,
            "boot".into(),
            vec![PersistedClientLayout {
                client_key: "terminal".into(),
                layout_revision: 2,
                session_names: vec!["build".into()],
            }],
            PersistedPreferences {
                theme: Some("dark".into()),
                prefix_key: Some("Ctrl-b".into()),
                recent_logs_enabled: true,
            },
        );
        write_persisted_state(&paths, &persisted).unwrap();
        assert_eq!(read_persisted_state(&paths).unwrap(), Some(persisted));
        assert_eq!(
            fs::symlink_metadata(&paths.state)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            PRIVATE_FILE_MODE
        );
        let text = fs::read_to_string(&paths.state).unwrap();
        assert!(!text.contains("\"pid\""));
        assert!(!text.contains("process_group"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn daemon_persist_rejects_symlinks_oversize_and_live_process_claims() {
        let root = root("unsafe");
        let paths = persist_paths_for(&root).unwrap();
        std::os::unix::fs::symlink("/tmp", &paths.state).unwrap();
        assert!(matches!(
            read_persisted_state(&paths),
            Err(DaemonPersistError::Unsafe { .. })
        ));
        fs::remove_file(&paths.state).unwrap();
        fs::write(
            &paths.state,
            vec![b'x'; MAX_DAEMON_PERSIST_BYTES as usize + 1],
        )
        .unwrap();
        fs::set_permissions(&paths.state, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        assert!(matches!(
            read_persisted_state(&paths),
            Err(DaemonPersistError::TooLarge)
        ));
        fs::remove_file(&paths.state).unwrap();

        let mut persisted = DaemonPersistedState::capture(
            &snapshot(),
            99,
            "boot".into(),
            Vec::new(),
            PersistedPreferences::default(),
        );
        persisted.terminal_sessions.push(PersistedTerminalSession {
            id: 1,
            name: "unsafe".into(),
            kind: PtyKind::Utility,
            cwd: "/tmp".into(),
            previous_lifecycle: LifecycleState::Running,
            dimensions: TerminalDimensions {
                columns: 80,
                rows: 24,
            },
            exit_code: None,
            restartable: true,
            live_process_persisted: true,
        });
        assert!(matches!(
            write_persisted_state(&paths, &persisted),
            Err(DaemonPersistError::Unsafe { .. })
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn daemon_recovery_marks_unrecoverable_work_lost_and_clears_live_attachments() {
        let mut prior = snapshot();
        prior.jobs.push(crate::daemon::JobSummary {
            id: crate::daemon::JobId(1),
            kind: crate::daemon::JobKind::BitBakeBuild,
            label: "core-image-minimal".into(),
            lifecycle: LifecycleState::Running,
            progress_current: Some(2),
            progress_total: Some(10),
            exit_code: None,
        });
        prior.pty_sessions.push(crate::daemon::PtySessionSummary {
            id: crate::daemon::PtySessionId(7),
            name: "devshell".into(),
            kind: PtyKind::Devshell,
            cwd: "/work/build".into(),
            lifecycle: LifecycleState::Running,
            dimensions: TerminalDimensions {
                columns: 120,
                rows: 40,
            },
            writer: Some(crate::daemon::ClientId([9; 16])),
            writer_epoch: 4,
            viewers: 2,
            exit_code: None,
            restartable: true,
        });
        let persisted = DaemonPersistedState::capture(
            &prior,
            99,
            "old-boot".into(),
            Vec::new(),
            PersistedPreferences::default(),
        );
        let mut current = snapshot();
        current.daemon_instance_id = DaemonInstanceId([8; 16]);
        current.sequence = 0;
        current.generation = 0;
        let (recovered, report) = recover_persisted_snapshot(current, &persisted, "new-boot");

        assert_eq!(recovered.daemon_instance_id, DaemonInstanceId([8; 16]));
        assert_eq!(recovered.jobs[0].lifecycle, LifecycleState::Lost);
        assert_eq!(recovered.pty_sessions[0].lifecycle, LifecycleState::Lost);
        assert_eq!(recovered.pty_sessions[0].writer, None);
        assert_eq!(recovered.pty_sessions[0].viewers, 0);
        assert_eq!(recovered.bitbake.lifecycle, LifecycleState::Disconnected);
        assert!(recovered.bitbake.diagnostic.is_some());
        assert!(recovered.clients.is_empty());
        assert_eq!(report.lost_jobs, 1);
        assert_eq!(report.lost_terminal_sessions, 1);
        assert!(report.previous_boot_changed);
        assert_eq!(report.boundary, DaemonRecoveryBoundary::HostReboot);
        assert!(report.bitbake_reconnect_recommended);
        assert_eq!(report.terminal_relaunch_intents.len(), 1);
        assert_eq!(report.terminal_relaunch_intents[0].name, "devshell");
        assert_eq!(report.terminal_relaunch_intents[0].cwd, "/work/build");
    }

    #[test]
    fn daemon_recovery_keeps_current_workspace_over_stale_persisted_identity() {
        let mut prior = snapshot();
        prior.workspace = Some(WorkspaceIdentity {
            canonical_source: "/srv/old/poky".into(),
            canonical_build: "/srv/old/build".into(),
            identity_hash: "old-workspace".into(),
        });
        let persisted = DaemonPersistedState::capture(
            &prior,
            99,
            "same-boot".into(),
            Vec::new(),
            PersistedPreferences::default(),
        );
        let current_workspace = WorkspaceIdentity {
            canonical_source: "/srv/current/poky".into(),
            canonical_build: "/srv/current/build".into(),
            identity_hash: "current-workspace".into(),
        };
        let mut current = snapshot();
        current.workspace = Some(current_workspace.clone());

        let (recovered, _) = recover_persisted_snapshot(current, &persisted, "same-boot");

        assert_eq!(recovered.workspace, Some(current_workspace));
    }

    #[test]
    fn raw_job_recovery_sanitizes_output_and_maps_running_work_to_lost_once() {
        let mut prior = snapshot();
        prior.raw_executions = vec![raw_job_snapshot()];
        let persisted = DaemonPersistedState::capture(
            &prior,
            99,
            "same-boot".into(),
            Vec::new(),
            PersistedPreferences::default(),
        );
        assert!(persisted.raw_executions[0].stdout.chunks.is_empty());
        assert_eq!(persisted.raw_executions[0].stdout.retained_bytes, 0);
        assert!(persisted.raw_executions[0].stdout.dropped_bytes > 0);

        let (recovered, report) = recover_persisted_snapshot(snapshot(), &persisted, "same-boot");
        let execution = &recovered.raw_executions[0];
        assert_eq!(
            execution.phase,
            RawExecutionPhaseData::Terminal(RawExecutionOutcomeData::Lost)
        );
        assert_eq!(execution.attachment, RawAttachmentData::Detached);
        assert_eq!(
            execution.result.as_ref().unwrap().outcome,
            RawExecutionOutcomeData::Lost
        );
        execution.validate().unwrap();
        assert_eq!(report.lost_raw_executions, 1);
        assert_eq!(recovered.raw_history.len(), 1);
        assert_eq!(
            recovered.raw_history[0].outcome,
            RawExecutionOutcomeData::Lost
        );
        assert_eq!(
            recovered.raw_history[0].request_id,
            execution.request.request_id
        );

        let persisted_again = DaemonPersistedState::capture(
            &recovered,
            100,
            "same-boot".into(),
            Vec::new(),
            PersistedPreferences::default(),
        );
        let (recovered_again, report_again) =
            recover_persisted_snapshot(snapshot(), &persisted_again, "same-boot");
        assert_eq!(report_again.lost_raw_executions, 0);
        assert_eq!(
            recovered_again.raw_executions[0],
            recovered.raw_executions[0]
        );
        assert_eq!(recovered_again.raw_history, recovered.raw_history);
    }
}
