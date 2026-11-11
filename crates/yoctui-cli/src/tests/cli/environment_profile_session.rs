use super::*;
#[test]
fn verified_environment_paths_survive_client_reopen_only_for_the_same_daemon_and_build() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("session.toml");
    let mut session = Session::default();
    let mut app = App::new(16, 4096);
    let profile = yoctui_model::BuildEnvironmentProfile {
        source_dir: "/source".into(),
        build_dir: "/build".into(),
        init_script: "/custom/selected-init.sh".into(),
    };
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.instance_id = Some(yoctui_model::DaemonModelInstanceId([3; 16]));
    app.workspace.build_dir = Some(profile.build_dir.clone());
    app.build_environment = yoctui_model::BuildEnvironmentState::Connected(profile.clone());
    persist_connected_environment(Some(&path), &mut session, &app).unwrap();
    let saved = read_session(Some(&path)).unwrap();
    app.build_environment = yoctui_model::BuildEnvironmentState::Unconfigured;
    restore_session_environment(&saved, &mut app);
    assert_eq!(
        app.build_environment,
        yoctui_model::BuildEnvironmentState::Connected(profile)
    );
    app.build_environment = yoctui_model::BuildEnvironmentState::Unconfigured;
    app.daemon.instance_id = Some(yoctui_model::DaemonModelInstanceId([4; 16]));
    restore_session_environment(&saved, &mut app);
    assert_eq!(
        app.build_environment,
        yoctui_model::BuildEnvironmentState::Unconfigured
    );
    app.daemon.instance_id = Some(yoctui_model::DaemonModelInstanceId([3; 16]));
    app.workspace.build_dir = Some("/different".into());
    restore_session_environment(&saved, &mut app);
    assert_eq!(
        app.build_environment,
        yoctui_model::BuildEnvironmentState::Unconfigured
    );
}
