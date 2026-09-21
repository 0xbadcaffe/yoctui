use super::*;

#[tokio::test]
async fn daemon_startup_inventory_is_delivered_once() {
    let mut scan =
        StartupMetadata::<Workspace>::spawn(|_| async { Ok(Some(Workspace::default())) });
    (&mut scan.task).await.unwrap();
    assert!(scan.try_result().unwrap().unwrap().is_some());
    assert!(!scan.pending());
    assert!(scan.try_result().is_none());
}
