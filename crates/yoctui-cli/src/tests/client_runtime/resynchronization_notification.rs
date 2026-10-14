use super::*;

#[test]
fn successful_snapshot_replacement_clears_only_the_pending_resynchronization_banner() {
    let mut app = App::new(16, 4096);
    app.notification = Some(
        "Daemon resynchronization required: client replica must be replaced: HistoryExpired".into(),
    );
    super::super::replica::clear_resynchronization_notification(&mut app);
    assert!(app.notification.is_none());

    app.notification = Some("QEMU launch failed: missing kernel".into());
    super::super::replica::clear_resynchronization_notification(&mut app);
    assert_eq!(
        app.notification.as_deref(),
        Some("QEMU launch failed: missing kernel")
    );
    app.notification = None;
    super::super::replica::clear_resynchronization_notification(&mut app);
    assert!(app.notification.is_none());
}
