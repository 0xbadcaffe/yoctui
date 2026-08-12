#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};

fn run(binary: &Path, runtime: &Path, action: &str) -> std::process::Output {
    Command::new(binary)
        .args(["daemon", action])
        .env("XDG_RUNTIME_DIR", runtime)
        .env("XDG_STATE_HOME", runtime.join("state"))
        .output()
        .unwrap()
}

#[test]
fn daemon_lifecycle_start_status_restart_and_stop_use_one_rust_binary() {
    let binary = Path::new(env!("CARGO_BIN_EXE_yoctui"));
    let runtime = std::env::temp_dir().join(format!(
        "yoctui-cli-daemon-lifecycle-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&runtime);
    fs::DirBuilder::new().mode(0o700).create(&runtime).unwrap();

    let start = run(binary, &runtime, "start");
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    let status = run(binary, &runtime, "status");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let status_text = String::from_utf8_lossy(&status.stdout);
    assert!(status_text.contains("status: running"));
    assert!(status_text.contains("instance:"));
    assert_eq!(
        fs::symlink_metadata(runtime.join("yoctui/daemon.sock"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    let restart = run(binary, &runtime, "restart");
    assert!(
        restart.status.success(),
        "{}",
        String::from_utf8_lossy(&restart.stderr)
    );
    let stop = run(binary, &runtime, "stop");
    assert!(
        stop.status.success(),
        "{}",
        String::from_utf8_lossy(&stop.stderr)
    );
    assert!(!runtime.join("yoctui/daemon.sock").exists());
    assert!(!runtime.join("yoctui/daemon.json").exists());

    assert!(run(binary, &runtime, "start").status.success());
    let status = run(binary, &runtime, "status");
    let pid = String::from_utf8_lossy(&status.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("pid: "))
        .unwrap()
        .parse::<i32>()
        .unwrap();
    // SAFETY: this PID was returned by the isolated daemon status command.
    assert_eq!(unsafe { libc::kill(pid, libc::SIGTERM) }, 0);
    let deadline = Instant::now() + Duration::from_secs(3);
    while runtime.join("yoctui/daemon.sock").exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(!runtime.join("yoctui/daemon.sock").exists());
    assert!(!runtime.join("yoctui/daemon.json").exists());
    fs::remove_dir_all(runtime).unwrap();
}
