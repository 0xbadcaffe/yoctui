use super::*;

#[tokio::test]
async fn security_report_rejects_explicit_symlinks_and_does_not_follow_nested_ones() {
    use std::os::unix::fs::symlink;

    let directory = TestDirectory::new();
    let outside = TestDirectory::new();
    let report = outside.path().join("outside.cve.json");
    write_cve(&report);
    let link = directory.path().join("linked.cve.json");
    symlink(&report, &link).unwrap();
    assert!(matches!(
        SecurityReportAdapter::new().scan(request(&link)).await,
        Err(SecurityReportAdapterError::SymlinkPath(path)) if path == link
    ));

    write_spdx(&directory.path().join("valid.spdx.json"));
    let response = SecurityReportAdapter::new()
        .scan(request(directory.path()))
        .await
        .unwrap();
    assert_eq!(response.outcome.reports().len(), 1);
    assert!(
        response
            .outcome
            .limitations()
            .iter()
            .any(|value| value.contains("symlink"))
    );
}
