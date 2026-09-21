use super::*;
use std::{
    ffi::OsStr,
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use std::os::unix::fs::symlink;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "yoctui-test-runner-{name}-{}-{}",
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

fn fixture(name: &str) -> (TestDirectory, TestRunnerAdapter) {
    let directory = TestDirectory::new(name);
    let bin = directory.path().join("bin");
    let build = directory.path().join("build");
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&build).unwrap();
    for tool in ["oe-selftest", "bitbake-selftest"] {
        executable(&bin.join(tool), "#!/bin/sh\nprintf '%s\\n' \"$@\"\n");
    }
    let adapter = TestRunnerAdapter::new(build, vec![bin], PtestCapability::Configured);
    (directory, adapter)
}

fn request(adapter: &TestRunnerAdapter, family: TestFamily) -> TestSelftestRequest {
    let executable = adapter.capability().executable_for(family).unwrap();
    TestSelftestRequest::new(
        executable,
        family,
        (family == TestFamily::OeSelftest).then(|| "tinfoil.Case.test_one".into()),
        4,
        family == TestFamily::BitbakeSelftest,
        family == TestFamily::BitbakeSelftest,
    )
    .unwrap()
}

#[cfg(unix)]
mod test_runner_capability_distinguishes_missing_and_unsafe_executables;

mod test_runner_commands_are_exact_revalidated_and_child_environment_only;

#[cfg(unix)]
mod test_runner_spawn_retries_only_transient_text_file_busy;

#[cfg(unix)]
mod test_runner_rejects_tampering_streams_bounded_output_and_completes;

#[cfg(unix)]
mod test_runner_reports_nonzero_worker_loss_and_cancellation_rejection;

#[cfg(unix)]
mod test_runner_cancels_gracefully_forcibly_and_times_out;
