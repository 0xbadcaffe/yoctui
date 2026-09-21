use super::*;

#[tokio::test]
async fn security_report_parses_text_and_retains_unsupported_spdx_artifacts() {
    let directory = TestDirectory::new();
    fs::write(
            directory.path().join("findings.cve.txt"),
            b"recipe package version cve status severity\nbusybox busybox 1.36 CVE-2024-1234 Patched HIGH\nbad row\n",
        )
        .unwrap();
    fs::write(
        directory.path().join("future.spdx.json"),
        br#"{"name":"future","components":[]}"#,
    )
    .unwrap();
    fs::write(directory.path().join("image.spdx.tar.zst"), b"archive").unwrap();

    let response = SecurityReportAdapter::new()
        .scan(request(directory.path()))
        .await
        .unwrap();
    assert_eq!(response.outcome.reports().len(), 3);
    assert!(!response.outcome.limitations().is_empty());
    assert!(response.outcome.reports().iter().any(|report| matches!(
        report,
        SecurityReport::Spdx(SpdxDocument {
            kind: SpdxArtifactKind::Archive,
            ..
        })
    )));
}
