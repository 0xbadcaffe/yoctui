use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use yoctui_model::SecurityScope;

#[cfg(unix)]
use std::os::unix::fs::symlink;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "yoctui-security-mapper-{name}-{}-{}",
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

#[cfg(unix)]
fn write_executable(path: &Path, body: &str) {
    crate::test_support::write_executable(path, body);
}

fn preview(directory: &TestDirectory) -> SecurityOperationPreview {
    let executable = directory.path().join("cve-check-map-pkgs");
    let reports = directory.path().join("reports");
    fs::create_dir_all(&reports).unwrap();
    #[cfg(unix)]
    write_executable(&executable, "#!/bin/sh\nexit 0\n");
    let arguments = vec![reports.display().to_string()];
    SecurityOperationPreview {
        id: SecuritySessionId(7),
        scope: SecurityScope::Image {
            target: "core-image-minimal".into(),
            machine: "qemux86-64".into(),
            distro: "poky".into(),
        },
        operation: SecurityOperation::PackageMap {
            executable: executable.clone(),
            arguments: arguments.clone(),
        },
        indexed_arguments: indexed_arguments(&executable, &arguments),
        report_roots: vec![reports],
    }
}

#[cfg(unix)]
mod security_mapper_reconstructs_exact_argv_and_streams_bounded_output;

#[cfg(unix)]
mod security_mapper_spawn_retries_only_transient_text_file_busy;

#[cfg(unix)]
mod security_mapper_rejects_tampering_symlinks_and_stale_identity;

#[cfg(unix)]
mod security_mapper_reports_duplicate_nonzero_and_worker_loss;

#[cfg(unix)]
mod security_mapper_cancels_gracefully_forcibly_and_times_out;
