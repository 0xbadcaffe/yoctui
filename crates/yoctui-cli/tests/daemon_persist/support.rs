pub(super) use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
pub(super) use yoctui_protocol::{
    daemon::{
        BitBakeState, Capability, ClientHello, ClientId, ClientMessage, DaemonInstanceId,
        DaemonSnapshot, JobId, JobKind, JobSummary, LifecycleState, ProjectProfileSummary,
        ProtocolVersion, PtyKind, PtySessionId, PtySessionSummary, RAW_HISTORY_SCHEMA_VERSION,
        RawExecutionOutcomeData, RawExecutionParameterData, RawHistoryRecordData,
        RawInteractionData, RawParameterValueData, ServerMessage, Subscription, TerminalDimensions,
    },
    daemon_ipc::{DaemonConnection, runtime_paths_for},
    daemon_lifecycle::read_boot_id,
    daemon_persist::{
        DaemonPersistedState, DaemonRecoveryBoundary, PersistedPreferences, persist_paths_for,
        read_persisted_state, recover_persisted_snapshot, write_persisted_state,
    },
};

pub(super) struct Cleanup(pub(super) PathBuf);

static NEXT_TEMP_ROOT_ID: AtomicU64 = AtomicU64::new(0);

pub(super) fn unique_temp_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{label}-{}-{}",
        std::process::id(),
        NEXT_TEMP_ROOT_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn run(
    binary: &Path,
    runtime: &Path,
    state: &Path,
    action: &str,
) -> std::process::Output {
    Command::new(binary)
        .args(["daemon", action])
        .env("XDG_RUNTIME_DIR", runtime)
        .env("XDG_STATE_HOME", state)
        .output()
        .unwrap()
}

pub(super) fn raw_history_record() -> RawHistoryRecordData {
    RawHistoryRecordData {
        schema_version: RAW_HISTORY_SCHEMA_VERSION,
        request_id: "raw-request:persisted-history".into(),
        catalog_version: 1,
        command_id: "build.target".into(),
        parameters: vec![RawExecutionParameterData {
            id: "target".into(),
            value: RawParameterValueData::Target("core-image-minimal".into()),
        }],
        interaction: RawInteractionData::NoninteractiveJob,
        started_unix_ms: 100,
        ended_unix_ms: 150,
        outcome: RawExecutionOutcomeData::Succeeded,
        exit_code: Some(0),
        durable_reference: Some("raw-durable:persisted-history".into()),
    }
}

pub(super) fn raw_history_snapshot(record: RawHistoryRecordData) -> DaemonSnapshot {
    DaemonSnapshot {
        daemon_instance_id: DaemonInstanceId([5; 16]),
        sequence: 4,
        generation: 4,
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
        raw_history: vec![record],
        pty_sessions: Vec::new(),
        pty_screens: Vec::new(),
        clients: Vec::new(),
        recent_logs: Vec::new(),
        build_events: Vec::new(),
        build_progress: None,
        recovery_warnings: Vec::new(),
    }
}
