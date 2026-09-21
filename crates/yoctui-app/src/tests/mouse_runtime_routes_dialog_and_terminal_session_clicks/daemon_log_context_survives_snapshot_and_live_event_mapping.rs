use super::*;

#[test]
fn daemon_log_context_survives_snapshot_and_live_event_mapping() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([8; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    let record = yoctui_protocol::daemon::LogRecord {
        source: "bitbake".into(),
        severity: yoctui_protocol::daemon::LogSeverity::Info,
        message: "compiler output".into(),
        unix_ms: 42,
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: Some("/build/tmp/work/busybox/temp/log.do_compile".into()),
        build: Some("core-image-minimal".into()),
    };
    snapshot.recent_logs.push(record.clone());

    let mut client = DaemonClientSnapshot::default();
    let mut app = yoctui_model::App::new(16, 4096);
    client.replace_app(&mut app, snapshot);
    let retained = app.logs.entries.back().unwrap();
    assert_eq!(retained.recipe.as_deref(), Some("busybox"));
    assert_eq!(retained.task.as_deref(), Some("do_compile"));
    assert_eq!(
        retained.path.as_deref(),
        Some(std::path::Path::new(
            "/build/tmp/work/busybox/temp/log.do_compile"
        ))
    );
    assert_eq!(retained.build.as_deref(), Some("core-image-minimal"));

    let event = yoctui_protocol::daemon::SequencedEvent {
        sequence: client.snapshot.as_ref().unwrap().sequence + 1,
        generation: client.snapshot.as_ref().unwrap().generation + 1,
        event: yoctui_protocol::daemon::DaemonEvent::Log(record),
    };
    client.apply_event_to_app(&mut app, &event).unwrap();
    assert_eq!(
        app.logs.entries.back().unwrap().task.as_deref(),
        Some("do_compile")
    );
}
