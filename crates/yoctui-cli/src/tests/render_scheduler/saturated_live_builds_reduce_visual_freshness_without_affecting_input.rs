use super::*;

#[test]
fn saturated_live_builds_reduce_visual_freshness_without_affecting_input() {
    let mut app = App::new(16, 16 * 1024);
    app.build.status = BuildStatus::Running;
    app.host_telemetry.cpu_utilization_percent = Some(99);
    assert_eq!(ordinary_frame_interval(&app), SATURATED_FRAME_INTERVAL);
    assert_eq!(animation_interval(&app), SATURATED_FRAME_INTERVAL);

    app.build.status = BuildStatus::Completed;
    assert_eq!(ordinary_frame_interval(&app), ORDINARY_FRAME_INTERVAL);
    app.build.status = BuildStatus::Running;
    app.host_telemetry.cpu_utilization_percent = Some(89);
    assert_eq!(ordinary_frame_interval(&app), ORDINARY_FRAME_INTERVAL);
}
