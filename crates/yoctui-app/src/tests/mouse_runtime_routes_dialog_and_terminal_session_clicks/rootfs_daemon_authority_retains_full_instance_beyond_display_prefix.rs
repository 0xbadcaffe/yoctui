use super::*;

#[test]
fn rootfs_daemon_authority_retains_full_instance_beyond_display_prefix() {
    let authority = compatibility_workspace_authority(1).normalize().unwrap();
    let mut snapshot = compatibility_workspace_daemon_snapshot(&authority);
    let mut app = yoctui_model::App::new(16, 4096);
    let mut client = DaemonClientSnapshot::default();
    client.replace_app(&mut app, snapshot.clone());
    let first = app.daemon.instance_id.unwrap();
    let short = app.daemon.instance_identity.clone();
    snapshot.daemon_instance_id.0[15] ^= 1;
    client.replace_app(&mut app, snapshot.clone());
    assert_eq!(app.daemon.instance_identity, short);
    assert_ne!(app.daemon.instance_id, Some(first));
    assert_eq!(
        app.daemon.instance_id,
        Some(yoctui_model::DaemonModelInstanceId(
            snapshot.daemon_instance_id.0
        ))
    );
}
