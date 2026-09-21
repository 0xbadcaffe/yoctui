use super::*;
use std::{
    ffi::OsStr,
    sync::atomic::{AtomicU64, Ordering},
};
use yoctui_model::{TestComparison, TestJunitDestinationInspection, TestResultInventoryState};

#[cfg(unix)]
use std::os::unix::fs::symlink;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "yoctui-test-results-{name}-{}-{}",
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

fn executable(path: &Path, body: &str) {
    crate::test_support::write_executable(path, body);
}

fn result_json(status: &str) -> String {
    format!(
        r#"{{
                "runtime-qemu": {{
                    "configuration": {{
                        "TEST_TYPE": "runtime",
                        "MACHINE": "qemux86-64",
                        "IMAGE_BASENAME": "core-image-minimal",
                        "OECOREREV": "abc123"
                    }},
                    "result": {{
                        "runtime.Case.test_one": {{
                            "status": "{status}",
                            "duration": 1.25,
                            "log": "bounded diagnostic"
                        }}
                    }}
                }}
            }}"#
    )
}

fn fixture(name: &str) -> (TestDirectory, TestResultAdapter, PathBuf, PathBuf) {
    let directory = TestDirectory::new(name);
    let bin = directory.path().join("bin");
    let results = directory.path().join("results");
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&results).unwrap();
    let resulttool = bin.join("resulttool");
    executable(&resulttool, "#!/bin/sh\nprintf '%s\\n' \"$@\"\nexit 0\n");
    let adapter = TestResultAdapter::new(vec![bin]);
    (directory, adapter, resulttool, results)
}

fn imported(
    adapter: &TestResultAdapter,
    generation: u64,
    paths: Vec<PathBuf>,
) -> TestResultImportResponse {
    adapter
        .import(&TestResultImportRequest::new(generation, paths).unwrap())
        .unwrap()
}

#[cfg(unix)]
mod test_results_capability_is_independent_canonical_and_fail_closed;

mod test_results_imports_official_shape_and_preserves_partial_failures;

mod test_results_import_rejects_unsafe_and_bounds_oversized_files;

mod test_results_construct_exact_comparison_and_non_overwriting_junit_vectors;

#[cfg(unix)]
mod test_results_revalidate_tampering_stream_output_and_nonzero;

#[cfg(unix)]
mod test_results_report_cancellation_timeout_and_worker_loss;
