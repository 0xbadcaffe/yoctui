use super::*;

fn spec() -> PtySessionSpec {
    PtySessionSpec {
        id: PtySessionId(7),
        name: "kernel menuconfig".into(),
        kind: PtySessionKind::Menuconfig,
        cwd: "/work/poky/build".into(),
        command: PtyCommandIdentity {
            executable: "/work/poky/bitbake/bin/bitbake".into(),
            arguments: vec!["-c".into(), "menuconfig".into(), "virtual/kernel".into()],
        },
        dimensions: PtyDimensions {
            columns: 120,
            rows: 40,
        },
        restartable: true,
        workspace: PtyWorkspaceContext {
            source_dir: "/work/poky".into(),
            build_dir: "/work/poky/build".into(),
            authorized_context_roots: Vec::new(),
            owner_identity: "workspace-1".into(),
        },
    }
}

fn client(value: u8) -> PtyClientId {
    PtyClientId([value; 16])
}

mod pty_session_validates_identity_context_and_dimensions;

mod pty_session_enforces_single_writer_epochs_resize_and_detach;

mod pty_session_bounds_scrollback_and_clears_live_ownership_on_exit;
