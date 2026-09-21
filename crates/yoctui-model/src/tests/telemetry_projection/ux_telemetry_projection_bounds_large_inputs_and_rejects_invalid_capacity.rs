use super::*;

#[test]
fn ux_telemetry_projection_bounds_large_inputs_and_rejects_invalid_capacity() {
    let mut app = App::new(8, 1_024);
    app.host_telemetry.cpu_utilization_percent = Some(u8::MAX);
    app.host_telemetry.memory_total_bytes = Some(u64::MAX);
    app.host_telemetry.memory_available_bytes = Some(u64::MAX - 1);
    app.host_telemetry_history.cpu_percent.extend(0..10_000);
    app.host_telemetry_history.memory_percent.extend(0..10_000);

    let projection = app.host_telemetry_projection();
    let cpu = projection.series(TelemetryMetric::HostCpuUtilization);
    assert_eq!(cpu.history.current, Some(100));
    assert_eq!(cpu.history.points.len(), HOST_TELEMETRY_HISTORY_SAMPLES);
    assert_eq!(cpu.history.points[0], 9_940);
    let memory = projection.series(TelemetryMetric::HostMemoryCapacity);
    assert_eq!(memory.history.current, Some(0));
    assert_eq!(memory.history.points.len(), HOST_TELEMETRY_HISTORY_SAMPLES);

    app.host_telemetry.memory_total_bytes = Some(0);
    app.host_telemetry.memory_available_bytes = Some(1);
    let invalid = app.host_telemetry_projection();
    assert_eq!(
        invalid
            .series(TelemetryMetric::HostMemoryCapacity)
            .history
            .state,
        WidgetState::Partial
    );
}
