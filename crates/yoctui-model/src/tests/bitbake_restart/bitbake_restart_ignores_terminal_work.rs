use super::*;

#[test]
fn bitbake_restart_ignores_terminal_work() {
    let jobs = DaemonJobState::capture(&App::new(128, 1024 * 1024));
    assert!(bitbake_restart_affected_jobs(&jobs).is_empty());
    let preview = BitBakeRestartPreview {
        controller_generation: 1,
        server_identity: "server".into(),
        affected_jobs: Vec::new(),
    };
    assert!(!preview.requires_confirmation());
}
