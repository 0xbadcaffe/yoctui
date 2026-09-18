use super::*;

#[test]
fn idle_daemon_waits_on_socket_readiness_without_delaying_active_work() {
    assert_eq!(daemon_service_wait(false), DAEMON_IDLE_WAIT);
    assert_eq!(daemon_service_wait(true), DAEMON_ACTIVE_WAIT);
    assert!(DAEMON_IDLE_WAIT >= Duration::from_millis(50));
    assert_eq!(DAEMON_ACTIVE_WAIT, Duration::from_millis(35));
}
