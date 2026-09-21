use super::*;

#[test]
fn pty_attach_listing_distinguishes_running_exited_and_lost() {
    assert_eq!(
        listing_lifecycle(PtySessionLifecycle::Running),
        PtyAttachLifecycle::Running
    );
    assert_eq!(
        listing_lifecycle(PtySessionLifecycle::Exited),
        PtyAttachLifecycle::Exited
    );
    assert_eq!(
        listing_lifecycle(PtySessionLifecycle::Lost),
        PtyAttachLifecycle::Lost
    );
}
