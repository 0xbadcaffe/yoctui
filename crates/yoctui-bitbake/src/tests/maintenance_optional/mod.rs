use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::symlink;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "yoctui-maintenance-optional-{name}-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }

    fn join(&self, path: &str) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn executable(path: &Path) {
    crate::test_support::write_executable(path, "#!/bin/sh\nexit 0\n");
}

fn process(root: &Path, pid: u32, comm: &str, cmdline: &[u8]) {
    let directory = root.join(pid.to_string());
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("comm"), format!("{comm}\n")).unwrap();
    fs::write(directory.join("cmdline"), cmdline).unwrap();
}

fn complete_fixture(fixture: &TestDirectory) -> MaintenanceOptionalCapabilityInput {
    for directory in [
        "build",
        "tools",
        "work/.git",
        "repo/.repo",
        "proc",
        "config",
    ] {
        fs::create_dir_all(fixture.join(directory)).unwrap();
    }
    for tool in [
        "create-pull-request",
        "send-pull-request",
        "send-error-report",
        "toaster",
        "repo",
    ] {
        executable(&fixture.join(&format!("tools/{tool}")));
    }
    fs::write(fixture.join("work/.git/HEAD"), "ref: refs/heads/main\n").unwrap();
    fs::write(fixture.join("report.json"), "{}\n").unwrap();
    fs::write(fixture.join("repo/.repo/manifest.xml"), "<manifest/>\n").unwrap();
    fs::write(fixture.join("config/toaster.conf"), "setting=true\n").unwrap();
    process(
        &fixture.join("proc"),
        42,
        "sh",
        b"/bin/sh\0/tools/toaster\0start\0",
    );
    MaintenanceOptionalCapabilityInput {
        build_dir: fixture.join("build"),
        executable_search_path: vec![fixture.join("tools")],
        git_worktree_candidates: vec![fixture.join("work")],
        error_report_candidates: vec![fixture.join("report.json")],
        repo_workspace_candidates: vec![fixture.join("repo")],
        toaster_configuration_candidates: vec![fixture.join("config/toaster.conf")],
        process_root: fixture.join("proc"),
    }
}

mod maintenance_optional_detects_complete_capabilities_without_side_effects;

mod maintenance_optional_preserves_missing_and_partial_states;

mod maintenance_optional_reports_fully_unavailable_inputs;

#[cfg(unix)]
mod maintenance_optional_rejects_symlinked_helpers_and_escaped_manifests;

mod maintenance_optional_revalidation_rejects_tampered_evidence;

mod maintenance_optional_bounds_candidates_and_process_records;

mod maintenance_optional_process_evidence_is_observational_only;
