use super::*;

#[test]
fn ux_telemetry_projection_preserves_units_roles_zero_and_missing_samples() {
    let mut app = App::new(8, 1_024);
    let _ = update(
        &mut app,
        Action::HostTelemetryUpdated(HostTelemetry {
            cpu_utilization_percent: Some(0),
            memory_total_bytes: Some(8_000),
            memory_available_bytes: Some(2_000),
            disk_read_bytes_per_second: Some(0),
            network_receive_bytes_per_second: Some(4_096),
            ..HostTelemetry::default()
        }),
    );
    let _ = update(
        &mut app,
        Action::HostTelemetryUpdated(HostTelemetry {
            memory_total_bytes: Some(8_000),
            memory_available_bytes: Some(4_000),
            ..HostTelemetry::default()
        }),
    );

    let projection = app.host_telemetry_projection();
    let cpu = projection.series(TelemetryMetric::HostCpuUtilization);
    assert_eq!(cpu.unit, TelemetryUnit::IntegerPercent);
    assert_eq!(cpu.history.role, WidgetRole::Cpu);
    assert_eq!(cpu.history.current, None);
    assert_eq!(cpu.history.points, [0]);
    assert_eq!(cpu.history.state, WidgetState::Partial);

    let memory = projection.series(TelemetryMetric::HostMemoryCapacity);
    assert_eq!(memory.history.current, Some(50));
    assert_eq!(memory.history.points, [75, 50]);

    let read = projection.series(TelemetryMetric::DiskReadRate);
    assert_eq!(read.unit, TelemetryUnit::BytesPerSecond);
    assert_eq!(read.history.role, WidgetRole::DiskRead);
    assert_eq!(read.history.current, None);
    assert_eq!(read.history.points, [0]);
    assert_eq!(read.history.state, WidgetState::Partial);

    let transmit = projection.series(TelemetryMetric::NetworkTransmitRate);
    assert_eq!(transmit.history.state, WidgetState::Unavailable);
    assert!(transmit.history.points.is_empty());
}
