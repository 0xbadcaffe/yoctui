use super::*;

#[test]
fn batched_activity_notification_is_rate_limited() {
    let notification = ActivityNotification::new().unwrap();
    let ready = |fd| {
        let mut descriptor = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: the descriptor is initialized and borrowed only for
        // this zero-timeout readiness check.
        unsafe { libc::poll(&mut descriptor, 1, 0) > 0 }
    };

    notification.sender.signal_batched();
    assert!(ready(notification.raw_fd()));
    notification.consume();
    notification.sender.signal_batched();
    assert!(!ready(notification.raw_fd()));

    std::thread::sleep(COSMETIC_ACTIVITY_MIN_INTERVAL + Duration::from_millis(5));
    notification.sender.signal_batched();
    assert!(ready(notification.raw_fd()));
}
