use super::*;

#[test]
fn udev_empty_scan_cancellation_and_deadline_are_explicit() {
    let root = TestRoot::new();
    let token = RootfsCompositionCancellation::default();
    let mut limitations = Vec::new();
    assert!(
        scan(
            root.path(),
            &token,
            Instant::now() + ROOTFS_SCAN_TIMEOUT,
            &mut limitations
        )
        .unwrap()
        .is_empty()
    );
    assert!(limitations.is_empty());
    assert!(matches!(
        scan(root.path(), &token, Instant::now(), &mut limitations),
        Err(RootfsCompositionAdapterError::Timeout(_))
    ));
    token.cancel();
    assert_eq!(
        scan(
            root.path(),
            &token,
            Instant::now() + ROOTFS_SCAN_TIMEOUT,
            &mut limitations
        ),
        Err(RootfsCompositionAdapterError::Cancelled)
    );
}
