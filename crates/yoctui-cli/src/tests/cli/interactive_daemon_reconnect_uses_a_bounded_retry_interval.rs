use super::*;

#[test]
fn interactive_daemon_reconnect_uses_a_bounded_retry_interval() {
    assert_eq!(
        client_runtime::DAEMON_RECONNECT_INTERVAL,
        Duration::from_secs(1)
    );
    assert!(client_runtime::DAEMON_RECONNECT_INTERVAL <= Duration::from_secs(5));
}
