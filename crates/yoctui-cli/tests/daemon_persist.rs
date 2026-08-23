#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use yoctui_protocol::{
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

struct Cleanup(PathBuf);

static NEXT_TEMP_ROOT_ID: AtomicU64 = AtomicU64::new(0);

fn unique_temp_root(label: &str) -> PathBuf {
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

fn run(binary: &Path, runtime: &Path, state: &Path, action: &str) -> std::process::Output {
    Command::new(binary)
        .args(["daemon", action])
        .env("XDG_RUNTIME_DIR", runtime)
        .env("XDG_STATE_HOME", state)
        .output()
        .unwrap()
}

fn raw_history_record() -> RawHistoryRecordData {
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

fn raw_history_snapshot(record: RawHistoryRecordData) -> DaemonSnapshot {
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
        recovery_warnings: Vec::new(),
    }
}

#[test]
fn raw_history_persistence_round_trips_safe_terminal_metadata_and_rejects_future_schema() {
    let root = unique_temp_root("yoctui-cli-raw-history");
    let cleanup = Cleanup(root.clone());
    let paths = persist_paths_for(&root).unwrap();
    let snapshot = raw_history_snapshot(raw_history_record());
    let persisted = DaemonPersistedState::capture(
        &snapshot,
        200,
        "boot-one".into(),
        Vec::new(),
        PersistedPreferences::default(),
    );
    write_persisted_state(&paths, &persisted).unwrap();
    let loaded = read_persisted_state(&paths).unwrap().unwrap();
    assert_eq!(loaded.raw_history, snapshot.raw_history);
    let serialized = fs::read_to_string(&paths.state).unwrap();
    for prohibited in [
        "raw-job:",
        "raw-session:",
        "process_group",
        "writer_epoch",
        "capability_generation",
        "preview_digest",
        "build_directory",
        "stdout",
        "stderr",
        "pty_screens",
    ] {
        assert!(!serialized.contains(prohibited), "retained {prohibited}");
    }

    let mut current = raw_history_snapshot(raw_history_record());
    current.raw_history.clear();
    current.daemon_instance_id = DaemonInstanceId([6; 16]);
    let (recovered, _) = recover_persisted_snapshot(current, &loaded, "boot-one");
    assert_eq!(recovered.raw_history, loaded.raw_history);
    assert!(recovered.raw_executions.is_empty());
    assert!(recovered.pty_sessions.is_empty());

    let mut future = loaded;
    future.raw_history[0].schema_version += 1;
    assert!(write_persisted_state(&paths, &future).is_err());
    drop(cleanup);
}

#[test]
fn daemon_persist_fixtures_use_unique_temp_roots() {
    let first = unique_temp_root("yoctui-cli-daemon-recovery");
    let second = unique_temp_root("yoctui-cli-daemon-recovery");

    assert_ne!(first, second);
}

#[test]
fn daemon_persist_writes_safe_metadata_without_live_process_identity() {
    let binary = Path::new(env!("CARGO_BIN_EXE_yoctui"));
    let root = unique_temp_root("yoctui-cli-daemon-persist");
    let _ = fs::remove_dir_all(&root);
    let runtime = root.join("runtime");
    let state_root = root.join("state");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&runtime)
        .unwrap();
    let cleanup = Cleanup(root);

    let start = run(binary, &runtime, &state_root, "start");
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    let paths = persist_paths_for(&state_root).unwrap();
    let running = read_persisted_state(&paths).unwrap().unwrap();
    assert_eq!(running.schema_version, 1);
    assert_eq!(running.last_sequence, 1);
    assert!(running.job_history.is_empty());
    assert!(running.terminal_sessions.is_empty());
    assert!(running.recent_logs.is_empty());
    assert_eq!(
        fs::symlink_metadata(&paths.state)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let serialized = fs::read_to_string(&paths.state).unwrap();
    assert!(!serialized.contains("\"pid\""));
    assert!(!serialized.contains("process_group"));

    let stop = run(binary, &runtime, &state_root, "stop");
    assert!(
        stop.status.success(),
        "{}",
        String::from_utf8_lossy(&stop.stderr)
    );
    let stopped = read_persisted_state(&paths).unwrap().unwrap();
    assert_eq!(
        stopped.previous_daemon_instance_id,
        running.previous_daemon_instance_id
    );
    assert!(stopped.saved_unix_ms >= running.saved_unix_ms);
    drop(cleanup);
}

#[test]
fn daemon_recovery_restores_history_but_marks_live_work_lost() {
    let binary = Path::new(env!("CARGO_BIN_EXE_yoctui"));
    let root = unique_temp_root("yoctui-cli-daemon-recovery");
    let _ = fs::remove_dir_all(&root);
    let runtime = root.join("runtime");
    let state_root = root.join("state");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&runtime)
        .unwrap();
    let cleanup = Cleanup(root);
    let persistence = persist_paths_for(&state_root).unwrap();
    let previous_instance = DaemonInstanceId([2; 16]);
    let snapshot = DaemonSnapshot {
        daemon_instance_id: previous_instance,
        sequence: 12,
        generation: 9,
        workspace: None,
        project_profile: ProjectProfileSummary::Absent,
        bitbake: BitBakeState {
            lifecycle: LifecycleState::Running,
            version: Some("2.8.1".into()),
            capabilities: Vec::new(),
            diagnostic: None,
        },
        compatibility: None,
        jobs: vec![JobSummary {
            id: JobId(4),
            kind: JobKind::BitBakeBuild,
            label: "core-image-minimal".into(),
            lifecycle: LifecycleState::Running,
            progress_current: Some(2),
            progress_total: Some(8),
            exit_code: None,
        }],
        raw_executions: Vec::new(),
        raw_history: Vec::new(),
        pty_sessions: vec![PtySessionSummary {
            id: PtySessionId(6),
            name: "devshell".into(),
            kind: PtyKind::Devshell,
            cwd: "/work/build".into(),
            lifecycle: LifecycleState::Running,
            dimensions: TerminalDimensions {
                columns: 100,
                rows: 30,
            },
            writer: Some(ClientId([3; 16])),
            writer_epoch: 2,
            viewers: 1,
            exit_code: None,
            restartable: true,
        }],
        pty_screens: Vec::new(),
        clients: Vec::new(),
        recent_logs: Vec::new(),
        build_events: Vec::new(),
        recovery_warnings: Vec::new(),
    };
    let persisted = DaemonPersistedState::capture(
        &snapshot,
        1,
        read_boot_id().unwrap(),
        Vec::new(),
        PersistedPreferences::default(),
    );
    write_persisted_state(&persistence, &persisted).unwrap();

    let start = run(binary, &runtime, &state_root, "start");
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    // SAFETY: geteuid has no preconditions and does not modify memory.
    let ipc_paths = runtime_paths_for(runtime.clone(), unsafe { libc::geteuid() }).unwrap();
    let mut connection = DaemonConnection::connect(&ipc_paths, Duration::from_secs(2)).unwrap();
    connection
        .set_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    connection
        .send(&ClientMessage::Hello(ClientHello {
            minimum_version: ProtocolVersion::CURRENT,
            maximum_version: ProtocolVersion::CURRENT,
            client_id: ClientId([7; 16]),
            client_name: "recovery-test".into(),
            capabilities: vec![Capability::StateSnapshots],
        }))
        .unwrap();
    let hello = match connection.receive::<ServerMessage>().unwrap() {
        ServerMessage::Hello(hello) => hello,
        message => panic!("expected hello, got {message:?}"),
    };
    connection
        .send(&ClientMessage::Attach {
            workspace: None,
            subscription: Subscription {
                state: true,
                jobs: true,
                logs: true,
                pty_sessions: Vec::new(),
            },
            resume: None,
        })
        .unwrap();
    let recovered = match connection.receive::<ServerMessage>().unwrap() {
        ServerMessage::Attached { snapshot, .. } => snapshot,
        message => panic!("expected recovered snapshot, got {message:?}"),
    };
    assert_ne!(hello.daemon_instance_id, previous_instance);
    assert_eq!(recovered.jobs[0].lifecycle, LifecycleState::Lost);
    assert_eq!(recovered.pty_sessions[0].lifecycle, LifecycleState::Lost);
    assert_eq!(recovered.pty_sessions[0].writer, None);
    assert_eq!(recovered.pty_sessions[0].viewers, 0);
    assert_eq!(recovered.bitbake.lifecycle, LifecycleState::Disconnected);
    assert!(recovered.bitbake.diagnostic.is_some());
    assert!(
        recovered
            .recovery_warnings
            .iter()
            .any(|warning| warning.contains("daemon instance restarted"))
    );
    connection.send(&ClientMessage::Detach).unwrap();
    assert!(matches!(
        connection.receive::<ServerMessage>().unwrap(),
        ServerMessage::Detaching
    ));
    drop(connection);
    assert!(run(binary, &runtime, &state_root, "stop").status.success());
    drop(cleanup);
}

#[test]
fn reboot_recovery_exposes_only_typed_explicit_relaunch_intent() {
    let previous = DaemonSnapshot {
        daemon_instance_id: DaemonInstanceId([1; 16]),
        sequence: 4,
        generation: 4,
        workspace: None,
        project_profile: ProjectProfileSummary::Absent,
        bitbake: BitBakeState {
            lifecycle: LifecycleState::Running,
            version: None,
            capabilities: Vec::new(),
            diagnostic: None,
        },
        compatibility: None,
        jobs: Vec::new(),
        raw_executions: Vec::new(),
        raw_history: Vec::new(),
        pty_sessions: vec![PtySessionSummary {
            id: PtySessionId(9),
            name: "sdk-shell".into(),
            kind: PtyKind::SdkShell,
            cwd: "/opt/sdk".into(),
            lifecycle: LifecycleState::Running,
            dimensions: TerminalDimensions {
                columns: 90,
                rows: 28,
            },
            writer: Some(ClientId([2; 16])),
            writer_epoch: 3,
            viewers: 1,
            exit_code: None,
            restartable: true,
        }],
        pty_screens: Vec::new(),
        clients: Vec::new(),
        recent_logs: Vec::new(),
        build_events: Vec::new(),
        recovery_warnings: Vec::new(),
    };
    let persisted = DaemonPersistedState::capture(
        &previous,
        1,
        "previous-host-boot".into(),
        Vec::new(),
        PersistedPreferences::default(),
    );
    let mut current = previous.clone();
    current.daemon_instance_id = DaemonInstanceId([8; 16]);
    current.sequence = 0;
    current.generation = 0;
    current.jobs.clear();
    current.pty_sessions.clear();
    let (recovered, report) = recover_persisted_snapshot(current, &persisted, "new-host-boot");

    assert_eq!(report.boundary, DaemonRecoveryBoundary::HostReboot);
    assert!(report.previous_boot_changed);
    assert_eq!(recovered.pty_sessions[0].lifecycle, LifecycleState::Lost);
    assert_eq!(recovered.pty_sessions[0].writer, None);
    assert_eq!(report.terminal_relaunch_intents.len(), 1);
    let relaunch = &report.terminal_relaunch_intents[0];
    assert_eq!(relaunch.name, "sdk-shell");
    assert_eq!(relaunch.kind, PtyKind::SdkShell);
    assert_eq!(relaunch.cwd, "/opt/sdk");
    assert_eq!(relaunch.dimensions.columns, 90);
}

#[test]
fn daemon_reboot_acceptance_reloads_metadata_and_marks_live_children_lost() {
    daemon_recovery_restores_history_but_marks_live_work_lost();
    reboot_recovery_exposes_only_typed_explicit_relaunch_intent();
}
