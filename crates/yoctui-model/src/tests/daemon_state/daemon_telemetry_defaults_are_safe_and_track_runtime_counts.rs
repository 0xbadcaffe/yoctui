use super::*;

#[test]
fn daemon_telemetry_defaults_are_safe_and_track_runtime_counts() {
    let telemetry = DaemonTelemetry {
        uptime_seconds: 42,
        connected_clients: 2,
        active_jobs: 3,
        pty_sessions: 1,
        queue_depth: 4,
        memory_bytes: Some(8 * 1024 * 1024),
        recovery: DaemonRecoveryState::Recovered,
    };
    assert_eq!(
        DaemonTelemetry::default().recovery,
        DaemonRecoveryState::CleanStart
    );
    assert_eq!(telemetry.uptime_seconds, 42);
    assert_eq!(telemetry.connected_clients + telemetry.pty_sessions, 3);
    assert_eq!(telemetry.memory_bytes, Some(8 * 1024 * 1024));
}
