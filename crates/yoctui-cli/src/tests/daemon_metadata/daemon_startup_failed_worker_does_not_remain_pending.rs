use super::*;

#[tokio::test]
async fn daemon_startup_failed_worker_does_not_remain_pending() {
    let mut scan =
        StartupMetadata::<Workspace>::spawn(|_| async { panic!("fixture scan failure") });
    assert!((&mut scan.task).await.is_err());
    assert!(scan.try_result().unwrap().is_err());
    assert!(!scan.pending());
}
