include!("security_mapper/process_and_identity.rs");

include!("security_mapper/command_spec.rs");

include!("security_mapper/job_runner.rs");

#[cfg(test)]
mod tests {
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
    #[tokio::test]
    async fn security_mapper_reconstructs_exact_argv_and_streams_bounded_output() {
        let directory = TestDirectory::new("output");
        let preview = preview(&directory);
        let SecurityOperation::PackageMap { executable, .. } = &preview.operation else {
            unreachable!();
        };
        write_executable(
            executable,
            &format!(
                "#!/bin/sh\nprintf 'arg=%s\\n' \"$1\"\nprintf 'stderr\\n' >&2\nprintf '{}\\n'\nprintf '\\377\\n'\n",
                "x".repeat(MAX_SECURITY_MAPPER_LINE_BYTES + 8)
            ),
        );
        let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
        assert_eq!(command.arguments().len(), 1);
        let mut runner = SecurityMapperJobRunner::new();
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::Started {
                id: SecuritySessionId(7)
            }
        );
        let mut saw_argument = false;
        let mut saw_stderr = false;
        let mut saw_truncated = false;
        let mut saw_invalid_utf8 = false;
        loop {
            match runner.next_event().await.unwrap() {
                SecurityMapperRunnerEvent::Output {
                    id,
                    stream,
                    line,
                    truncated,
                } => {
                    assert_eq!(id, SecuritySessionId(7));
                    saw_argument |= line == format!("arg={}", preview.report_roots[0].display());
                    saw_stderr |= stream == SecurityOutputStream::Stderr;
                    saw_truncated |= truncated;
                    saw_invalid_utf8 |= line.contains('\u{fffd}');
                }
                SecurityMapperRunnerEvent::Completed { id, exit_code } => {
                    assert_eq!(id, SecuritySessionId(7));
                    assert_eq!(exit_code, Some(0));
                    break;
                }
                event => panic!("unexpected event: {event:?}"),
            }
        }
        assert!(saw_argument && saw_stderr && saw_truncated && saw_invalid_utf8);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn security_mapper_spawn_retries_only_transient_text_file_busy() {
        let directory = TestDirectory::new("spawn-retry");
        let executable = directory.path().join("cve-check-map-pkgs");
        write_executable(&executable, "#!/bin/sh\nexit 0\n");

        let writer = fs::OpenOptions::new()
            .write(true)
            .open(&executable)
            .unwrap();
        let release = tokio::spawn(async move {
            tokio::time::sleep(
                SECURITY_MAPPER_SPAWN_RETRY_DELAY + SECURITY_MAPPER_SPAWN_RETRY_DELAY,
            )
            .await;
            drop(writer);
        });
        let mut process = Command::new(&executable);
        let mut child = spawn_security_mapper_process(&mut process).await.unwrap();
        assert!(child.wait().await.unwrap().success());
        release.await.unwrap();

        let writer = fs::OpenOptions::new()
            .write(true)
            .open(&executable)
            .unwrap();
        let mut process = Command::new(&executable);
        let error = match spawn_security_mapper_process(&mut process).await {
            Ok(_) => panic!("write-held Security mapper executable unexpectedly spawned"),
            Err(error) => error,
        };
        assert!(is_transient_security_mapper_spawn_error(&error));
        drop(writer);

        assert!(!is_transient_security_mapper_spawn_error(
            &io::Error::from_raw_os_error(libc::EACCES)
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn security_mapper_rejects_tampering_symlinks_and_stale_identity() {
        let directory = TestDirectory::new("validation");
        let mut tampered = preview(&directory);
        tampered.indexed_arguments.push("2: injected".into());
        assert_eq!(
            SecurityMapperCommandSpec::from_preview(&tampered),
            Err(SecurityMapperAdapterError::PreviewMismatch)
        );

        let safe = preview(&directory);
        let SecurityOperation::PackageMap {
            executable: tool, ..
        } = &safe.operation
        else {
            unreachable!();
        };
        let linked = directory.path().join("linked");
        symlink(tool, &linked).unwrap();
        let mut linked_preview = safe.clone();
        let SecurityOperation::PackageMap { executable, .. } = &mut linked_preview.operation else {
            unreachable!();
        };
        *executable = linked.clone();
        linked_preview.indexed_arguments[0] = format!("0: {}", linked.display());
        assert!(matches!(
            SecurityMapperCommandSpec::from_preview(&linked_preview),
            Err(SecurityMapperAdapterError::UnsafeExecutable(_))
        ));

        let linked_reports = directory.path().join("linked-reports");
        symlink(&safe.report_roots[0], &linked_reports).unwrap();
        let mut linked_input = safe.clone();
        linked_input.report_roots = vec![linked_reports.clone()];
        let SecurityOperation::PackageMap { arguments, .. } = &mut linked_input.operation else {
            unreachable!();
        };
        *arguments = vec![linked_reports.display().to_string()];
        let SecurityOperation::PackageMap {
            executable,
            arguments,
        } = &linked_input.operation
        else {
            unreachable!();
        };
        linked_input.indexed_arguments = indexed_arguments(executable, arguments);
        assert!(matches!(
            SecurityMapperCommandSpec::from_preview(&linked_input),
            Err(SecurityMapperAdapterError::UnsafeInput(_))
        ));

        let command = SecurityMapperCommandSpec::from_preview(&safe).unwrap();
        write_executable(tool, "#!/bin/sh\nprintf 'changed\\n'\n");
        assert!(matches!(
            SecurityMapperJobRunner::new().start(command).await,
            Err(SecurityMapperAdapterError::StaleIdentity(_))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn security_mapper_reports_duplicate_nonzero_and_worker_loss() {
        let directory = TestDirectory::new("outcomes");
        let preview = preview(&directory);
        let SecurityOperation::PackageMap { executable, .. } = &preview.operation else {
            unreachable!();
        };
        write_executable(executable, "#!/bin/sh\nexit 9\n");
        let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
        let mut runner = SecurityMapperJobRunner::new();
        runner.start(command.clone()).await.unwrap();
        assert_eq!(
            runner.start(command.clone()).await,
            Err(SecurityMapperAdapterError::Busy)
        );
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::Started {
                id: SecuritySessionId(7)
            }
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::Failed {
                id: SecuritySessionId(7),
                exit_code: Some(9)
            }
        ));

        write_executable(executable, "#!/bin/sh\nsleep 2\n");
        let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
        runner.start(command).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        runner.lose_output_channel();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::Lost {
                id: SecuritySessionId(7),
                ..
            }
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn security_mapper_cancels_gracefully_forcibly_and_times_out() {
        let directory = TestDirectory::new("control");
        let preview = preview(&directory);
        let SecurityOperation::PackageMap { executable, .. } = &preview.operation else {
            unreachable!();
        };
        write_executable(
            executable,
            "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n",
        );
        let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
        let mut runner =
            SecurityMapperJobRunner::new().with_cancellation_timeout(Duration::from_secs(1));
        runner.start(command.clone()).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        assert!(!runner.cancel(SecuritySessionId(8)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::CancellationRejected {
                id: SecuritySessionId(8),
                ..
            }
        ));
        assert!(runner.cancel(SecuritySessionId(7)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::CancellationRequested {
                id: SecuritySessionId(7)
            }
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::Cancelled {
                id: SecuritySessionId(7),
                forced: false,
                ..
            }
        ));

        write_executable(
            executable,
            "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
        );
        let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
        let mut forced =
            SecurityMapperJobRunner::new().with_cancellation_timeout(Duration::from_millis(20));
        forced.start(command.clone()).await.unwrap();
        let _ = forced.next_event().await.unwrap();
        let _ = forced.next_event().await.unwrap();
        assert!(forced.cancel(SecuritySessionId(7)).await.unwrap());
        let _ = forced.next_event().await.unwrap();
        assert!(matches!(
            forced.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::Cancelled { forced: true, .. }
        ));

        let mut timed_out = SecurityMapperJobRunner::new()
            .with_cancellation_timeout(Duration::from_millis(20))
            .with_operation_timeout(Duration::from_millis(20));
        timed_out.start(command).await.unwrap();
        let _ = timed_out.next_event().await.unwrap();
        let _ = timed_out.next_event().await.unwrap();
        assert!(matches!(
            timed_out.next_event().await.unwrap(),
            SecurityMapperRunnerEvent::TimedOut {
                id: SecuritySessionId(7),
                forced: true,
                ..
            }
        ));
    }
}
