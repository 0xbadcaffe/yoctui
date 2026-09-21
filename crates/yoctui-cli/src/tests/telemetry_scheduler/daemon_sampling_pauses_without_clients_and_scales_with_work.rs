use super::*;

#[test]
fn daemon_sampling_pauses_without_clients_and_scales_with_work() {
    assert_eq!(daemon_telemetry_interval(0, false), None);
    assert_eq!(daemon_telemetry_interval(0, true), None);
    assert_eq!(
        daemon_telemetry_interval(1, false),
        Some(DAEMON_ATTACHED_IDLE_INTERVAL)
    );
    assert_eq!(
        daemon_telemetry_interval(1, true),
        Some(DAEMON_ACTIVE_INTERVAL)
    );
}
