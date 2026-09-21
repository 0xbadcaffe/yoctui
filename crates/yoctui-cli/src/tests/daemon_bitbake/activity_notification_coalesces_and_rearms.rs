use super::*;

#[test]
fn activity_notification_coalesces_and_rearms() {
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

    notification.sender.signal();
    notification.sender.signal();
    assert!(ready(notification.raw_fd()));
    notification.consume();
    assert!(!ready(notification.raw_fd()));

    notification.sender.signal();
    assert!(ready(notification.raw_fd()));
}
