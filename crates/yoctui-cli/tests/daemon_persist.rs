#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
};
use yoctui_protocol::daemon_persist::{persist_paths_for, read_persisted_state};

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
