include!("test_runner/adapter_and_capability.rs");

include!("test_runner/command_and_discovery.rs");

include!("test_runner/job_runner.rs");

#[cfg(test)]
mod tests {
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
    #[test]
    fn test_runner_capability_distinguishes_missing_and_unsafe_executables() {
        let directory = TestDirectory::new("capability");
        let bin = directory.path().join("bin");
        fs::create_dir(&bin).unwrap();
        executable(&bin.join("oe-selftest"), "#!/bin/sh\nexit 0\n");
        let inspector =
            TestRunnerCapabilityInspector::new(vec![bin.clone()], PtestCapability::Configured);
        assert!(matches!(
            inspector.inspect(),
            TestCapability {
                oe_selftest: TestExecutableCapability::Available(_),
                bitbake_selftest: TestExecutableCapability::Missing,
                ptest: PtestCapability::Configured,
            }
        ));

        let outside = directory.path().join("outside");
        executable(&outside, "#!/bin/sh\nexit 0\n");
        symlink(&outside, bin.join("bitbake-selftest")).unwrap();
        assert!(matches!(
            inspector.inspect().bitbake_selftest,
            TestExecutableCapability::Failed(_)
        ));
        assert!(matches!(
            TestRunnerCapabilityInspector::new(
                vec![directory.path().join("missing")],
                PtestCapability::NotInspected
            )
            .inspect()
            .oe_selftest,
            TestExecutableCapability::Failed(_)
        ));
    }

    #[test]
    fn test_runner_commands_are_exact_revalidated_and_child_environment_only() {
        let (_directory, adapter) = fixture("commands");
        let oe = request(&adapter, TestFamily::OeSelftest);
        let oe_command = adapter.command(&oe).unwrap();
        assert_eq!(
            oe_command.arguments(),
            ["-r", "tinfoil.Case.test_one", "-j", "4"]
        );
        assert!(oe_command.environment().is_empty());

        let bitbake = request(&adapter, TestFamily::BitbakeSelftest);
        let command = adapter.command(&bitbake).unwrap();
        assert_eq!(command.arguments(), ["-v"]);
        assert_eq!(
            command.environment().get(OsStr::new("BB_SKIP_NETTESTS")),
            Some(&OsString::from("yes"))
        );
        assert!(std::env::var_os("BB_SKIP_NETTESTS").is_none());

        let mut invalid = oe;
        invalid.skip_network = true;
        assert!(matches!(
            adapter.command(&invalid),
            Err(TestRunnerAdapterError::InvalidRequest(_))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_runner_spawn_retries_only_transient_text_file_busy() {
        let directory = TestDirectory::new("spawn-retry");
        let program = directory.path().join("test-runner");
        executable(&program, "#!/bin/sh\nexit 0\n");
        let writer = fs::OpenOptions::new().write(true).open(&program).unwrap();
        let release = tokio::spawn(async move {
            tokio::time::sleep(TEST_RUNNER_SPAWN_RETRY_DELAY + TEST_RUNNER_SPAWN_RETRY_DELAY).await;
            drop(writer);
        });
        let mut process = Command::new(&program);
        let mut child = spawn_test_runner_process(&mut process).await.unwrap();
        assert!(child.wait().await.unwrap().success());
        release.await.unwrap();

        let writer = fs::OpenOptions::new().write(true).open(&program).unwrap();
        let mut process = Command::new(&program);
        let error = match spawn_test_runner_process(&mut process).await {
            Ok(_) => panic!("write-held Testing executable unexpectedly spawned"),
            Err(error) => error,
        };
        assert!(is_transient_test_runner_spawn_error(&error));
        drop(writer);

        assert!(!is_transient_test_runner_spawn_error(
            &io::Error::from_raw_os_error(libc::EACCES)
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_runner_rejects_tampering_streams_bounded_output_and_completes() {
        let (_directory, adapter) = fixture("stream");
        let request = request(&adapter, TestFamily::BitbakeSelftest);
        let tool = request.executable.clone();
        let command = adapter.command(&request).unwrap();
        executable(&tool, "#!/bin/sh\nexit 0\n");
        let mut stale = TestRunnerJob::new();
        assert!(matches!(
            stale.start(command).await,
            Err(TestRunnerAdapterError::StaleExecutable(_))
        ));

        executable(
            &tool,
            &format!(
                "#!/bin/sh\nprintf 'env=%s\\n' \"$BB_SKIP_NETTESTS\"\nprintf 'stderr\\n' >&2\nprintf '{}\\n'\nexit 0\n",
                "x".repeat(MAX_TEST_RUNNER_LINE_BYTES + 8)
            ),
        );
        let refreshed = TestSelftestRequest::new(
            tool,
            request.family,
            request.selector,
            request.parallelism,
            request.verbose,
            request.skip_network,
        )
        .unwrap();
        let command = adapter.command(&refreshed).unwrap();
        let mut runner = TestRunnerJob::new();
        runner.start(command.clone()).await.unwrap();
        assert_eq!(
            runner.start(command).await,
            Err(TestRunnerAdapterError::Busy)
        );
        assert_eq!(runner.next_event().await.unwrap(), TestRunnerEvent::Started);
        let mut stdout = false;
        let mut stderr = false;
        let mut truncated = false;
        loop {
            match runner.next_event().await.unwrap() {
                TestRunnerEvent::Output {
                    stream,
                    line,
                    truncated: line_truncated,
                } => {
                    stdout |= stream == TestOutputStream::Stdout;
                    stderr |= stream == TestOutputStream::Stderr;
                    truncated |= line_truncated;
                    if line.starts_with("env=") {
                        assert_eq!(line, "env=yes");
                    }
                }
                TestRunnerEvent::Completed {
                    exit_code,
                    result_paths,
                } => {
                    assert_eq!(exit_code, Some(0));
                    assert!(result_paths.is_empty());
                    break;
                }
                event => panic!("unexpected event: {event:?}"),
            }
        }
        assert!(stdout && stderr && truncated);
        assert!(std::env::var_os("BB_SKIP_NETTESTS").is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_runner_reports_nonzero_worker_loss_and_cancellation_rejection() {
        let (_directory, adapter) = fixture("outcomes");
        let mut request = request(&adapter, TestFamily::OeSelftest);
        executable(&request.executable, "#!/bin/sh\nexit 7\n");
        request = TestSelftestRequest::new(
            request.executable,
            request.family,
            request.selector,
            request.parallelism,
            request.verbose,
            request.skip_network,
        )
        .unwrap();
        let command = adapter.command(&request).unwrap();
        let mut runner = TestRunnerJob::new();
        runner.start(command).await.unwrap();
        assert_eq!(runner.next_event().await.unwrap(), TestRunnerEvent::Started);
        assert_eq!(
            runner.next_event().await.unwrap(),
            TestRunnerEvent::Failed { exit_code: Some(7) }
        );
        assert!(!runner.cancel().await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            TestRunnerEvent::CancellationRejected { .. }
        ));

        executable(&request.executable, "#!/bin/sh\nsleep 2\n");
        let request = TestSelftestRequest::new(
            request.executable,
            request.family,
            request.selector,
            request.parallelism,
            request.verbose,
            request.skip_network,
        )
        .unwrap();
        runner
            .start(adapter.command(&request).unwrap())
            .await
            .unwrap();
        assert_eq!(runner.next_event().await.unwrap(), TestRunnerEvent::Started);
        runner.lose_output_channel();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            TestRunnerEvent::Lost { .. }
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_runner_cancels_gracefully_forcibly_and_times_out() {
        let (_directory, adapter) = fixture("control");
        let mut request = request(&adapter, TestFamily::OeSelftest);
        executable(
            &request.executable,
            "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n",
        );
        request = TestSelftestRequest::new(
            request.executable,
            request.family,
            request.selector,
            request.parallelism,
            request.verbose,
            request.skip_network,
        )
        .unwrap();
        let mut graceful = TestRunnerJob::new().with_cancellation_timeout(Duration::from_secs(1));
        graceful
            .start(adapter.command(&request).unwrap())
            .await
            .unwrap();
        assert_eq!(
            graceful.next_event().await.unwrap(),
            TestRunnerEvent::Started
        );
        assert!(matches!(
            graceful.next_event().await.unwrap(),
            TestRunnerEvent::Output { .. }
        ));
        assert!(graceful.cancel().await.unwrap());
        assert!(matches!(
            graceful.next_event().await.unwrap(),
            TestRunnerEvent::Cancelled { forced: false, .. }
        ));

        executable(
            &request.executable,
            "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
        );
        let request = TestSelftestRequest::new(
            request.executable,
            request.family,
            request.selector,
            request.parallelism,
            request.verbose,
            request.skip_network,
        )
        .unwrap();
        let command = adapter.command(&request).unwrap();
        let mut forced = TestRunnerJob::new().with_cancellation_timeout(Duration::from_millis(20));
        forced.start(command.clone()).await.unwrap();
        assert_eq!(forced.next_event().await.unwrap(), TestRunnerEvent::Started);
        assert!(matches!(
            forced.next_event().await.unwrap(),
            TestRunnerEvent::Output { .. }
        ));
        assert!(forced.cancel().await.unwrap());
        assert!(matches!(
            forced.next_event().await.unwrap(),
            TestRunnerEvent::Cancelled { forced: true, .. }
        ));

        let mut timed_out = TestRunnerJob::new()
            .with_cancellation_timeout(Duration::from_millis(20))
            .with_operation_timeout(Duration::from_millis(20));
        timed_out.start(command).await.unwrap();
        assert_eq!(
            timed_out.next_event().await.unwrap(),
            TestRunnerEvent::Started
        );
        assert!(matches!(
            timed_out.next_event().await.unwrap(),
            TestRunnerEvent::Output { .. }
        ));
        assert!(matches!(
            timed_out.next_event().await.unwrap(),
            TestRunnerEvent::TimedOut { forced: true, .. }
        ));
    }
}
