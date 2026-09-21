use super::*;

#[tokio::test]
async fn pkgdata_adapter_times_out_and_cancels_process_groups() {
    let script = "#!/bin/sh\nsleep 5\nprintf 'busybox\\n'\n";
    let (_timeout_directory, timeout_adapter, _log) = fixture("timeout", script);
    let error = timeout_adapter
        .with_timeout(Duration::from_millis(20))
        .inventory(inventory_request())
        .await
        .unwrap_err();
    assert!(matches!(error, PackageDataAdapterError::Timeout(_)));

    let (_cancel_directory, cancel_adapter, _log) = fixture("cancel", script);
    let cancellation = PackageDataCancellation::default();
    let task_cancellation = cancellation.clone();
    let task = tokio::spawn(async move {
        cancel_adapter
            .inventory_with_cancellation(inventory_request(), task_cancellation)
            .await
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(cancellation.cancel());
    assert_eq!(
        task.await.unwrap().unwrap_err(),
        PackageDataAdapterError::Cancelled
    );
}
