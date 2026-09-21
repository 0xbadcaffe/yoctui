use super::*;

#[test]
fn client_runtime_jobs_routes_maintenance_to_daemon() {
    let mut app = App::new(16, 4096);
    app.workspace.build_dir = Some("/build".into());
    let effect =
        Effect::Maintenance(yoctui_model::MaintenanceEffect::InspectCapability { request: 1 });
    assert!(matches!(
        daemon_command_for_effect(&app, &effect).unwrap(),
        Some(DaemonCommand::InspectMaintenanceCapability { request: 1, .. })
    ));
}
