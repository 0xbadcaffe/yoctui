use std::{
    collections::{BTreeMap, VecDeque},
    ffi::{OsStr, OsString},
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    process::{Child, Command},
    time::Instant,
};
use yoctui_model::{
    MAX_MAINTENANCE_LIMITATIONS, MAX_MAINTENANCE_PATHS, MAX_MAINTENANCE_TEXT_BYTES,
    MaintenanceCapabilitySnapshot, MaintenanceFileIdentity, MaintenanceMetadata,
    MaintenanceOperation, MaintenanceOperationPreview, MaintenanceOutputStream,
    MaintenanceSessionId, MaintenanceTool, MaintenanceToolCapability, MaintenanceToolInterface,
    PrServiceOperation, PrServiceRequest, SstateCleanupMode, SstateCleanupPreview,
    SstateCleanupRequest, SstateReadinessMode, SstateReadinessRequest,
};
use yoctui_utils::is_transient_spawn_error;

const SSTATE_EVENT_CHANNEL_CAPACITY: usize = 64;
const SSTATE_OPERATION_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const MAX_PREVIEW_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const SPAWN_ATTEMPTS: usize = 4;
const SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);

include!("maintenance_sstate/capability_inspection.rs");
include!("maintenance_sstate/filesystem_validation.rs");
include!("maintenance_sstate/command_types.rs");
include!("maintenance_sstate/command_planning.rs");
include!("maintenance_sstate/preview_and_events.rs");
include!("maintenance_sstate/job_runner.rs");

#[cfg(test)]
mod tests {
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
    #[test]
    fn maintenance_sstate_capability_distinguishes_python_legacy_missing_and_unsafe() {
        let (root, snapshot) = fixture(
            "python-capability",
            "sstate-cache-management.py",
            "#!/bin/sh\nexit 0\n",
        );
        assert!(matches!(
            snapshot.capability(MaintenanceTool::SstateCacheManagement),
            Some(MaintenanceToolCapability::Available {
                interface: MaintenanceToolInterface::SstatePython,
                ..
            })
        ));
        fs::remove_file(root.0.join("bin/sstate-cache-management.py")).unwrap();
        write_executable(
            &root.0.join("bin/sstate-cache-management.sh"),
            "#!/bin/sh\nexit 0\n",
        );
        let legacy =
            MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
                build_dir: root.0.join("build"),
                sstate_dir: Some(root.0.join("cache")),
                tmp_dir: Some(root.0.join("tmp")),
                stamps_dirs: vec![root.0.join("stamps")],
                executable_search_path: vec![root.0.join("bin")],
            })
            .unwrap();
        assert!(matches!(
            legacy.capability(MaintenanceTool::SstateCacheManagement),
            Some(MaintenanceToolCapability::Available {
                interface: MaintenanceToolInterface::SstateLegacyShell,
                ..
            })
        ));
        fs::remove_file(root.0.join("bin/sstate-cache-management.sh")).unwrap();
        let missing =
            MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
                build_dir: root.0.join("build"),
                sstate_dir: Some(root.0.join("cache")),
                tmp_dir: None,
                stamps_dirs: vec![],
                executable_search_path: vec![root.0.join("bin")],
            })
            .unwrap();
        assert!(matches!(
            missing.capability(MaintenanceTool::SstateCacheManagement),
            Some(MaintenanceToolCapability::Unavailable { .. })
        ));

        let linked = root.0.join("linked-bin");
        symlink(root.0.join("bin"), &linked).unwrap();
        let unsafe_snapshot =
            MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
                build_dir: root.0.join("build"),
                sstate_dir: Some(root.0.join("cache")),
                tmp_dir: None,
                stamps_dirs: vec![],
                executable_search_path: vec![linked],
            })
            .unwrap();
        assert!(!unsafe_snapshot.limitations.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn maintenance_sstate_reconstructs_exact_readiness_and_cleanup_vectors() {
        let (root, snapshot) = fixture(
            "vectors",
            "sstate-cache-management.py",
            "#!/bin/sh\nexit 0\n",
        );
        let output = root.0.join("output/readiness.txt");
        let (preview, command) = MaintenanceSstateCommandSpec::readiness(
            MaintenanceSessionId(1),
            3,
            &snapshot,
            9,
            SstateReadinessRequest::new(
                vec!["busybox".into(), "core-image-minimal".into()],
                SstateReadinessMode::SameTmpdir,
                Some(output.clone()),
                None,
                60,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            command.arguments(),
            &[
                OsString::from("--outfile"),
                output.into_os_string(),
                OsString::from("--same-tmpdir"),
                OsString::from("busybox"),
                OsString::from("core-image-minimal"),
            ]
        );
        assert_eq!(
            command.environment().get(OsStr::new("BB_SETSCENE_ENFORCE")),
            Some(&OsString::from("1"))
        );
        assert_eq!(
            preview.arguments[0],
            format!(
                "0: {}",
                snapshot
                    .capability(MaintenanceTool::OeCheckSstate)
                    .and_then(|capability| match capability {
                        MaintenanceToolCapability::Available { executable, .. } =>
                            Some(executable.path.display().to_string()),
                        _ => None,
                    })
                    .unwrap()
            )
        );

        let cleanup = MaintenanceSstateCommandSpec::cleanup_preview(
            MaintenanceSessionId(2),
            &snapshot,
            cleanup_request(&root),
        )
        .unwrap();
        let arguments = cleanup
            .arguments()
            .iter()
            .map(|value| value.to_string_lossy())
            .collect::<Vec<_>>();
        assert!(arguments.contains(&std::borrow::Cow::Borrowed("--remove-duplicated")));
        assert!(arguments.contains(&std::borrow::Cow::Borrowed("--remove-orphans")));
        assert!(arguments.contains(&std::borrow::Cow::Borrowed("--stamps-dir")));
        assert_eq!(
            arguments.last().map(|value| value.as_ref()),
            Some("--debug")
        );
    }

    #[cfg(unix)]
    #[test]
    fn maintenance_sstate_preview_is_bounded_and_cleanup_rejects_changes_or_tampering() {
        let (root, snapshot) = fixture(
            "preview",
            "sstate-cache-management.py",
            "#!/bin/sh\nexit 0\n",
        );
        let candidate = root.0.join("cache/sstate:a");
        fs::write(&candidate, "one").unwrap();
        let request = cleanup_request(&root);
        let confirmed =
            parse_cleanup_preview(request.clone(), &[candidate.display().to_string()]).unwrap();
        assert_eq!(confirmed.candidates.len(), 1);
        assert!(
            parse_cleanup_preview(request.clone(), &[])
                .unwrap()
                .candidates
                .is_empty()
        );
        let (execution_preview, execution) = MaintenanceSstateCommandSpec::cleanup_execution(
            MaintenanceSessionId(3),
            1,
            &snapshot,
            3,
            &confirmed,
            &confirmed,
        )
        .unwrap();
        assert_eq!(execution.arguments().last(), Some(&OsString::from("--yes")));
        assert!(
            !execution
                .arguments()
                .iter()
                .any(|argument| argument == OsStr::new("--debug"))
        );
        assert_eq!(
            execution_preview.operation,
            MaintenanceOperation::SstateCleanup(confirmed.clone())
        );
        let changed = SstateCleanupPreview::new(request.clone(), vec![]).unwrap();
        assert!(matches!(
            MaintenanceSstateCommandSpec::cleanup_execution(
                MaintenanceSessionId(3),
                1,
                &snapshot,
                3,
                &confirmed,
                &changed,
            ),
            Err(MaintenanceSstateAdapterError::CandidateMismatch)
        ));
        fs::write(&candidate, "changed").unwrap();
        assert!(matches!(
            MaintenanceSstateCommandSpec::cleanup_execution(
                MaintenanceSessionId(3),
                1,
                &snapshot,
                3,
                &confirmed,
                &confirmed,
            ),
            Err(MaintenanceSstateAdapterError::StaleIdentity(path)) if path == candidate
        ));
        assert!(
            parse_cleanup_preview(
                request,
                &[format!("/outside/{}", "x".repeat(MAX_PREVIEW_OUTPUT_BYTES))]
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn maintenance_sstate_runner_streams_bounded_output_and_terminal_status() {
        let (_root, snapshot) = fixture(
            "runner",
            "sstate-cache-management.py",
            &format!(
                "#!/bin/sh\nprintf 'out\\n'\nprintf '%*s\\n' {} x >&2\nexit 0\n",
                MAX_MAINTENANCE_TEXT_BYTES + 64
            ),
        );
        let (_, command) = MaintenanceSstateCommandSpec::readiness(
            MaintenanceSessionId(4),
            1,
            &snapshot,
            4,
            SstateReadinessRequest::new(
                vec!["busybox".into()],
                SstateReadinessMode::IsolatedTmpdir,
                None,
                None,
                60,
            )
            .unwrap(),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(command).await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Started { .. }
        ));
        let mut truncated = false;
        loop {
            match runner.next_event().await.unwrap() {
                MaintenanceSstateRunnerEvent::Output {
                    truncated: value, ..
                } => truncated |= value,
                MaintenanceSstateRunnerEvent::Completed {
                    exit_code: Some(0), ..
                } => break,
                event => panic!("unexpected event {event:?}"),
            }
        }
        assert!(truncated);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn maintenance_sstate_cleanup_preview_receives_negative_input_and_revalidates_tool() {
        let (root, snapshot) = fixture(
            "preview-runner",
            "sstate-cache-management.py",
            "#!/bin/sh\nread answer\nprintf '%s\\n' \"$answer\"\nexit 0\n",
        );
        let command = MaintenanceSstateCommandSpec::cleanup_preview(
            MaintenanceSessionId(40),
            &snapshot,
            cleanup_request(&root),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(command).await.unwrap();
        runner.next_event().await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Output {
                stream: MaintenanceOutputStream::Stdout,
                line,
                ..
            } if line == "n"
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Completed {
                exit_code: Some(0),
                ..
            }
        ));

        let command = MaintenanceSstateCommandSpec::cleanup_preview(
            MaintenanceSessionId(41),
            &snapshot,
            cleanup_request(&root),
        )
        .unwrap();
        write_executable(
            &root.0.join("bin/sstate-cache-management.py"),
            "#!/bin/sh\nprintf 'tampered\\n'\nexit 0\n",
        );
        assert!(matches!(
            MaintenanceSstateJobRunner::new().start(command).await,
            Err(MaintenanceSstateAdapterError::StaleIdentity(_))
        ));
    }

    #[tokio::test]
    async fn maintenance_sstate_closed_stdin_does_not_mask_child_outcome() {
        let (mut stdin, child_stdin) = tokio::io::duplex(1);
        drop(child_stdin);

        write_process_stdin(&mut stdin, b"n\n").await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn maintenance_sstate_spawn_retries_only_transient_text_file_busy() {
        let root = TestDirectory::new("spawn-retry");
        let executable = root.0.join("runner");
        write_executable(&executable, "#!/bin/sh\nexit 0\n");
        let writer = fs::OpenOptions::new()
            .write(true)
            .open(&executable)
            .unwrap();
        let release = tokio::spawn(async move {
            tokio::time::sleep(SPAWN_RETRY_DELAY + SPAWN_RETRY_DELAY).await;
            drop(writer);
        });
        let mut process = Command::new(&executable);
        let mut child = spawn_process(&mut process).await.unwrap();
        assert!(child.wait().await.unwrap().success());
        release.await.unwrap();

        let writer = fs::OpenOptions::new()
            .write(true)
            .open(&executable)
            .unwrap();
        let mut process = Command::new(&executable);
        let error = match spawn_process(&mut process).await {
            Ok(_) => panic!("write-held executable unexpectedly spawned"),
            Err(error) => error,
        };
        assert!(is_transient_spawn_error(&error));
        drop(writer);

        assert!(!is_transient_spawn_error(
            &std::io::Error::from_raw_os_error(libc::EACCES)
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn maintenance_sstate_fast_cleanup_failure_retains_status_and_stderr() {
        let (root, snapshot) = fixture(
            "fast-preview-failure",
            "sstate-cache-management.py",
            "#!/bin/sh\nexec 0<&-\nprintf 'denied\\n' >&2\nexit 9\n",
        );
        let command = MaintenanceSstateCommandSpec::cleanup_preview(
            MaintenanceSessionId(42),
            &snapshot,
            cleanup_request(&root),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(command).await.unwrap();

        let mut stderr = None;
        loop {
            match runner.next_event().await.unwrap() {
                MaintenanceSstateRunnerEvent::Started { .. } => {}
                MaintenanceSstateRunnerEvent::Output {
                    stream: MaintenanceOutputStream::Stderr,
                    line,
                    ..
                } => stderr = Some(line),
                MaintenanceSstateRunnerEvent::Failed {
                    exit_code: Some(9), ..
                } => break,
                event => panic!("unexpected event {event:?}"),
            }
        }
        assert_eq!(stderr.as_deref(), Some("denied"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn maintenance_sstate_runner_reports_nonzero_duplicate_and_cancellation() {
        let (_root, snapshot) = fixture(
            "cancel",
            "sstate-cache-management.py",
            "#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n",
        );
        let (_, command) = MaintenanceSstateCommandSpec::readiness(
            MaintenanceSessionId(5),
            1,
            &snapshot,
            5,
            SstateReadinessRequest::new(
                vec!["busybox".into()],
                SstateReadinessMode::IsolatedTmpdir,
                None,
                None,
                60,
            )
            .unwrap(),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(command.clone()).await.unwrap();
        assert!(matches!(
            runner.start(command).await,
            Err(MaintenanceSstateAdapterError::Busy)
        ));
        assert!(runner.cancel(MaintenanceSessionId(5)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::CancellationRequested { .. }
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Cancelled { forced: false, .. }
        ));

        let (_root, snapshot) = fixture(
            "nonzero",
            "sstate-cache-management.py",
            "#!/bin/sh\nexit 7\n",
        );
        let (_, command) = MaintenanceSstateCommandSpec::readiness(
            MaintenanceSessionId(6),
            1,
            &snapshot,
            6,
            SstateReadinessRequest::new(
                vec!["busybox".into()],
                SstateReadinessMode::IsolatedTmpdir,
                None,
                None,
                60,
            )
            .unwrap(),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(command).await.unwrap();
        runner.next_event().await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Failed {
                exit_code: Some(7),
                ..
            }
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn maintenance_sstate_runner_preserves_timeout_forced_cancel_rejection_and_loss() {
        let (_root, snapshot) = fixture(
            "timeout",
            "sstate-cache-management.py",
            "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
        );
        let make_command = |id| {
            MaintenanceSstateCommandSpec::readiness(
                MaintenanceSessionId(id),
                1,
                &snapshot,
                id,
                SstateReadinessRequest::new(
                    vec!["busybox".into()],
                    SstateReadinessMode::IsolatedTmpdir,
                    None,
                    None,
                    60,
                )
                .unwrap(),
            )
            .unwrap()
            .1
        };
        let mut runner = MaintenanceSstateJobRunner::new()
            .with_operation_timeout(Duration::from_secs(2))
            .with_cancellation_timeout(Duration::from_millis(20));
        runner.start(make_command(7)).await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Started { .. }
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Output { ref line, .. } if line == "ready"
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::TimedOut { forced: true, .. }
        ));
        assert!(!runner.cancel(MaintenanceSessionId(99)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::CancellationRejected { .. }
        ));

        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(make_command(8)).await.unwrap();
        runner.next_event().await.unwrap();
        runner.lose_output_channel();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Lost { .. }
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn hardening_stress_process_tree_cancellation_reaps_descendant() {
        let (root, snapshot) = fixture(
            "process-tree-stress",
            "sstate-cache-management.py",
            "#!/bin/sh\n(\n  trap '' TERM\n  while :; do sleep 1; done\n) &\ndescendant=$!\nprintf '%s\\n' \"$descendant\" > descendant.pid\ntrap 'wait \"$descendant\"' TERM\nwhile :; do sleep 1; done\n",
        );
        let descendant_file = root.0.join("build/descendant.pid");
        let (_, command) = MaintenanceSstateCommandSpec::readiness(
            MaintenanceSessionId(70),
            1,
            &snapshot,
            70,
            SstateReadinessRequest::new(
                vec!["busybox".into()],
                SstateReadinessMode::IsolatedTmpdir,
                None,
                None,
                60,
            )
            .unwrap(),
        )
        .unwrap();
        let mut runner =
            MaintenanceSstateJobRunner::new().with_cancellation_timeout(Duration::from_millis(50));
        runner.start(command).await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Started { .. }
        ));

        let mut descendant = None;
        for _ in 0..200 {
            descendant = fs::read_to_string(&descendant_file)
                .ok()
                .and_then(|contents| contents.trim().parse::<i32>().ok());
            if descendant.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let descendant = descendant.expect("fixture did not publish a descendant PID");
        // SAFETY: signal zero only probes the exact child PID written by the fixture.
        assert_eq!(unsafe { libc::kill(descendant, 0) }, 0);

        assert!(runner.cancel(MaintenanceSessionId(70)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::CancellationRequested { .. }
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Cancelled { forced: true, .. }
        ));
        let mut reaped = false;
        for _ in 0..200 {
            // SAFETY: signal zero only probes the previously observed fixture PID.
            if unsafe { libc::kill(descendant, 0) } != 0 {
                reaped = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(reaped, "cancelled process-group descendant survived");
    }
}
