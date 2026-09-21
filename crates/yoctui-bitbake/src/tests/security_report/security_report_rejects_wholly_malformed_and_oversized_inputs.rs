use super::*;

#[tokio::test]
async fn security_report_rejects_wholly_malformed_and_oversized_inputs() {
    let directory = TestDirectory::new();
    let malformed = directory.path().join("bad.cve.json");
    fs::write(&malformed, b"{").unwrap();
    assert!(matches!(
        SecurityReportAdapter::new().scan(request(&malformed)).await,
        Err(SecurityReportAdapterError::MalformedReport(path)) if path == malformed
    ));

    let oversized = directory.path().join("huge.cve.json");
    let file = fs::File::create(&oversized).unwrap();
    file.set_len(MAX_SECURITY_FILE_BYTES + 1).unwrap();
    assert!(matches!(
        SecurityReportAdapter::new().scan(request(&oversized)).await,
        Err(SecurityReportAdapterError::OversizedReport(path)) if path == oversized
    ));
}
