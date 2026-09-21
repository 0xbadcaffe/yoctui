include!("security_report/types_and_adapter.rs");

include!("security_report/report_acquisition.rs");

include!("security_report/manifest_and_cyclonedx.rs");

include!("security_report/cve_parsing.rs");

include!("security_report/spdx_and_metadata.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-security-report-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn request(path: &Path) -> SecurityReportRequest {
        SecurityReportRequest::new(1, vec![path.to_path_buf()]).unwrap()
    }

    fn write_cve(path: &Path) {
        fs::write(
            path,
            br#"{
              "version": "1",
              "packages": [{
                "name": "busybox",
                "version": "1.36",
                "products": [{
                  "product": "busybox",
                  "cves": [{
                    "id": "CVE-2024-1234",
                    "status": "Unpatched",
                    "severity": "HIGH",
                    "score": "8.1",
                    "vector": "CVSS:3.1/AV:N",
                    "link": "https://example.invalid/CVE-2024-1234",
                    "summary": "bounded finding",
                    "mapping": {"source": "cve-check"}
                  }]
                }]
              }]
            }"#,
        )
        .unwrap();
    }

    fn write_spdx(path: &Path) {
        fs::write(
            path,
            br#"{
              "spdxVersion": "SPDX-2.3",
              "SPDXID": "SPDXRef-DOCUMENT",
              "name": "core-image-minimal",
              "documentNamespace": "https://example.invalid/spdx/image",
              "dataLicense": "CC0-1.0",
              "creationInfo": {"creators": ["Tool: bitbake"]},
              "packages": [{
                "SPDXID": "SPDXRef-Package-busybox",
                "name": "busybox",
                "versionInfo": "1.36",
                "supplier": "Organization: Yocto",
                "licenseConcluded": "GPL-2.0-only"
              }],
              "files": [{}, {}],
              "relationships": [{}]
            }"#,
        )
        .unwrap();
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

    #[cfg(unix)]
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
}
