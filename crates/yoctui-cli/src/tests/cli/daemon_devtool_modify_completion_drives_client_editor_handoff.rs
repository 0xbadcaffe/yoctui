use super::*;

#[test]
fn daemon_devtool_modify_completion_drives_client_editor_handoff() {
    let identity = yoctui_model::RecipeIdentity {
        name: "busybox".into(),
        file: "/poky/meta/recipes-core/busybox/busybox.bb".into(),
    };
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    assert_eq!(
        daemon_devtool_modify_completion(&app, &identity),
        DaemonDevtoolModifyCompletion::Pending
    );
    app.daemon.jobs.push(yoctui_model::ClientDaemonJobSummary {
        id: 7,
        kind: yoctui_model::ClientDaemonJobKind::Devtool,
        label: "Devtool busybox".into(),
        lifecycle: yoctui_model::ClientDaemonLifecycle::Exited,
        progress_current: None,
        progress_total: None,
        exit_code: Some(0),
    });
    assert_eq!(
        daemon_devtool_modify_completion(&app, &identity),
        DaemonDevtoolModifyCompletion::Succeeded
    );
    app.daemon.jobs[0].lifecycle = yoctui_model::ClientDaemonLifecycle::Failed;
    assert_eq!(
        daemon_devtool_modify_completion(&app, &identity),
        DaemonDevtoolModifyCompletion::Failed
    );
}
