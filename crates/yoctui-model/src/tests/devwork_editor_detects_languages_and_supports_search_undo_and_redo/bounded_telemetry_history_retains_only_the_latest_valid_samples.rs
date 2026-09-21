use super::*;

#[test]
fn bounded_telemetry_history_retains_only_the_latest_valid_samples() {
    let mut app = App::new(10, 1_000);
    for sample in 0..75 {
        let telemetry = HostTelemetry {
            cpu_utilization_percent: Some(sample),
            memory_total_bytes: Some(1_000),
            memory_available_bytes: Some(750),
            disk_available_bytes: Some(8 * 1024 * 1024 * 1024),
            disk_read_bytes_per_second: Some(u64::from(sample) * 10),
            disk_write_bytes_per_second: Some(u64::from(sample) * 20),
            network_receive_bytes_per_second: Some(u64::from(sample) * 30),
            network_transmit_bytes_per_second: Some(u64::from(sample) * 40),
            ..HostTelemetry::default()
        };
        let _ = update(&mut app, Action::HostTelemetryUpdated(telemetry));
    }
    let telemetry = app.host_telemetry.clone();
    assert_eq!(app.host_telemetry, telemetry);
    let history = &app.host_telemetry_history;
    assert_eq!(history.cpu_percent.len(), 60);
    assert_eq!(history.cpu_percent.front(), Some(&15));
    assert_eq!(history.cpu_percent.back(), Some(&74));
    assert_eq!(history.memory_percent.len(), 60);
    assert!(history.memory_percent.iter().all(|sample| *sample == 25));
    assert_eq!(history.disk_read_bytes_per_second.front(), Some(&150));
    assert_eq!(history.disk_read_bytes_per_second.back(), Some(&740));
    assert_eq!(history.disk_write_bytes_per_second.len(), 60);
    assert_eq!(history.network_receive_bytes_per_second.len(), 60);
    assert_eq!(history.network_transmit_bytes_per_second.len(), 60);

    let _ = update(
        &mut app,
        Action::HostTelemetryUpdated(HostTelemetry {
            cpu_utilization_percent: None,
            memory_total_bytes: Some(100),
            memory_available_bytes: Some(101),
            ..HostTelemetry::default()
        }),
    );
    let history = &app.host_telemetry_history;
    assert_eq!(history.cpu_percent.len(), 60);
    assert_eq!(history.memory_percent.len(), 60);
    assert_eq!(history.disk_read_bytes_per_second.len(), 60);
    assert_eq!(history.network_receive_bytes_per_second.len(), 60);

    let _ = update(
        &mut app,
        Action::HostTelemetryUpdated(HostTelemetry {
            memory_total_bytes: Some(u64::MAX),
            memory_available_bytes: Some(u64::MAX / 2),
            ..HostTelemetry::default()
        }),
    );
    assert_eq!(app.host_telemetry_history.memory_percent.back(), Some(&50));
}
