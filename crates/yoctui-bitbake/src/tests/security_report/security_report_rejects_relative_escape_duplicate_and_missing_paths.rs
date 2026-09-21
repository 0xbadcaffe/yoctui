use super::*;

#[tokio::test]
async fn security_report_rejects_relative_escape_duplicate_and_missing_paths() {
    let directory = TestDirectory::new();
    let relative = SecurityReportRequest {
        generation: 1,
        paths: vec![PathBuf::from("../reports")],
    };
    assert!(matches!(
        SecurityReportAdapter::new().scan(relative).await,
        Err(SecurityReportAdapterError::InvalidRequest(_))
    ));
    let duplicate = SecurityReportRequest {
        generation: 1,
        paths: vec![directory.0.clone(), directory.0.clone()],
    };
    assert!(matches!(
        SecurityReportAdapter::new().scan(duplicate).await,
        Err(SecurityReportAdapterError::InvalidRequest(_))
    ));
    assert!(matches!(
        SecurityReportAdapter::new()
            .scan(request(&directory.path().join("stale.cve.json")))
            .await,
        Err(SecurityReportAdapterError::MissingPath(_))
    ));
}
