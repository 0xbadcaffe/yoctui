use super::*;
use crate::{
    PtyCommandIdentity, PtyDimensions, PtySessionAction, PtySessionSpec, PtyWorkspaceContext,
};

fn session(id: PtySessionId, name: &str, group: i32) -> PtySession {
    let mut session = PtySession::new(
        PtySessionSpec {
            id,
            name: name.into(),
            kind: PtySessionKind::BuildShell,
            cwd: "/work/build".into(),
            command: PtyCommandIdentity {
                executable: "/bin/sh".into(),
                arguments: Vec::new(),
            },
            dimensions: PtyDimensions {
                columns: 80,
                rows: 24,
            },
            restartable: true,
            workspace: PtyWorkspaceContext {
                source_dir: "/work".into(),
                build_dir: "/work/build".into(),
                authorized_context_roots: Vec::new(),
                owner_identity: "workspace".into(),
            },
        },
        group,
    )
    .unwrap();
    session.apply(PtySessionAction::MarkRunning).unwrap();
    session
}

mod pty_multi_allocates_nonreused_ids_and_independent_client_selection;

mod pty_multi_requires_runner_termination_before_close_and_bounds_history;

mod pty_multi_enforces_session_and_client_limits;
