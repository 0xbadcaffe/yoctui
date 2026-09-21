use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use yoctui_model::{PtyCommandIdentity, PtySessionId, PtyWorkspaceContext};

static FIXTURE: AtomicU64 = AtomicU64::new(1);

fn fixture() -> (PathBuf, PtySessionSpec) {
    let id = FIXTURE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("yoctui-pty-attach-{}-{id}", std::process::id()));
    fs::create_dir_all(root.join("build")).unwrap();
    let spec = PtySessionSpec {
            id: PtySessionId(id),
            name: "persistent shell".into(),
            kind: PtySessionKind::BuildShell,
            cwd: root.join("build"),
            command: PtyCommandIdentity {
                executable: "/bin/sh".into(),
                arguments: vec![
                    "-c".into(),
                    "printf 'ready\\n'; while IFS= read -r line; do [ \"$line\" = exit ] && exit 0; printf 'seen:%s\\n' \"$line\"; done".into(),
                ],
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

async fn pump_until(session: &mut DaemonPtySession, needle: &str) {
    loop {
        match session.next_event().await.unwrap() {
            PtyAttachEvent::Output { .. } => {
                if session
                    .snapshot(0)
                    .unwrap()
                    .terminal
                    .plain_text
                    .contains(needle)
                {
                    return;
                }
            }
            event => assert!(!matches!(event, PtyAttachEvent::Lost { .. })),
        }
    }
}

mod pty_attach_prefix_detach_and_client_exit_leave_process_running;

mod pty_attach_listing_distinguishes_running_exited_and_lost;
