#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use yoctui_protocol::{
    daemon::{
        BitBakeState, Capability, ClientHello, ClientId, ClientMessage, DaemonInstanceId,
        DaemonSnapshot, JobId, JobKind, JobSummary, LifecycleState, ProjectProfileSummary,
        ProtocolVersion, PtyKind, PtySessionId, PtySessionSummary, ServerMessage, Subscription,
        TerminalDimensions,
    },
    daemon_ipc::{DaemonConnection, runtime_paths_for},
    daemon_lifecycle::read_boot_id,
    daemon_persist::{
        DaemonPersistedState, PersistedPreferences, persist_paths_for, read_persisted_state,
        write_persisted_state,
    },
};

struct Cleanup(PathBuf);

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

#[test]
fn daemon_persist_writes_safe_metadata_without_live_process_identity() {
    let binary = Path::new(env!("CARGO_BIN_EXE_yoctui"));
    let root =
        std::env::temp_dir().join(format!("yoctui-cli-daemon-persist-{}", std::process::id()));
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
    let root =
        std::env::temp_dir().join(format!("yoctui-cli-daemon-recovery-{}", std::process::id()));
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
        jobs: vec![JobSummary {
            id: JobId(4),
            kind: JobKind::BitBakeBuild,
            label: "core-image-minimal".into(),
            lifecycle: LifecycleState::Running,
            progress_current: Some(2),
            progress_total: Some(8),
            exit_code: None,
        }],
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
        clients: Vec::new(),
        recent_logs: Vec::new(),
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
