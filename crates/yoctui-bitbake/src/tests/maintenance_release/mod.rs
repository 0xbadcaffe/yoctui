use super::*;
use crate::maintenance_sstate::{MaintenanceSstateJobRunner, MaintenanceSstateRunnerEvent};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::symlink;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "yoctui-maintenance-release-{name}-{}-{}",
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

fn executable(path: &Path, body: &str) {
    crate::test_support::write_executable(path, body);
}

fn git_repository(path: &Path) {
    fs::create_dir_all(path.join(".git")).unwrap();
    fs::write(path.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
}

fn prepare_fixture(fixture: &TestDirectory, body: &str) {
    for directory in [
        "build",
        "tools",
        "history",
        "input-cache",
        "output-cache",
        "archive-data",
    ] {
        fs::create_dir_all(fixture.join(directory)).unwrap();
    }
    git_repository(&fixture.join("history"));
    fs::write(fixture.join("locked.inc"), "SIGGEN_LOCKEDSIGS = \"x\"\n").unwrap();
    fs::write(fixture.join("filter.txt"), "recipe:task\n").unwrap();
    fs::write(fixture.join("note.txt"), "release note\n").unwrap();
    for name in ["gen-lockedsig-cache", "buildhistory-diff", "oe-git-archive"] {
        executable(&fixture.join(&format!("tools/{name}")), body);
    }
}

fn inspect(fixture: &TestDirectory) -> MaintenanceCapabilitySnapshot {
    MaintenanceReleaseCapabilityInspector::inspect(MaintenanceReleaseCapabilityInput {
        build_dir: fixture.join("build"),
        buildhistory_dir: Some(fixture.join("history")),
        native_lsb: Some("ubuntu-24.04".into()),
        executable_search_path: vec![fixture.join("tools")],
    })
    .unwrap()
}

fn locked_request(fixture: &TestDirectory) -> LockedSignatureCacheRequest {
    LockedSignatureCacheRequest::new(
        fixture.join("locked.inc"),
        fixture.join("input-cache"),
        fixture.join("output-cache"),
        "ubuntu-24.04".into(),
        Some(fixture.join("filter.txt")),
    )
    .unwrap()
}

fn comparison_request(fixture: &TestDirectory) -> BuildComparisonRequest {
    BuildComparisonRequest::new(BuildComparisonRequest {
        repository: fixture.join("history"),
        from_revision: Some("HEAD^".into()),
        to_revision: Some("HEAD".into()),
        report_version: true,
        report_all: true,
        signatures: true,
        signature_diff: true,
        exclude_paths: vec!["images/*".into()],
        no_colour: true,
    })
    .unwrap()
}

fn archive_request(fixture: &TestDirectory, remote: Option<&str>) -> GitArchiveRequest {
    GitArchiveRequest::new(GitArchiveRequest {
        data_dir: fixture.join("archive-data"),
        git_dir: fixture.join("archive.git"),
        create: true,
        bare: false,
        create_tag: true,
        branch_name: "release/{machine}".into(),
        tag_name: Some("release/{tag_number}".into()),
        commit_subject: "Release {commit}".into(),
        commit_body: "machine: {machine}".into(),
        tag_subject: "Release tag {tag_number}".into(),
        tag_body: "archived by Yoctui".into(),
        exclusions: vec!["tmp/*".into()],
        notes: vec![("release".into(), fixture.join("note.txt"))],
        push_remote: remote.map(str::to_owned),
    })
    .unwrap()
}

mod maintenance_release_capability_keeps_optional_build_compare_distinct;

mod maintenance_release_locked_signature_vector_and_changed_evidence_are_exact;

mod maintenance_release_locked_signature_revalidates_every_input_before_spawn;

mod maintenance_release_buildhistory_vector_uses_only_documented_flags;

mod maintenance_release_archive_separates_local_result_from_network_push;

mod maintenance_release_archive_push_rejects_changed_local_head;

async fn terminal_event(runner: &mut MaintenanceSstateJobRunner) -> MaintenanceSstateRunnerEvent {
    loop {
        let event = runner.next_event().await.unwrap();
        if matches!(
            event,
            MaintenanceSstateRunnerEvent::Completed { .. }
                | MaintenanceSstateRunnerEvent::Failed { .. }
                | MaintenanceSstateRunnerEvent::Cancelled { .. }
                | MaintenanceSstateRunnerEvent::TimedOut { .. }
                | MaintenanceSstateRunnerEvent::Lost { .. }
        ) {
            return event;
        }
    }
}

mod maintenance_release_shared_runner_keeps_success_nonzero_timeout_cancel_and_loss_typed;
