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

mod security_report_parses_cve_and_spdx_with_exact_identities;

mod security_report_parses_cyclonedx_and_legacy_image_manifest;

mod security_report_distinguishes_empty_and_mixed_partial_scans;

mod security_report_parses_text_and_retains_unsupported_spdx_artifacts;

mod security_report_rejects_wholly_malformed_and_oversized_inputs;

#[cfg(unix)]
mod security_report_rejects_explicit_symlinks_and_does_not_follow_nested_ones;

mod security_report_rejects_relative_escape_duplicate_and_missing_paths;

mod security_report_exposes_timeout_cancellation_and_worker_loss;
