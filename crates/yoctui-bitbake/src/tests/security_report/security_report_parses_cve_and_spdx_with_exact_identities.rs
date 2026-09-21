use super::*;

#[tokio::test]
async fn security_report_parses_cve_and_spdx_with_exact_identities() {
    let directory = TestDirectory::new();
    write_cve(&directory.path().join("busybox.cve.json"));
    write_spdx(&directory.path().join("image.spdx.json"));

    let response = SecurityReportAdapter::new()
        .scan(request(directory.path()))
        .await
        .unwrap();
    assert!(matches!(
        response.outcome,
        SecurityReportScanOutcome::Complete(_)
    ));
    let reports = response.outcome.reports();
    assert_eq!(reports.len(), 2);
    assert!(reports.iter().all(|report| {
        report.identity().path.starts_with(directory.path())
            && report.identity().fingerprint.len() == 64
    }));
    let cve = reports
        .iter()
        .find_map(|report| match report {
            SecurityReport::Cve(report) => Some(report),
            _ => None,
        })
        .unwrap();
    assert_eq!(cve.findings.len(), 1);
    assert_eq!(cve.findings[0].status, CveStatus::Vulnerable);
    assert_eq!(cve.findings[0].identity.recipe, "busybox");
    let spdx = reports
        .iter()
        .find_map(|report| match report {
            SecurityReport::Spdx(report) => Some(report),
            _ => None,
        })
        .unwrap();
    assert_eq!(spdx.components.len(), 1);
    assert_eq!(spdx.file_count, Some(2));
    assert_eq!(spdx.relationship_count, Some(1));
}
