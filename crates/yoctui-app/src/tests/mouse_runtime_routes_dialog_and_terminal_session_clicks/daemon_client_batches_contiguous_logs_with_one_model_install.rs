use super::*;

#[test]
fn daemon_client_batches_contiguous_logs_with_one_model_install() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([6; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut client = DaemonClientSnapshot::default();
    let mut app = yoctui_model::App::new(16, 4096);
    client.replace_app(&mut app, daemon_protocol_snapshot(&state));
    let record = |severity, message: &str| yoctui_protocol::daemon::LogRecord {
        source: "bitbake".into(),
        severity,
        message: message.into(),
        unix_ms: 42,
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: None,
        build: Some("core-image-minimal".into()),
    };
    let events = vec![
        yoctui_protocol::daemon::SequencedEvent {
            sequence: 1,
            generation: 1,
            event: yoctui_protocol::daemon::DaemonEvent::Log(record(
                yoctui_protocol::daemon::LogSeverity::Info,
                "ordinary",
            )),
        },
        yoctui_protocol::daemon::SequencedEvent {
            sequence: 2,
            generation: 2,
            event: yoctui_protocol::daemon::DaemonEvent::Log(record(
                yoctui_protocol::daemon::LogSeverity::Error,
                "critical",
            )),
        },
    ];

    client.apply_log_events_to_app(&mut app, &events).unwrap();
    assert_eq!(client.resume_cursor().unwrap().last_sequence, 2);
    assert_eq!(app.logs.entries.len(), 2);
    assert_eq!(app.logs.entries[0].message, "ordinary");
    assert_eq!(app.logs.entries[1].message, "critical");
    assert!(app.logs.entries[1].protected);
    assert_eq!(app.build.errors, 1);
}
