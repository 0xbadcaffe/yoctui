use super::*;

#[test]
fn dashboard_reuses_task_resource_meters() {
    let mut app = App::new(10, 1024);
    app.screen = Screen::Dashboard;
    app.host_telemetry.cpu_utilization_percent = Some(72);
    app.host_telemetry.memory_total_bytes = Some(1000);
    app.host_telemetry.memory_available_bytes = Some(590);
    app.host_telemetry.disk_total_bytes = Some(2000);
    app.host_telemetry.disk_available_bytes = Some(500);
    app.workspace.build_dir = Some("/build".into());

    let dashboard = rendered_text(&app, 160, 50);
    assert!(dashboard.contains("Resource Telemetry"), "{dashboard}");
    assert!(dashboard.contains("CPU Usage") && dashboard.contains("72%"));
    assert!(dashboard.contains("RAM Usage") && dashboard.contains("41%"));
    assert!(
        dashboard.contains("Build FS Usage") && dashboard.contains("75%"),
        "{dashboard}"
    );
}
