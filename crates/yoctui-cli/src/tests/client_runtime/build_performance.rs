use super::*;
use crate::client_runtime::effect_routing::daemon_command_for_effect;
#[test]
fn build_profile_is_only_transmitted_on_explicit_selection() {
    let mut app = App::new(100, 4096);
    let effect = Effect::Start(yoctui_model::BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: None,
        force: false,
    });
    assert!(matches!(
        daemon_command_for_effect(&app, &effect).unwrap(),
        Some(DaemonCommand::StartBuild { .. })
    ));
    let settings = yoctui_model::BuildPerformanceSettings::default();
    app.preferences.build_performance = Some(settings);
    assert!(
        daemon_command_for_effect(&app, &effect).is_err(),
        "offline profiles must not silently fall back"
    );
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/build".into());
    app.workspace.bitbake_version = Some("2.20.0".into());
    app.build_environment =
        yoctui_model::BuildEnvironmentState::Connected(yoctui_model::BuildEnvironmentProfile {
            source_dir: "/source".into(),
            build_dir: "/build".into(),
            init_script: "/source/oe-init-build-env".into(),
        });
    let pressure = Some(yoctui_model::PressureSample {
        some_hundredths: 0,
        full_hundredths: None,
    });
    app.daemon.telemetry = Some(yoctui_model::ClientDaemonTelemetry {
        uptime_seconds: 1,
        active_jobs: 0,
        pty_sessions: 0,
        queue_depth: 0,
        pressure: Default::default(),
        memory_bytes: None,
        recovery: yoctui_model::DaemonRecoveryState::CleanStart,
        build_resources: Some(Box::new(yoctui_model::BuildResourceTelemetry {
            sampled_unix_ms: 1,
            cpu_pressure: pressure,
            io_pressure: pressure,
            memory_pressure: pressure,
            ..Default::default()
        })),
    });
    assert!(
        matches!(daemon_command_for_effect(&app,&effect).unwrap(),Some(DaemonCommand::StartProfiledBuild { profile,targets,force:false,task:None }) if profile==settings && targets==vec!["core-image-minimal"])
    );
    app.preferences.build_performance = None;
    assert!(matches!(
        daemon_command_for_effect(&app, &effect).unwrap(),
        Some(DaemonCommand::StartBuild { .. })
    ));
}
