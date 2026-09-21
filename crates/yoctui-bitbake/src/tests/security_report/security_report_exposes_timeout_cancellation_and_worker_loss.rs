use super::*;

#[tokio::test]
async fn security_report_exposes_timeout_cancellation_and_worker_loss() {
    let directory = TestDirectory::new();
    write_cve(&directory.path().join("valid.cve.json"));
    assert_eq!(
        SecurityReportAdapter::new()
            .with_timeout(Duration::ZERO)
            .scan(request(directory.path()))
            .await,
        Err(SecurityReportAdapterError::Timeout(0))
    );

    let cancellation = SecurityReportCancellation::default();
    assert!(cancellation.cancel());
    assert_eq!(
        SecurityReportAdapter::new()
            .scan_with_cancellation(request(directory.path()), cancellation)
            .await,
        Err(SecurityReportAdapterError::Cancelled)
    );

    assert!(matches!(
        SecurityReportAdapter::new()
            .with_worker_panic()
            .scan(request(directory.path()))
            .await,
        Err(SecurityReportAdapterError::WorkerLost(_))
    ));
}
