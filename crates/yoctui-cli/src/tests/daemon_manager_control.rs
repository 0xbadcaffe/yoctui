use super::*;

#[test]
fn daemon_manager_control_refuses_running_work_and_quotes_configuration() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([1; 16]),
        1,
        "boot".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = yoctui_app::daemon_protocol_snapshot(&state);
    assert!(!busy(&snapshot));
    snapshot.jobs.push(yoctui_protocol::daemon::JobSummary {
        id: yoctui_protocol::daemon::JobId(1),
        kind: yoctui_protocol::daemon::JobKind::BitBakeBuild,
        label: "build".into(),
        lifecycle: LifecycleState::Connecting,
        progress_current: None,
        progress_total: None,
        exit_code: None,
    });
    assert!(busy(&snapshot));
    snapshot.jobs[0].lifecycle = LifecycleState::Failed;
    assert!(!busy(&snapshot));
    let unit = configuration(
        Path::new("/bin/yoctui"),
        Path::new("/build a%$b"),
        Path::new("/source $x"),
    )
    .unwrap();
    assert!(unit.contains("--build-dir \"/build a%%$$b\" daemon foreground"));
    assert!(unit.contains("Environment=\"YOCTUI_SOURCE_DIR=/source $x\""));
    assert!(
        configuration(
            Path::new("/bad\npath"),
            Path::new("/build"),
            Path::new("/source")
        )
        .is_err()
    );
}
