include!("sdk_tool/adapter_and_capability.rs");

include!("sdk_tool/command_spec.rs");

include!("sdk_tool/environment_and_validation.rs");

include!("sdk_tool/job_runner.rs");

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
                "yoctui-sdk-tool-{name}-{}-{}",
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

    fn fixture(name: &str) -> (TestDirectory, SdkToolAdapter) {
        let directory = TestDirectory::new(name);
        let workspace = directory.path().join("workspace");
        let scripts = workspace.join("scripts");
        let build = directory.path().join("build");
        let deploy = directory.path().join("deploy/sdk");
        fs::create_dir_all(&scripts).unwrap();
        fs::create_dir_all(&build).unwrap();
        fs::create_dir_all(&deploy).unwrap();
        for tool in SDK_TOOL_NAMES {
            executable(&scripts.join(tool), "#!/bin/sh\nprintf '%s\\n' \"$@\"\n");
        }
        let adapter = SdkToolAdapter::new(build, deploy, vec![workspace]);
        (directory, adapter)
    }

    fn artifact(path: &Path) -> SdkArtifactIdentity {
        let metadata = fs::metadata(path).unwrap();
        SdkArtifactIdentity {
            path: path.into(),
            size_bytes: metadata.len(),
            modified_unix_seconds: modified_seconds(&metadata).unwrap(),
        }
    }

    fn publish_preview(adapter: &SdkToolAdapter, directory: &TestDirectory) -> SdkPublishPreview {
        let installer = adapter.sdk_deploy_root.join("poky-toolchain.sh");
        fs::write(&installer, b"installer").unwrap();
        let destination = directory.path().join("published");
        fs::create_dir(&destination).unwrap();
        let executable = match adapter.capability() {
            SdkToolCapability::Available {
                publish: Some(path),
                ..
            } => path,
            capability => panic!("unexpected capability: {capability:?}"),
        };
        SdkPublishPreview::new(executable, artifact(&installer), destination).unwrap()
    }

    fn native_preview(
        adapter: &SdkToolAdapter,
        mode: SdkNativeMode,
        extracted_root: Option<PathBuf>,
    ) -> SdkNativePreview {
        let capability = adapter.capability();
        let executable = capability.executable_for(mode).unwrap();
        SdkNativePreview::new(SdkNativeRequest {
            executable,
            mode,
            extracted_root,
            recipe: "cmake-native".into(),
            tool: (mode == SdkNativeMode::RunNative).then(|| "cmake".into()),
            arguments: if mode == SdkNativeMode::RunNative {
                vec!["--version".into()]
            } else {
                Vec::new()
            },
        })
        .unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn sdk_tool_capability_is_partial_and_rejects_unsafe_candidates() {
        let directory = TestDirectory::new("capability");
        let workspace = directory.path().join("workspace");
        let scripts = workspace.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        executable(&scripts.join("oe-publish-sdk"), "#!/bin/sh\nexit 0\n");
        let inspector = SdkToolCapabilityInspector::new(vec![workspace.clone()]);
        assert!(matches!(
            inspector.inspect(),
            SdkToolCapability::Available {
                publish: Some(_),
                find_sysroot: None,
                run_native: None,
            }
        ));

        let outside = directory.path().join("outside");
        executable(&outside, "#!/bin/sh\nexit 0\n");
        symlink(&outside, scripts.join("oe-run-native")).unwrap();
        assert!(matches!(
            inspector.inspect(),
            SdkToolCapability::Failed { .. }
        ));
        assert!(matches!(
            SdkToolCapabilityInspector::new(vec![directory.path().join("missing")]).inspect(),
            SdkToolCapability::Failed { .. }
        ));
    }

    #[test]
    fn sdk_tool_commands_reconstruct_exact_publication_and_native_argv() {
        let (directory, adapter) = fixture("commands");
        let publish = publish_preview(&adapter, &directory);
        let publish_command = adapter.publication_command(&publish).unwrap();
        assert_eq!(
            publish_command.arguments(),
            [
                publish.request.artifact.path.as_os_str(),
                publish.request.destination.as_os_str(),
            ]
        );

        let native = native_preview(&adapter, SdkNativeMode::RunNative, None);
        let native_command = adapter.native_command(&native).unwrap();
        assert_eq!(
            native_command.arguments(),
            ["cmake-native", "cmake", "--version"]
        );
        assert_eq!(native_command.current_directory(), adapter.build_directory);
        assert!(!native_command.clears_environment());

        let mut tampered = native;
        tampered.argv.push("injected".into());
        assert_eq!(
            adapter.native_command(&tampered),
            Err(SdkToolAdapterError::PreviewMismatch)
        );
    }

    #[cfg(unix)]
    #[test]
    fn sdk_tool_commands_reject_unsafe_paths_and_stale_installer() {
        let (directory, adapter) = fixture("unsafe");
        let mut preview = publish_preview(&adapter, &directory);
        preview.argv.push("injected".into());
        assert_eq!(
            adapter.publication_command(&preview),
            Err(SdkToolAdapterError::PreviewMismatch)
        );
        preview.argv.pop();
        fs::write(&preview.request.artifact.path, b"changed installer").unwrap();
        assert!(matches!(
            adapter.publication_command(&preview),
            Err(SdkToolAdapterError::UnsafeInstaller(_))
        ));

        let other = directory.path().join("other-tool");
        executable(&other, "#!/bin/sh\nexit 0\n");
        let mut native = native_preview(&adapter, SdkNativeMode::FindSysroot, None);
        native.request.executable = other.clone();
        native.argv[0] = other;
        assert!(matches!(
            adapter.native_command(&native),
            Err(SdkToolAdapterError::UnsafeTool(_))
        ));

        let destination = directory.path().join("nonempty");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("existing"), b"data").unwrap();
        let installer = adapter.sdk_deploy_root.join("fresh.sh");
        fs::write(&installer, b"installer").unwrap();
        let executable = adapter.capability().publish_executable().unwrap();
        let nonempty =
            SdkPublishPreview::new(executable, artifact(&installer), destination).unwrap();
        assert!(matches!(
            adapter.publication_command(&nonempty),
            Err(SdkToolAdapterError::UnsafeDestination(_))
        ));

        let extracted = directory.path().join("real-extracted");
        fs::create_dir(&extracted).unwrap();
        fs::write(
            extracted.join("environment-setup-core2"),
            "export SDK_ROOT='/opt/sdk'\n",
        )
        .unwrap();
        let linked = directory.path().join("linked-extracted");
        symlink(&extracted, &linked).unwrap();
        let linked_preview = native_preview(&adapter, SdkNativeMode::FindSysroot, Some(linked));
        assert!(matches!(
            adapter.native_command(&linked_preview),
            Err(SdkToolAdapterError::UnsafeExtractedRoot(_))
        ));
    }

    #[test]
    fn sdk_tool_extracted_environment_is_validated_and_child_only() {
        let (directory, adapter) = fixture("environment");
        let extracted = directory.path().join("extracted");
        fs::create_dir(&extracted).unwrap();
        fs::write(
            extracted.join("environment-setup-core2-64-poky-linux"),
            "export SDK_ROOT='/opt/sdk'\nexport SDK_BIN=\"$SDK_ROOT/bin\"\n",
        )
        .unwrap();
        let preview = native_preview(&adapter, SdkNativeMode::RunNative, Some(extracted.clone()));
        let command = adapter.native_command(&preview).unwrap();
        assert!(command.clears_environment());
        assert_eq!(
            command.environment().get(OsStr::new("SDK_BIN")),
            Some(&OsString::from("/opt/sdk/bin"))
        );
        assert!(!command.environment().contains_key(OsStr::new("HOME")));

        fs::write(
            extracted.join("environment-setup-second"),
            "export SECOND='value'\n",
        )
        .unwrap();
        assert!(matches!(
            adapter.native_command(&preview),
            Err(SdkToolAdapterError::InvalidEnvironment(_))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_tool_runner_confines_extracted_environment_to_the_child() {
        let (directory, adapter) = fixture("child-environment");
        let extracted = directory.path().join("extracted");
        fs::create_dir(&extracted).unwrap();
        fs::write(
            extracted.join("environment-setup-core2-64-poky-linux"),
            "export SDK_ROOT='/opt/sdk'\nexport SDK_BIN=\"$SDK_ROOT/bin\"\n",
        )
        .unwrap();
        let preview = native_preview(&adapter, SdkNativeMode::RunNative, Some(extracted.clone()));
        executable(
            &preview.request.executable,
            "#!/bin/sh\nprintf 'SDK_BIN=%s HOME=%s\\n' \"$SDK_BIN\" \"${HOME-unset}\"\n",
        );
        let preview = SdkNativePreview::new(preview.request).unwrap();
        let command = adapter.native_command(&preview).unwrap();
        let mut runner = SdkToolJobRunner::new();
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        let output = match runner.next_event().await.unwrap() {
            SdkToolRunnerEvent::Output { line, .. } => line,
            event => panic!("unexpected runner event: {event:?}"),
        };
        assert_eq!(output, "SDK_BIN=/opt/sdk/bin HOME=unset");
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Completed { exit_code: Some(0) }
        ));
        assert!(std::env::var_os("SDK_BIN").is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_tool_spawn_retries_only_transient_text_file_busy() {
        let directory = TestDirectory::new("spawn-retry");
        let program = directory.path().join("sdk-tool");
        executable(&program, "#!/bin/sh\nexit 0\n");
        let writer = fs::OpenOptions::new().write(true).open(&program).unwrap();
        let release = tokio::spawn(async move {
            tokio::time::sleep(SDK_TOOL_SPAWN_RETRY_DELAY + SDK_TOOL_SPAWN_RETRY_DELAY).await;
            drop(writer);
        });
        let mut process = Command::new(&program);
        let mut child = spawn_sdk_tool_process(&mut process).await.unwrap();
        assert!(child.wait().await.unwrap().success());
        release.await.unwrap();

        let writer = fs::OpenOptions::new().write(true).open(&program).unwrap();
        let mut process = Command::new(&program);
        let error = match spawn_sdk_tool_process(&mut process).await {
            Ok(_) => panic!("write-held SDK tool unexpectedly spawned"),
            Err(error) => error,
        };
        assert!(is_transient_sdk_tool_spawn_error(&error));
        drop(writer);

        assert!(!is_transient_sdk_tool_spawn_error(
            &io::Error::from_raw_os_error(libc::EACCES)
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_tool_runner_streams_bounded_output_and_terminal_status() {
        let (directory, adapter) = fixture("runner-output");
        let publish = publish_preview(&adapter, &directory);
        let tool = publish.request.executable.clone();
        executable(
            &tool,
            &format!(
                "#!/bin/sh\nprintf 'stdout\\n'\nprintf 'stderr\\n' >&2\nprintf '{}\\n'\nexit 0\n",
                "x".repeat(MAX_SDK_TOOL_LINE_BYTES + 8)
            ),
        );
        let publish =
            SdkPublishPreview::new(tool, publish.request.artifact, publish.request.destination)
                .unwrap();
        let command = adapter.publication_command(&publish).unwrap();
        let mut runner = SdkToolJobRunner::new();
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        let mut saw_stdout = false;
        let mut saw_stderr = false;
        let mut saw_truncated = false;
        loop {
            match runner.next_event().await.unwrap() {
                SdkToolRunnerEvent::Output {
                    stream, truncated, ..
                } => {
                    saw_stdout |= stream == SdkOutputStream::Stdout;
                    saw_stderr |= stream == SdkOutputStream::Stderr;
                    saw_truncated |= truncated;
                }
                SdkToolRunnerEvent::Completed { exit_code } => {
                    assert_eq!(exit_code, Some(0));
                    break;
                }
                event => panic!("unexpected runner event: {event:?}"),
            }
        }
        assert!(saw_stdout && saw_stderr && saw_truncated);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_tool_runner_rejects_duplicate_and_reports_nonzero_and_loss() {
        let (directory, adapter) = fixture("runner-outcomes");
        let preview = publish_preview(&adapter, &directory);
        executable(&preview.request.executable, "#!/bin/sh\nexit 7\n");
        let preview = SdkPublishPreview::new(
            preview.request.executable,
            preview.request.artifact,
            preview.request.destination,
        )
        .unwrap();
        let command = adapter.publication_command(&preview).unwrap();
        let mut runner = SdkToolJobRunner::new();
        runner.start(command.clone()).await.unwrap();
        assert_eq!(
            runner.start(command.clone()).await,
            Err(SdkToolAdapterError::Busy)
        );
        assert_eq!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        assert_eq!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Failed { exit_code: Some(7) }
        );

        executable(&preview.request.executable, "#!/bin/sh\nsleep 2\n");
        let command = adapter.publication_command(&preview).unwrap();
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        runner.lose_output_channel();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Lost { .. }
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_tool_runner_times_out_and_cancels_gracefully_or_forcibly() {
        let (directory, adapter) = fixture("runner-control");
        let preview = publish_preview(&adapter, &directory);
        executable(
            &preview.request.executable,
            "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n",
        );
        let preview = SdkPublishPreview::new(
            preview.request.executable,
            preview.request.artifact,
            preview.request.destination,
        )
        .unwrap();
        let command = adapter.publication_command(&preview).unwrap();
        let mut runner = SdkToolJobRunner::new().with_cancellation_timeout(Duration::from_secs(1));
        runner.start(command.clone()).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Output { line, .. } if line == "ready"
        ));
        assert!(runner.cancel().await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Cancelled { forced: false, .. }
        ));
        assert!(!runner.cancel().await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::CancellationRejected { .. }
        ));

        executable(
            &preview.request.executable,
            "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
        );
        let command = adapter.publication_command(&preview).unwrap();
        let mut forced_cancel =
            SdkToolJobRunner::new().with_cancellation_timeout(Duration::from_millis(20));
        forced_cancel.start(command.clone()).await.unwrap();
        assert_eq!(
            forced_cancel.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        assert!(matches!(
            forced_cancel.next_event().await.unwrap(),
            SdkToolRunnerEvent::Output { .. }
        ));
        assert!(forced_cancel.cancel().await.unwrap());
        assert!(matches!(
            forced_cancel.next_event().await.unwrap(),
            SdkToolRunnerEvent::Cancelled { forced: true, .. }
        ));

        let mut timed_out = SdkToolJobRunner::new()
            .with_cancellation_timeout(Duration::from_millis(20))
            .with_operation_timeout(Duration::from_millis(20));
        timed_out.start(command).await.unwrap();
        assert_eq!(
            timed_out.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        assert!(matches!(
            timed_out.next_event().await.unwrap(),
            SdkToolRunnerEvent::Output { .. }
        ));
        assert!(matches!(
            timed_out.next_event().await.unwrap(),
            SdkToolRunnerEvent::TimedOut { forced: true, .. }
        ));
    }
}
