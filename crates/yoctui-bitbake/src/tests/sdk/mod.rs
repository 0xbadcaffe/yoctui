use super::*;
use std::{
    fs::File,
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "yoctui-sdk-artifact-{}-{}",
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

fn fixture() -> TestDirectory {
    TestDirectory::new()
}

fn request(root: PathBuf) -> SdkArtifactInventoryRequest {
    SdkArtifactInventoryRequest {
        generation: 1,
        root,
        machine: "qemux86-64".into(),
    }
}

mod sdk_artifact_scan_sorts_classifies_and_associates_records;

mod sdk_artifact_scan_distinguishes_empty_and_unavailable_metadata;

mod sdk_artifact_scan_reports_partial_malformed_and_oversized_records;

#[cfg(unix)]
mod sdk_artifact_scan_rejects_root_symlink_mismatch_and_entry_escape;

mod sdk_artifact_scan_has_distinct_timeout_cancellation_permission_and_worker_loss;

mod sdk_artifact_scan_bounds_directory_entries_deterministically;

mod sdk_artifact_scan_bounds_traversed_directories;
