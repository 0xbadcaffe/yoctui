use super::*;

#[tokio::test]
async fn daemon_startup_slow_inventory_does_not_block_and_can_cancel() {
    let (cleaned_tx, cleaned_rx) = oneshot::channel();
    let mut scan = StartupMetadata::<Workspace>::spawn(|cancel| async move {
        let _ = cancel.await;
        let _ = cleaned_tx.send(());
        Ok(None)
    });
    assert!(scan.pending());
    assert!(scan.try_result().is_none());
    tokio::time::timeout(std::time::Duration::from_secs(1), scan.shutdown())
        .await
        .unwrap();
    cleaned_rx.await.unwrap();
    assert!(scan.try_result().unwrap().unwrap().is_none());
    assert!(!scan.pending());
    assert!(scan.try_result().is_none());
}
