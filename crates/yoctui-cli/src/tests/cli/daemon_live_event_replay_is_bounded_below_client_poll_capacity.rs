use super::*;

#[cfg(unix)]
#[test]
fn daemon_live_event_replay_is_bounded_below_client_poll_capacity() {
    assert!(daemon_replay_is_bounded(MAX_DAEMON_CLIENT_EVENTS_PER_TICK));
    assert!(!daemon_replay_is_bounded(
        MAX_DAEMON_CLIENT_EVENTS_PER_TICK + 1
    ));
}
