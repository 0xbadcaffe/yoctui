use super::*;

#[test]
fn snapshot_timing_legacy_never_invents_attachment_start() {
    use yoctui_protocol::daemon::DaemonBuildEvent;
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        1,
        "fixture-boot".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    snapshot.build_events = vec![
        DaemonBuildEvent::Reset {
            targets: vec!["image".into()],
        },
        DaemonBuildEvent::Started {
            started_unix_ms: None,
        },
        DaemonBuildEvent::TaskStarted {
            recipe: "llvm-native".into(),
            task: "do_compile".into(),
            pid: Some(42),
            worker: None,
            log_path: None,
            stats: None,
            started_unix_ms: None,
        },
    ];
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    DaemonClientSnapshot::default().replace_app(&mut app, snapshot);
    assert_eq!(
        app.build.started, None,
        "legacy snapshots carry no start authority"
    );
    assert_eq!(
        app.tasks[&TaskId("llvm-native:do_compile".into())].started,
        None
    );
}
