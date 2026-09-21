use super::*;

#[test]
fn telemetry_cadences_are_low_frequency_and_bounded() {
    assert!(CLIENT_VISIBLE_INTERVAL >= Duration::from_secs(1));
    assert!(CLIENT_BACKGROUND_INTERVAL >= Duration::from_secs(5));
    assert!(DAEMON_ACTIVE_INTERVAL >= Duration::from_secs(1));
    assert!(DAEMON_ATTACHED_IDLE_INTERVAL >= Duration::from_secs(5));
}
