use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::symlink;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "yoctui-maintenance-sstate-{name}-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(unix)]
fn write_executable(path: &Path, body: &str) {
    crate::test_support::write_executable(path, body);
}

#[cfg(unix)]
fn fixture(
    name: &str,
    cleanup_name: &str,
    body: &str,
) -> (TestDirectory, MaintenanceCapabilitySnapshot) {
    let root = TestDirectory::new(name);
    for directory in ["bin", "build", "cache", "tmp", "stamps", "output"] {
        fs::create_dir(root.0.join(directory)).unwrap();
    }
    write_executable(&root.0.join("bin/oe-check-sstate"), body);
    write_executable(&root.0.join("bin").join(cleanup_name), body);
    let snapshot =
        MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
            build_dir: root.0.join("build"),
            sstate_dir: Some(root.0.join("cache")),
            tmp_dir: Some(root.0.join("tmp")),
            stamps_dirs: vec![root.0.join("stamps")],
            executable_search_path: vec![root.0.join("bin")],
        })
        .unwrap();
    (root, snapshot)
}

fn cleanup_request(root: &TestDirectory) -> SstateCleanupRequest {
    SstateCleanupRequest::new(
        root.0.join("cache"),
        vec![root.0.join("stamps")],
        vec![
            SstateCleanupMode::Duplicates,
            SstateCleanupMode::Orphans,
            SstateCleanupMode::UnreferencedByStamps,
        ],
        4,
    )
    .unwrap()
}

#[cfg(unix)]
mod maintenance_sstate_capability_distinguishes_python_legacy_missing_and_unsafe;

#[cfg(unix)]
mod maintenance_sstate_reconstructs_exact_readiness_and_cleanup_vectors;

#[cfg(unix)]
mod maintenance_sstate_preview_is_bounded_and_cleanup_rejects_changes_or_tampering;

#[cfg(unix)]
mod maintenance_sstate_runner_streams_bounded_output_and_terminal_status;

#[cfg(unix)]
mod maintenance_sstate_cleanup_preview_receives_negative_input_and_revalidates_tool;

mod maintenance_sstate_closed_stdin_does_not_mask_child_outcome;

#[cfg(unix)]
mod maintenance_sstate_spawn_retries_only_transient_text_file_busy;

#[cfg(unix)]
mod maintenance_sstate_fast_cleanup_failure_retains_status_and_stderr;

#[cfg(unix)]
mod maintenance_sstate_runner_reports_nonzero_duplicate_and_cancellation;

#[cfg(unix)]
mod maintenance_sstate_runner_preserves_timeout_forced_cancel_rejection_and_loss;

#[cfg(unix)]
mod hardening_stress_process_tree_cancellation_reaps_descendant;
