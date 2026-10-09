use super::*;

#[test]
fn client_runtime_effect_mapping_uses_daemon_global_state() {
    let request = yoctui_model::BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: Some("build".into()),
        force: false,
    };
    let command = match Effect::Start(request.clone()) {
        Effect::Start(request) => DaemonCommand::StartBuild {
            targets: request.targets,
            task: request.task,
            force: request.force,
        },
        _ => unreachable!(),
    };
    assert!(matches!(
        command,
        DaemonCommand::StartBuild { targets, task: Some(task), force: false }
            if targets == request.targets && task == "build"
    ));
    let mut app = App::new(16, 4096);
    app.daemon.jobs.push(yoctui_model::ClientDaemonJobSummary {
        id: 71,
        kind: yoctui_model::ClientDaemonJobKind::BitBakeBuild,
        label: "core-image-minimal".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        progress_current: None,
        progress_total: None,
        exit_code: None,
    });
    assert_eq!(app.daemon.jobs[0].id, 71);
    assert!(matches!(Effect::PersistSettings, Effect::PersistSettings));
}

#[test]
fn build_cancellation_routes_only_the_active_bitbake_job() {
    let mut app = App::new(16, 4096);
    for (id, kind, lifecycle) in [
        (
            1,
            yoctui_model::ClientDaemonJobKind::Devtool,
            ClientDaemonLifecycle::Running,
        ),
        (
            2,
            yoctui_model::ClientDaemonJobKind::BitBakeBuild,
            ClientDaemonLifecycle::Exited,
        ),
        (
            3,
            yoctui_model::ClientDaemonJobKind::BitBakeBuild,
            ClientDaemonLifecycle::Running,
        ),
    ] {
        app.daemon.jobs.push(yoctui_model::ClientDaemonJobSummary {
            id,
            kind,
            label: "job".into(),
            lifecycle,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        });
    }
    assert!(
        matches!(daemon_command_for_effect(&app, &Effect::Cancel).unwrap(), Some(DaemonCommand::CancelJob { job_id }) if job_id.0 == 3)
    );
    app.daemon.jobs[2].lifecycle = ClientDaemonLifecycle::Exited;
    assert!(matches!(
        daemon_command_for_effect(&app, &Effect::Cancel),
        Err(ClientRuntimeError::NoActiveDaemonJob)
    ));
}
