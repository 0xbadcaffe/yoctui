use super::*;

#[tokio::test]
async fn signature_adapter_bounds_output_and_supports_cancellation() {
    let directory = TestDirectory::new("bounds");
    signature_path(directory.path(), "aaa");
    let oversized = directory.path().join("oversized");
    write_executable(
        &oversized,
        "#!/bin/sh\nhead -c 9000000 /dev/zero | tr '\\0' x\n",
    );
    let error = test_adapter(directory.path(), oversized, directory.path().join("unused"))
        .dump(target())
        .await
        .unwrap_err();
    assert_eq!(
        error,
        SignatureAdapterError::OutputLimit(MAX_SIGNATURE_OUTPUT_BYTES)
    );

    let sleeping = directory.path().join("sleeping");
    write_executable(&sleeping, "#!/bin/sh\nsleep 30\n");
    let adapter = test_adapter(directory.path(), sleeping, directory.path().join("unused"))
        .with_timeout(Duration::from_secs(60));
    let cancellation = SignatureCancellation::default();
    let cancel_handle = cancellation.clone();
    let operation =
        tokio::spawn(async move { adapter.dump_with_cancellation(target(), cancellation).await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(cancel_handle.cancel());
    assert!(!cancel_handle.cancel());
    assert_eq!(
        operation.await.unwrap().unwrap_err(),
        SignatureAdapterError::Cancelled
    );
}
