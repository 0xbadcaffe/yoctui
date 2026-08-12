#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use yoctui_protocol::{
    daemon::{
        Capability, ClientHello, ClientId, ClientMessage, ProjectProfileSummary, ProtocolVersion,
        ServerMessage, Subscription,
    },
    daemon_ipc::{DaemonConnection, runtime_paths_for},
};

struct DaemonGuard {
    binary: PathBuf,
    runtime: PathBuf,
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = Command::new(&self.binary)
            .args(["daemon", "stop"])
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .output();
        let _ = fs::remove_dir_all(&self.runtime);
    }
}

fn attach_snapshot(
    runtime: &Path,
    resume: Option<yoctui_protocol::daemon::ResumeCursor>,
) -> (yoctui_protocol::daemon::DaemonHello, ServerMessage) {
    let paths = runtime_paths_for(runtime.to_path_buf(), unsafe { libc::geteuid() }).unwrap();
    let mut connection = DaemonConnection::connect(&paths, Duration::from_secs(2)).unwrap();
    connection
        .set_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    connection
        .send(&ClientMessage::Hello(ClientHello {
            minimum_version: ProtocolVersion::CURRENT,
            maximum_version: ProtocolVersion::CURRENT,
            client_id: ClientId([9; 16]),
            client_name: "daemon-state-test".into(),
            capabilities: vec![Capability::StateSnapshots, Capability::BackgroundJobs],
        }))
        .unwrap();
    let hello = match connection.receive::<ServerMessage>().unwrap() {
        ServerMessage::Hello(hello) => hello,
        response => panic!("expected daemon hello, got {response:?}"),
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
            resume,
        })
        .unwrap();
    let attached = connection.receive().unwrap();
    connection.send(&ClientMessage::Detach).unwrap();
    assert!(matches!(
        connection.receive::<ServerMessage>().unwrap(),
        ServerMessage::Detaching
    ));
    (hello, attached)
}

#[test]
fn daemon_state_runtime_owns_snapshot_across_client_detach_and_reattach() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_yoctui"));
    let runtime = std::env::temp_dir().join(format!(
        "yoctui-cli-daemon-state-runtime-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&runtime);
    fs::DirBuilder::new().mode(0o700).create(&runtime).unwrap();
    let guard = DaemonGuard {
        binary: binary.clone(),
        runtime: runtime.clone(),
    };
    let start = Command::new(&binary)
        .args(["daemon", "start"])
        .env("XDG_RUNTIME_DIR", &runtime)
        .output()
        .unwrap();
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );

    let (first_hello, first_attached) = attach_snapshot(&runtime, None);
    let first_snapshot = match first_attached {
        ServerMessage::Attached {
            snapshot,
            replayed_through,
        } => {
            assert_eq!(snapshot.sequence, replayed_through);
            snapshot
        }
        response => panic!("expected attached snapshot, got {response:?}"),
    };
    assert_eq!(
        first_snapshot.daemon_instance_id,
        first_hello.daemon_instance_id
    );
    assert_eq!(first_snapshot.sequence, 1);
    assert_eq!(first_snapshot.generation, 1);
    assert!(matches!(
        first_snapshot.project_profile,
        ProjectProfileSummary::NotLoaded
    ));
    assert!(first_snapshot.jobs.is_empty());

    let (second_hello, second_attached) = attach_snapshot(
        &runtime,
        Some(yoctui_protocol::daemon::ResumeCursor {
            daemon_instance_id: first_snapshot.daemon_instance_id,
            last_sequence: first_snapshot.sequence,
        }),
    );
    let second_snapshot = match second_attached {
        ServerMessage::Attached { snapshot, .. } => snapshot,
        response => panic!("expected attached snapshot, got {response:?}"),
    };
    assert_eq!(
        second_hello.daemon_instance_id,
        first_hello.daemon_instance_id
    );
    assert_eq!(second_snapshot, first_snapshot);

    let status = Command::new(&binary)
        .args(["daemon", "status"])
        .env("XDG_RUNTIME_DIR", &runtime)
        .output()
        .unwrap();
    assert!(status.status.success());
    drop(guard);
}
