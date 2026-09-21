use super::*;

#[tokio::test]
async fn security_report_parses_cyclonedx_and_legacy_image_manifest() {
    let directory = TestDirectory::new();
    fs::write(
        directory.path().join("image.cyclonedx.json"),
        br#"{
              "bomFormat": "CycloneDX",
              "specVersion": "1.6",
              "serialNumber": "urn:uuid:1234",
              "version": 1,
              "components": [{
                "type": "library",
                "bom-ref": "pkg:generic/busybox@1.36",
                "name": "busybox",
                "version": "1.36",
                "supplier": {"name": "Yocto"},
                "licenses": [{"license": {"id": "GPL-2.0-only"}}]
              }],
              "dependencies": [{"ref": "pkg:generic/busybox@1.36"}]
            }"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("core-image-minimal.manifest"),
        b"busybox core2-64 1.36.1\nbase-files core2-64 3.0\n",
    )
    .unwrap();

    let response = SecurityReportAdapter::new()
        .scan(request(directory.path()))
        .await
        .unwrap();
    let reports = response.outcome.reports();
    let cyclonedx = reports
        .iter()
        .find_map(|report| match report {
            SecurityReport::CycloneDx(document) => Some(document),
            _ => None,
        })
        .unwrap();
    assert_eq!(cyclonedx.spec_version.as_deref(), Some("1.6"));
    assert_eq!(
        cyclonedx.components[0].license.as_deref(),
        Some("GPL-2.0-only")
    );
    assert_eq!(cyclonedx.dependency_count, Some(1));
    let manifest = reports
        .iter()
        .find_map(|report| match report {
            SecurityReport::PackageManifest(document) => Some(document),
            _ => None,
        })
        .unwrap();
    assert_eq!(manifest.components.len(), 2);
    assert_eq!(manifest.components[0].version.as_deref(), Some("3.0"));
    assert_eq!(manifest.components[1].version.as_deref(), Some("1.36.1"));
}
