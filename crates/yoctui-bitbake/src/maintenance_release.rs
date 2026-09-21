use std::{
    collections::{BTreeMap, VecDeque},
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use thiserror::Error;
use yoctui_model::{
    BuildComparisonRequest, GitArchiveRequest, LockedSignatureCacheRequest,
    MAX_MAINTENANCE_ARGUMENTS, MAX_MAINTENANCE_EVIDENCE, MAX_MAINTENANCE_LIMITATIONS,
    MAX_MAINTENANCE_PATHS, MAX_MAINTENANCE_TEXT_BYTES, MaintenanceCapabilitySnapshot,
    MaintenanceEvidence, MaintenanceFileIdentity, MaintenanceMetadata, MaintenanceOperation,
    MaintenanceOperationPreview, MaintenanceSessionId, MaintenanceTool, MaintenanceToolCapability,
    MaintenanceToolInterface,
};

use crate::maintenance_sstate::{
    MaintenanceExternalCommand, MaintenanceFilesystemGuard, MaintenanceSstateAdapterError,
    MaintenanceSstateCommandKind, MaintenanceSstateCommandSpec, guard_directory,
    guard_directory_or_absent, guard_regular_file,
};

const RELEASE_OPERATION_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const MAX_EVIDENCE_SCAN_DIRECTORIES: usize = 4_096;

include!("maintenance_release/capability_and_commands.rs");
include!("maintenance_release/arguments_and_validation.rs");
include!("maintenance_release/evidence_snapshot.rs");

#[cfg(test)]
mod tests {
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

    #[test]
    fn maintenance_release_capability_keeps_optional_build_compare_distinct() {
        let fixture = TestDirectory::new("capability");
        prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
        executable(&fixture.join("tools/build-compare"), "#!/bin/sh\nexit 0\n");
        let snapshot = inspect(&fixture);
        for tool in [
            MaintenanceTool::LockedSignatureCache,
            MaintenanceTool::BuildHistoryDiff,
            MaintenanceTool::GitArchive,
        ] {
            assert!(snapshot.supports(tool));
        }
        assert!(!snapshot.supports(MaintenanceTool::BuildCompare));
        assert!(
            snapshot
                .limitations
                .iter()
                .any(|line| line.contains("not the buildhistory-diff interface"))
        );
        assert!(matches!(
            build_compare_command(
                MaintenanceSessionId(1),
                &snapshot,
                comparison_request(&fixture)
            ),
            Err(MaintenanceReleaseAdapterError::Unavailable(_))
        ));

        #[cfg(unix)]
        {
            let unsafe_fixture = TestDirectory::new("unsafe-capability");
            prepare_fixture(&unsafe_fixture, "#!/bin/sh\nexit 0\n");
            fs::remove_file(unsafe_fixture.join("tools/gen-lockedsig-cache")).unwrap();
            let real = unsafe_fixture.join("real-tool");
            executable(&real, "#!/bin/sh\nexit 0\n");
            symlink(&real, unsafe_fixture.join("tools/gen-lockedsig-cache")).unwrap();
            let snapshot = inspect(&unsafe_fixture);
            assert!(!snapshot.supports(MaintenanceTool::LockedSignatureCache));
            assert!(
                snapshot
                    .limitations
                    .iter()
                    .any(|line| line.contains("unsafe executable"))
            );
        }
    }

    #[test]
    fn maintenance_release_locked_signature_vector_and_changed_evidence_are_exact() {
        let fixture = TestDirectory::new("locked");
        prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
        let snapshot = inspect(&fixture);
        let request = locked_request(&fixture);
        let (preview, command, before) =
            locked_signature_command(MaintenanceSessionId(1), 2, &snapshot, 3, request.clone())
                .unwrap();
        assert_eq!(
            command.kind(),
            MaintenanceSstateCommandKind::LockedSignatureCache
        );
        assert_eq!(
            command.arguments(),
            [
                request.locked_signatures.as_os_str().to_owned(),
                request.input_cache.as_os_str().to_owned(),
                request.output_cache.as_os_str().to_owned(),
                OsString::from("ubuntu-24.04"),
                request.filter.unwrap().as_os_str().to_owned(),
            ]
        );
        assert!(
            preview
                .limitations
                .iter()
                .any(|line| line.contains("may be replaced"))
        );
        let created = fixture.join("output-cache/aa/new.siginfo");
        fs::create_dir_all(created.parent().unwrap()).unwrap();
        fs::write(&created, "sig\n").unwrap();
        let evidence = before.changed_evidence().unwrap();
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].identity.path, created);
    }

    #[tokio::test]
    async fn maintenance_release_locked_signature_revalidates_every_input_before_spawn() {
        let fixture = TestDirectory::new("locked-stale");
        prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
        let snapshot = inspect(&fixture);
        let (_, command, _) = locked_signature_command(
            MaintenanceSessionId(1),
            1,
            &snapshot,
            1,
            locked_request(&fixture),
        )
        .unwrap();
        fs::write(fixture.join("locked.inc"), "changed input identity\n").unwrap();
        assert!(matches!(
            MaintenanceSstateJobRunner::new().start(command).await,
            Err(MaintenanceSstateAdapterError::StaleIdentity(path))
                if path == fixture.join("locked.inc")
        ));
    }

    #[test]
    fn maintenance_release_buildhistory_vector_uses_only_documented_flags() {
        let fixture = TestDirectory::new("buildhistory");
        prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
        let snapshot = inspect(&fixture);
        let request = comparison_request(&fixture);
        let (preview, command) =
            buildhistory_command(MaintenanceSessionId(2), 3, &snapshot, 4, request).unwrap();
        assert_eq!(
            command.arguments(),
            [
                OsString::from("-p"),
                fixture.join("history").into_os_string(),
                OsString::from("-v"),
                OsString::from("-a"),
                OsString::from("-s"),
                OsString::from("-S"),
                OsString::from("-e"),
                OsString::from("images/*"),
                OsString::from("-c"),
                OsString::from("no"),
                OsString::from("HEAD^"),
                OsString::from("HEAD"),
            ]
        );
        assert!(matches!(
            preview.operation,
            MaintenanceOperation::BuildHistoryComparison(_)
        ));
        let mut invalid = comparison_request(&fixture);
        invalid.from_revision = Some("--help".into());
        assert!(buildhistory_command(MaintenanceSessionId(3), 3, &snapshot, 5, invalid).is_err());
    }

    #[test]
    fn maintenance_release_archive_separates_local_result_from_network_push() {
        let fixture = TestDirectory::new("archive");
        prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
        let snapshot = inspect(&fixture);
        let request = archive_request(&fixture, Some("origin"));
        let (local_preview, local_command) =
            git_archive_local_command(MaintenanceSessionId(4), 1, &snapshot, 6, &request).unwrap();
        assert_eq!(
            local_command.kind(),
            MaintenanceSstateCommandKind::GitArchiveLocal
        );
        assert!(
            !local_command
                .arguments()
                .iter()
                .any(|argument| argument == "--push")
        );
        assert!(!local_preview.operation.network_side_effect());

        git_repository(&request.git_dir);
        let local_result = GitArchiveLocalResult::capture(&request).unwrap();
        let (push_preview, push_command) = git_archive_push_command(
            MaintenanceSessionId(5),
            1,
            &snapshot,
            7,
            request.clone(),
            &local_result,
        )
        .unwrap();
        assert_eq!(
            push_command.kind(),
            MaintenanceSstateCommandKind::GitArchivePush
        );
        let push_index = push_command
            .arguments()
            .iter()
            .position(|argument| argument == "--push")
            .unwrap();
        assert_eq!(push_command.arguments()[push_index + 1], "origin");
        assert!(push_preview.operation.network_side_effect());
        assert!(
            push_preview
                .limitations
                .iter()
                .any(|line| line.contains("after the retained local archive result"))
        );
    }

    #[tokio::test]
    async fn maintenance_release_archive_push_rejects_changed_local_head() {
        let fixture = TestDirectory::new("archive-stale");
        prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
        let snapshot = inspect(&fixture);
        let request = archive_request(&fixture, Some("origin"));
        git_repository(&request.git_dir);
        let local_result = GitArchiveLocalResult::capture(&request).unwrap();
        let (_, command) = git_archive_push_command(
            MaintenanceSessionId(1),
            1,
            &snapshot,
            1,
            request.clone(),
            &local_result,
        )
        .unwrap();
        fs::write(request.git_dir.join(".git/HEAD"), "changed\n").unwrap();
        assert!(matches!(
            MaintenanceSstateJobRunner::new().start(command).await,
            Err(MaintenanceSstateAdapterError::StaleIdentity(_))
        ));
    }

    async fn terminal_event(
        runner: &mut MaintenanceSstateJobRunner,
    ) -> MaintenanceSstateRunnerEvent {
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

    #[tokio::test]
    async fn maintenance_release_shared_runner_keeps_success_nonzero_timeout_cancel_and_loss_typed()
    {
        for (name, body, expected_success) in [
            ("runner-success", "#!/bin/sh\necho release\nexit 0\n", true),
            (
                "runner-failure",
                "#!/bin/sh\necho failed >&2\nexit 8\n",
                false,
            ),
        ] {
            let fixture = TestDirectory::new(name);
            prepare_fixture(&fixture, body);
            let snapshot = inspect(&fixture);
            let (_, command) = buildhistory_command(
                MaintenanceSessionId(8),
                1,
                &snapshot,
                1,
                comparison_request(&fixture),
            )
            .unwrap();
            let mut runner = MaintenanceSstateJobRunner::new();
            runner.start(command).await.unwrap();
            assert!(matches!(
                runner.next_event().await.unwrap(),
                MaintenanceSstateRunnerEvent::Started { .. }
            ));
            let terminal = terminal_event(&mut runner).await;
            assert_eq!(
                matches!(terminal, MaintenanceSstateRunnerEvent::Completed { .. }),
                expected_success
            );
            if !expected_success {
                assert!(matches!(
                    terminal,
                    MaintenanceSstateRunnerEvent::Failed {
                        exit_code: Some(8),
                        ..
                    }
                ));
            }
        }

        let forced = TestDirectory::new("runner-timeout");
        prepare_fixture(
            &forced,
            "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
        );
        let forced_snapshot = inspect(&forced);
        let (_, forced_command) = buildhistory_command(
            MaintenanceSessionId(9),
            1,
            &forced_snapshot,
            1,
            comparison_request(&forced),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new()
            .with_operation_timeout(Duration::from_millis(200))
            .with_cancellation_timeout(Duration::from_millis(10));
        runner.start(forced_command).await.unwrap();
        assert!(matches!(
            terminal_event(&mut runner).await,
            MaintenanceSstateRunnerEvent::TimedOut { forced: true, .. }
        ));

        let graceful = TestDirectory::new("runner-cancel");
        prepare_fixture(
            &graceful,
            "#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n",
        );
        let graceful_snapshot = inspect(&graceful);
        let make_command = || {
            buildhistory_command(
                MaintenanceSessionId(10),
                1,
                &graceful_snapshot,
                1,
                comparison_request(&graceful),
            )
            .unwrap()
            .1
        };
        let mut cancelled =
            MaintenanceSstateJobRunner::new().with_cancellation_timeout(Duration::from_millis(100));
        cancelled.start(make_command()).await.unwrap();
        cancelled.next_event().await.unwrap();
        assert!(cancelled.cancel(MaintenanceSessionId(10)).await.unwrap());
        assert!(matches!(
            cancelled.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::CancellationRequested { .. }
        ));
        assert!(matches!(
            cancelled.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Cancelled { .. }
        ));
        assert!(!cancelled.cancel(MaintenanceSessionId(10)).await.unwrap());
        assert!(matches!(
            cancelled.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::CancellationRejected { .. }
        ));

        let mut lost = MaintenanceSstateJobRunner::new();
        lost.start(make_command()).await.unwrap();
        lost.next_event().await.unwrap();
        lost.lose_output_channel();
        assert!(matches!(
            lost.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Lost { .. }
        ));
    }
}
