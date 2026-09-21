use super::*;

#[test]
fn ux_progress_separates_build_parse_runqueue_resources_and_sstate() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(600);
    let mut app = App::new(16, 4_096);
    app.build.status = BuildStatus::Running;
    app.build.started = Some(now - Duration::from_secs(120));
    app.build.completed = 30;
    app.build.total = Some(100);
    app.build.parse_current = Some(20);
    app.build.parse_total = Some(20);
    app.host_telemetry.cpu_utilization_percent = Some(72);
    app.host_telemetry.memory_total_bytes = Some(1_000);
    app.host_telemetry.memory_available_bytes = Some(250);
    app.host_telemetry.disk_total_bytes = Some(2_000);
    app.host_telemetry.disk_available_bytes = Some(1_000);

    let hierarchy = app.progress_hierarchy_at(now);
    assert_eq!(
        hierarchy.build.fraction.unwrap().exact_text(),
        "30/100 (30%)"
    );
    assert_eq!(hierarchy.parse.state, WidgetState::TerminalSuccess);
    assert_eq!(hierarchy.runqueue.fraction.unwrap().current, 30);
    assert_eq!(hierarchy.resources.cpu.fraction.unwrap().current, 72);
    assert_eq!(hierarchy.resources.memory.fraction.unwrap().current, 75);
    assert_eq!(
        hierarchy
            .resources
            .build_filesystem
            .fraction
            .unwrap()
            .current,
        50
    );
    assert_eq!(hierarchy.sstate.state, WidgetState::Unavailable);
    assert!(
        hierarchy
            .estimate
            .unwrap()
            .text()
            .starts_with("estimate avg")
    );
}
