use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use yoctui_model::{PtyCommandIdentity, PtySessionId, PtySessionKind, PtyWorkspaceContext};

static FIXTURE: AtomicU64 = AtomicU64::new(1);

fn fixture(script: &str) -> (PathBuf, PtySessionSpec) {
    let id = FIXTURE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("yoctui-pty-runner-{}-{id}", std::process::id()));
    fs::create_dir_all(root.join("build")).unwrap();
    let spec = PtySessionSpec {
        id: PtySessionId(id),
        name: "test shell".into(),
        kind: PtySessionKind::BuildShell,
        cwd: root.join("build"),
        command: PtyCommandIdentity {
            executable: "/bin/sh".into(),
            arguments: vec!["-c".into(), script.into()],
        },
        dimensions: PtyDimensions {
            columns: 80,
            rows: 24,
        },
        restartable: true,
        workspace: PtyWorkspaceContext {
            source_dir: root.clone(),
            build_dir: root.join("build"),
            authorized_context_roots: Vec::new(),
            owner_identity: format!("fixture-{id}"),
        },
    };
    (root, spec)
}

async fn collect_until_exit(runner: &mut PtyRunner) -> Vec<u8> {
    let mut output = Vec::new();
    loop {
        match runner.next_event().await.unwrap() {
            PtyRunnerEvent::Started => {}
            PtyRunnerEvent::Output { bytes, .. } => output.extend(bytes),
            PtyRunnerEvent::Exited(_) => return output,
            PtyRunnerEvent::Lost { message } => panic!("PTY lost: {message}"),
        }
    }
}

mod pty_runner_uses_real_pty_raw_io_and_resize;

mod pty_runner_bounds_input_and_forces_ignored_termination;

mod pty_runner_preserves_raw_bytes_and_terminates_gracefully;
