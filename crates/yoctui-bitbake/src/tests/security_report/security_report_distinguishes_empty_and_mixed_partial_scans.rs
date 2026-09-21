use super::*;

#[tokio::test]
async fn security_report_distinguishes_empty_and_mixed_partial_scans() {
    let empty = TestDirectory::new();
    let response = SecurityReportAdapter::new()
        .scan(request(empty.path()))
        .await
        .unwrap();
    assert_eq!(response.outcome, SecurityReportScanOutcome::Empty);

    let mixed = TestDirectory::new();
    write_cve(&mixed.path().join("valid.cve.json"));
    fs::write(mixed.path().join("broken.cve.json"), b"{").unwrap();
    let response = SecurityReportAdapter::new()
        .scan(request(mixed.path()))
        .await
        .unwrap();
    assert!(matches!(
        response.outcome,
        SecurityReportScanOutcome::Partial {
            ref reports,
            ref limitations
        } if reports.len() == 1 && !limitations.is_empty()
    ));
}
