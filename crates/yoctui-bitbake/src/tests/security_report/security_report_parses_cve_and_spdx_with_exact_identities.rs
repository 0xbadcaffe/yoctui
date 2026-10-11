use super::*;

#[tokio::test]
#[ignore = "explicit read-only Poky SPDX 3 scan, no report generation"]
async fn security_report_actual_poky_spdx3_read_only_smoke() {
    let path = PathBuf::from(
        std::env::var("YOCTUI_LIVE_POKY_SPDX")
            .expect("set YOCTUI_LIVE_POKY_SPDX to a canonical regular file"),
    );
    let response = SecurityReportAdapter::new()
        .scan(request(&path))
        .await
        .unwrap();
    let SecurityReport::Spdx(report) = &response.outcome.reports()[0] else {
        panic!()
    };
    assert_eq!(report.spdx_version.as_deref(), Some("3.0.1"));
    assert!(
        report
            .components
            .iter()
            .any(|package| package.name == "busybox")
    );
    assert!(report.file_count.is_some_and(|count| count > 0));
    println!(
        "Read-only Poky SPDX: version={:?}, components={}, files={:?}",
        report.spdx_version,
        report.components.len(),
        report.file_count
    );
}

#[tokio::test]
async fn security_report_parses_poky_spdx3_packages_not_files_or_agents() {
    let directory = TestDirectory::new();
    fs::write(directory.path().join("image.spdx.json"), serde_json::json!({"@graph": [
        {"type": "CreationInfo", "@id": "_:creation", "specVersion": "3.0.1", "createdBy": ["OpenEmbedded"]},
        {"type": "SpdxDocument", "spdxId": "https://example/document", "name": "core-image-minimal", "creationInfo": "_:creation"},
        {"type": "software_Package", "spdxId": "https://example/package", "name": "busybox", "software_packageVersion": "1.37.0"},
        {"type": "software_File", "spdxId": "https://example/file", "name": "/bin/busybox"},
        {"type": "Organization", "spdxId": "https://example/agent", "name": "OpenEmbedded"},
        {"type": "Relationship"}
    ]}).to_string()).unwrap();
    let response = SecurityReportAdapter::new()
        .scan(request(directory.path()))
        .await
        .unwrap();
    let SecurityReport::Spdx(report) = &response.outcome.reports()[0] else {
        panic!()
    };
    assert_eq!(report.spdx_version.as_deref(), Some("3.0.1"));
    assert_eq!(report.name.as_deref(), Some("core-image-minimal"));
    assert_eq!(report.components.len(), 1);
    assert_eq!(report.components[0].version.as_deref(), Some("1.37.0"));
    assert_eq!(report.file_count, Some(1));
    assert_eq!(report.relationship_count, Some(1));
    assert!(
        report
            .limitations
            .iter()
            .any(|value| value.contains("not resolved"))
    );
}

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
