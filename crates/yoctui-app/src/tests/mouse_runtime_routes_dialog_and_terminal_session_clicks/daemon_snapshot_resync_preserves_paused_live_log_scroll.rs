use super::*;

#[test]
fn daemon_snapshot_resync_preserves_paused_live_log_scroll() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    for index in 0..12 {
        snapshot
            .recent_logs
            .push(yoctui_protocol::daemon::LogRecord {
                source: "bitbake".into(),
                severity: yoctui_protocol::daemon::LogSeverity::Info,
                message: format!("line {index}"),
                unix_ms: index,
                recipe: Some("busybox".into()),
                task: Some("do_compile".into()),
                path: None,
                build: Some("core-image-minimal".into()),
            });
    }
    let mut client = DaemonClientSnapshot::default();
    let mut app = yoctui_model::App::new(32, 8192);
    client.replace_app(&mut app, snapshot.clone());
    let _ = yoctui_model::update(&mut app, Action::ScrollLogs { delta: 5 });
    app.logs.query = "line".into();
    let selection = app.logs.selection;

    snapshot.sequence += 1;
    snapshot.generation += 1;
    snapshot
        .recent_logs
        .push(yoctui_protocol::daemon::LogRecord {
            source: "bitbake".into(),
            severity: yoctui_protocol::daemon::LogSeverity::Info,
            message: "line 12".into(),
            unix_ms: 12,
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: None,
            build: Some("core-image-minimal".into()),
        });
    client.replace_app(&mut app, snapshot);

    assert!(!app.logs.follow);
    assert_eq!(app.logs.query, "line");
    assert_eq!(app.logs.selection, selection);
    assert!(app.logs.scroll_offset > 0);
}
